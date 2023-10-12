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
use core::mem::align_of;
use multiboot2::{BootInformation, MemoryAreaType};

use crate::serial_println;
use crate::units::MiB;

pub mod frame_alloc;
pub mod page_table;

pub fn init(boot_info: &BootInformation) {
    let memory_map_tag = boot_info.memory_map_tag().unwrap();
    let elf_sections_tag = boot_info.elf_sections_tag().unwrap();

    let mut available_memory = 0;
    let mut allocation_region = 0;
    let mut allocation_region_size = 0;
    serial_println!("\n|<=============== Memory Areas ===============>|");
    for (idx, memory_area) in memory_map_tag.memory_areas().enumerate() {
        if memory_area.typ() == MemoryAreaType::Available {
            available_memory += memory_area.size();

            if allocation_region_size < memory_area.size() {
                allocation_region = memory_area.start_address();
                allocation_region_size = memory_area.size();
            }
        }

        serial_println!("Memory area: {}", idx + 1);
        serial_println!("==> Start address: {:#x}", memory_area.start_address());
        serial_println!("==> End address: {:#x}", memory_area.end_address());
        serial_println!("==> Size: {} bytes", memory_area.size());
        serial_println!("==> Type: {:?}\n", memory_area.typ());
    }

    // Linker script requires that the kernel starts at 1M
    let kernel_start_addr = 0x100000;
    let mut kernel_end_addr = 0;

    serial_println!("|<=============== Kernel ELF Sections ===============>|");
    for elf_section in elf_sections_tag.sections() {
        if kernel_end_addr < elf_section.end_address() {
            kernel_end_addr = elf_section.end_address();
        }

        serial_println!("Elf section: {:?}", elf_section.name().unwrap());
        serial_println!("==> Start address: {:#x}", elf_section.start_address());
        serial_println!("==> End address: {:#x}", elf_section.end_address());
        serial_println!("==> Size: {} bytes", elf_section.size());
        serial_println!("==> Type: {:?}\n", elf_section.section_type());
    }
    let kernel_size = kernel_end_addr - kernel_start_addr;
    if allocation_region < kernel_end_addr {
        serial_println!(
            "[NOTE]: kernel resides in the allocation region. Adjusting it to kernel end address"
        );
        serial_println!("======> Allocation region start: {:#x}", allocation_region);

        // Point to the next valid(aligned) address for the start of the allocation region.
        // SAFETY: the address is valid.
        unsafe {
            let kernel_end_ptr = kernel_end_addr as *mut u8;
            let mut kernel_end_next_ptr = kernel_end_ptr.add(1);
            kernel_end_next_ptr =
                kernel_end_next_ptr.add(kernel_end_next_ptr.align_offset(align_of::<u64>()));
            allocation_region = kernel_end_next_ptr as u64;
        }
        allocation_region_size -= kernel_size;

        serial_println!(
            "======> Adjusted allocation region start: {:#x}\n",
            allocation_region
        );
    }

    serial_println!("=> Kernel start address: {:#x}", kernel_start_addr);
    serial_println!("=> Kernel end address: {:#x}", kernel_end_addr);
    serial_println!("=> Kernel size: {:.2} MiB", kernel_size as f64 / MiB as f64);
    serial_println!("=> Free memory: {:.2} MiB", allocation_region_size / MiB);

    // TODO: only works on one big allocation region. In some cases, reserved memories can break one
    //       region into multiple parts. In such cases, only the biggest is chosen. This may result
    //       in a huge waste of memory.
    let mut frame_allocator = unsafe { FrameAllocator::new(allocation_region as _) };
}
