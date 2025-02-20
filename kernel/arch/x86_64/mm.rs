/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2024  MD Gaziur Rahman Noor
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::arch::x86_64::{BOOT_INFO, KERNEL_CONTENT_INFO};
use crate::ds::{static_bitmap_size, StaticBitMap};
use crate::kutils::{ADDRESS_SPACE_SIZE, KB};
use crate::verify_called_once;
use multiboot2::MemoryAreaType;
use spin::Mutex;

const FRAME_SIZE: usize = 4 * KB;
const FRAME_COUNT: usize = ADDRESS_SPACE_SIZE / FRAME_SIZE;

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

struct FrameAllocator {
    bit_map: StaticBitMap<{ static_bitmap_size(FRAME_COUNT) }>,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        FrameAllocator {
            bit_map: StaticBitMap::new(),
        }
    }

    pub fn init(&mut self) {
        verify_called_once!();

        let boot_info = BOOT_INFO.get().unwrap();

        for memory_map in boot_info.memory_map_tag().unwrap().memory_areas() {
            if memory_map.typ() != MemoryAreaType::Available {
                self.reserve_area(
                    memory_map.start_address() as usize,
                    memory_map.end_address() as usize,
                );
            }
        }

        for elf_section in boot_info.elf_sections().unwrap() {
            self.reserve_area(
                elf_section.start_address() as usize,
                elf_section.end_address() as usize,
            );
        }

        let kernel_content_info = KERNEL_CONTENT_INFO.get().unwrap();
        self.reserve_area(
            kernel_content_info.phys_start_addr as usize,
            kernel_content_info.phys_end_addr as usize,
        );
    }

    fn reserve_area(&mut self, start: usize, end: usize) {
        let start = align_down(start, FRAME_SIZE);
        let mut end = align_up(end, FRAME_SIZE);

        if start >= ADDRESS_SPACE_SIZE {
            return;
        }
        if end >= ADDRESS_SPACE_SIZE {
            end = ADDRESS_SPACE_SIZE - 1;
        }

        let mut cur_addr = start;
        while cur_addr < end {
            self.bit_map.set(cur_addr / FRAME_SIZE);
            cur_addr += FRAME_SIZE;
        }
    }
}

fn align_up(addr: usize, alignment: usize) -> usize {
    (addr + alignment - 1) & !(alignment - 1)
}

fn align_down(addr: usize, alignment: usize) -> usize {
    addr & !(alignment - 1)
}

pub fn mm_init() {
    FRAME_ALLOCATOR.lock().init();
}
