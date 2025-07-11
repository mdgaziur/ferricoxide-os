/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2025  MD Gaziur Rahman Noor
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
use crate::arch::x86_64::mm::PhysAddr;
use crate::arch::x86_64::{mm, KERNEL_CONTENT_INFO};
use crate::ds::{static_bitmap_size, StaticBitmap};
use crate::kutils::{ADDRESS_SPACE_SIZE, KB, MB};
use crate::{serial_println, verify_called_once, BOOT_INFO};
use multiboot2::MemoryAreaType;
use spin::Mutex;

pub const FRAME_SIZE: usize = 4 * KB;
pub const FRAME_COUNT: usize = ADDRESS_SPACE_SIZE / FRAME_SIZE;

pub static FRAME_ALLOCATOR: Mutex<BitmapFrameAllocator> = Mutex::new(BitmapFrameAllocator::new());

pub struct BitmapFrameAllocator {
    bit_map: StaticBitmap<{ static_bitmap_size(FRAME_COUNT) }>,
    total_memory: usize,
    available_memory: usize,
}

impl BitmapFrameAllocator {
    pub const fn new() -> Self {
        BitmapFrameAllocator {
            bit_map: StaticBitmap::new(),
            total_memory: 0,
            available_memory: 0,
        }
    }

    pub fn init(&mut self) {
        verify_called_once!();

        let boot_info = BOOT_INFO.get().unwrap();
        let mut total_memory = 0;
        let mut unavailable_memory = 0;

        // reserve the first 1MiB area to avoid weird corruptions
        self.reserve_area(0x00000000, MB);

        self.reserve_area(boot_info.start_address(), boot_info.end_address());

        let framebuffer_tag = boot_info.framebuffer_tag().unwrap().unwrap();
        self.reserve_area(
            framebuffer_tag.address() as usize,
            framebuffer_tag.address() as usize
                + (framebuffer_tag.height() * framebuffer_tag.pitch()) as usize,
        );

        for memory_map in boot_info.memory_map_tag().unwrap().memory_areas() {
            if memory_map.typ() != MemoryAreaType::Available {
                serial_println!("Reserving unavailable area: {:?}", memory_map);
                self.reserve_area(
                    memory_map.start_address() as usize,
                    memory_map.end_address() as usize,
                );
            } else {
                total_memory += memory_map.size() as usize;
            }
        }

        for elf_section in boot_info.elf_sections_tag().unwrap().sections() {
            serial_println!("Reserving elf section: {:?}", elf_section);
            unavailable_memory += self.reserve_area(
                elf_section.start_address() as usize,
                elf_section.end_address() as usize,
            );
        }

        let kernel_content_info = KERNEL_CONTENT_INFO.get().unwrap();
        serial_println!("Reserving kernel content: {:?}", kernel_content_info);
        unavailable_memory += self.reserve_area(
            kernel_content_info.phys_start_addr as usize,
            kernel_content_info.phys_end_addr as usize,
        );

        self.total_memory = total_memory;
        self.available_memory = self.total_memory - unavailable_memory;
    }

    fn reserve_area(&mut self, start: usize, end: usize) -> usize {
        assert!(
            start <= end,
            "start address must be less than or equal to end address"
        );

        let start = mm::align_down(start, FRAME_SIZE);
        let mut end = mm::align_up(end, FRAME_SIZE);

        if start >= ADDRESS_SPACE_SIZE {
            return 0;
        }
        if end >= ADDRESS_SPACE_SIZE {
            end = ADDRESS_SPACE_SIZE - 1;
        }

        let mut cur_addr = start;
        while cur_addr < end {
            self.bit_map.set(cur_addr / FRAME_SIZE);
            cur_addr += FRAME_SIZE;
        }

        (end - start) + 1
    }

    pub fn total_memory(&self) -> usize {
        self.total_memory
    }

    pub fn available_memory(&self) -> usize {
        self.available_memory
    }
}

impl FrameAllocator for BitmapFrameAllocator {
    /// Finds a free frame and returns a `Frame` containing the frame index
    fn allocate(&mut self) -> Option<Frame> {
        if self.available_memory < FRAME_SIZE {
            return None;
        }
        let mut res_frame = None;

        for (idx, bit) in self.bit_map.iter().enumerate() {
            if !bit {
                res_frame = Some(Frame { number: idx });
                break;
            }
        }

        if let Some(frame) = res_frame {
            self.bit_map.set(frame.number);
        }

        self.available_memory -= FRAME_SIZE;
        res_frame
    }

    /// Marks given frame as free to be reused by a subsequent allocation.
    ///
    /// # SAFETY
    /// *Must* ensure that the given frame is no longer in use. Also, it should be made sure
    /// that the frame does not point to a reserved memory.
    unsafe fn deallocate(&mut self, frame: Frame) {
        if !self.bit_map.get(frame.number) {
            panic!("attempt to free an unused frame: {:?}", frame);
        }

        self.bit_map.clear(frame.number);
        self.available_memory += FRAME_SIZE;
    }
}

pub trait FrameAllocator {
    /// Finds a free frame and returns a `Frame` containing the frame index
    fn allocate(&mut self) -> Option<Frame>;

    /// Marks given frame as free to be reused by a subsequent allocation.
    ///
    /// # SAFETY
    /// *Must* ensure that the given frame is no longer in use.
    unsafe fn deallocate(&mut self, frame: Frame);
}

#[derive(Debug, Copy, Clone)]
pub struct Frame {
    pub(crate) number: usize,
}

impl Frame {
    pub fn containing_address(addr: PhysAddr) -> Self {
        Self {
            number: addr / FRAME_SIZE,
        }
    }

    pub fn start_address(&self) -> PhysAddr {
        self.number * FRAME_SIZE
    }
}
