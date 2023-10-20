/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2023  MD Gaziur Rahman Noor
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

use crate::arch::mm::frame_alloc::FrameAllocator;

use crate::arch::mm::paging::remap_kernel;
use crate::serial_println;
use crate::units::MiB;
use core::fmt::{Debug, Formatter};
use linked_list_allocator::LockedHeap;
use multiboot2::{BootInformation, MemoryAreaType};

pub mod frame_alloc;
pub mod page_table;
pub mod paging;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty(); // TODO: implement own allocator

pub const KERNEL_HEAP_START: *mut u8 = 0xcafebeef as *mut u8;
pub const KERNEL_HEAP_SIZE: usize = 32 * MiB;

pub fn init(boot_info: &BootInformation) {
    let memory_map_tag = boot_info.memory_map_tag().unwrap();
    let elf_sections = boot_info.elf_sections().unwrap();

    let mut total_available_memory = 0usize;
    serial_println!("\n|<=============== Memory Areas ===============>|");
    for (idx, memory_area) in memory_map_tag.memory_areas().iter().enumerate() {
        if memory_area.typ() == MemoryAreaType::Available {
            total_available_memory += memory_area.size() as usize;
        }

        serial_println!("Memory area: {}", idx + 1);
        serial_println!("==> Start address: {:#x}", memory_area.start_address());
        serial_println!("==> End address: {:#x}", memory_area.end_address());
        serial_println!("==> Size: {} bytes", memory_area.size());
        serial_println!("==> Type: {:?}\n", memory_area.typ());
    }

    // Linker script requires that the kernel starts at 1M
    let kernel_start_addr = 0x100000;
    let mut kernel_end_addr = 0usize;

    serial_println!("|<=============== Kernel ELF Sections ===============>|");
    for elf_section in elf_sections {
        if kernel_end_addr < elf_section.end_address() as usize {
            kernel_end_addr = elf_section.end_address() as usize;
        }

        serial_println!("Elf section: {:?}", elf_section.name().unwrap());
        serial_println!("==> Start address: {:#x}", elf_section.start_address());
        serial_println!("==> End address: {:#x}", elf_section.end_address());
        serial_println!("==> Size: {} bytes", elf_section.size());
        serial_println!("==> Type: {:?}\n", elf_section.section_type());
    }

    let memory_used = (kernel_end_addr - kernel_start_addr)
        + (boot_info.end_address() - boot_info.start_address())
        + FrameAllocator::SIZE;
    let free_memory = total_available_memory - memory_used;

    serial_println!("=> Kernel start address: {:#x}", kernel_start_addr);
    serial_println!("=> Kernel end address: {:#x}", kernel_end_addr);
    serial_println!(
        "=> Kernel size: {:.4} MiB",
        (kernel_end_addr - kernel_start_addr) as f64 / MiB as f64
    );
    serial_println!("=> Used memory: {:.4} MiB", memory_used as f64 / MiB as f64);
    serial_println!(
        "=> Total available memory: {:.4} MiB",
        total_available_memory as f64 / MiB as f64
    );
    serial_println!("=> Free memory: {:.4} MiB", free_memory as f64 / MiB as f64);

    let frame_allocator = FrameAllocator::new(
        memory_map_tag.memory_areas(),
        boot_info.start_address(),
        boot_info.end_address(),
        kernel_start_addr,
        kernel_end_addr,
    );
    let mut kmm = KernelMM {
        frame_allocator,
        total_available_memory,
        free_memory,
    };

    remap_kernel(&mut kmm, boot_info);
    unsafe { ALLOCATOR.lock().init(KERNEL_HEAP_START, KERNEL_HEAP_SIZE) }
}

pub struct KernelMM {
    frame_allocator: FrameAllocator,
    total_available_memory: usize,
    free_memory: usize,
}

#[derive(Copy, Clone)]
pub struct PhysicalAddress(usize);

#[derive(Copy, Clone)]
pub struct VirtualAddress(usize);

impl From<usize> for PhysicalAddress {
    fn from(val: usize) -> Self {
        PhysicalAddress(val)
    }
}

impl Debug for PhysicalAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("PhysicalAddress")
            .field(&format_args!("{:#x}", self.0))
            .finish()
    }
}

impl From<usize> for VirtualAddress {
    fn from(val: usize) -> Self {
        VirtualAddress(val)
    }
}

impl Debug for VirtualAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("VirtualAddress")
            .field(&format_args!("{:#x}", self.0))
            .finish()
    }
}
