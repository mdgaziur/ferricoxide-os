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

use crate::kutils::{KB, MB};
use crate::{_binary_kernel_bin_end, _binary_kernel_bin_start, dbg, serial_print, serial_println};
use core::arch::asm;
use core::mem::size_of;
use core::ptr::addr_of;
use core::slice;
use elf_rs::*;
use multiboot2::{BootInformation, BootInformationHeader};
use spin::Once;

pub(super) static BOOT_INFO: Once<BootInformation> = Once::new();

#[link_section = ".pml4"]
static mut PML4: [u64; 512] = [0; 512];

#[link_section = ".pdpt"]
static mut PDPT: [u64; 512] = [0; 512];

#[link_section = ".pdt"]
static mut PDT: [u64; 512] = [0; 512];

#[link_section = ".higher_half_pdpt"]
static mut HIGHER_HALF_PDPT: [u64; 512] = [0; 512];

#[link_section = ".higher_half_pdt"]
static mut HIGHER_HALF_PDT: [u64; 512] = [0; 512];

static GDT: [u64; 2] = [0, (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)];

/// Assumes that the kernel never exceeds 8MB in size
/// NOTE: Bump the size in case kernel exceeds 8MB in size
#[link_section = ".kernel_content"]
static mut KERNEL_CONTENT: [u8; 8 * MB] = [0; 8 * MB];

const PAGE_SIZE: usize = 4 * KB;

unsafe fn map_kernel_to_higher_half(kernel_start_addr: u32, kernel_elf: &Elf) {
    let boot_info = BOOT_INFO.get().unwrap();
    let basic_memory_info = *boot_info.basic_memory_info_tag().unwrap();

    serial_println!(
        "Memory size: {}MB",
        (basic_memory_info.memory_upper() - basic_memory_info.memory_lower()) as f64 / 1024.0
    );

    // 1. Identity map the first 2GB of the address space
    unsafe {
        PML4[0] = (addr_of!(PDPT) as u32 | 0b11) as u64;
        PDPT[0] = (addr_of!(PDT) as u32 | 0b11) as u64;

        for entry_idx in 0..PDT.len() {
            let entry = (0x200000 * entry_idx) | 0b10000011;
            PDT[entry_idx] = entry as u64;
        }
    }

    // 2. Map the kernel using 2MB pages to keep the paging structure simple.
    unsafe fn map_kernel_from_phdr(offset: usize, phdr: &ProgramHeaderEntry) {
        let Some(phdr_content) = phdr.content() else {
            serial_println!("Skipping program header because it has no content: {:?}", phdr);
            return;
        };

        for idx in offset..phdr.filesz() as usize - 1 {
            KERNEL_CONTENT[idx] = phdr_content[idx - offset];
        }
    }

    let mut phdr_iter = kernel_elf.program_header_iter();
    let first_phdr = phdr_iter.nth(0).unwrap();
    let first_phdr_offset = first_phdr.offset() as usize;
    if first_phdr.ph_type() == ProgramType::LOAD {
        map_kernel_from_phdr(0, &first_phdr);
    } else if first_phdr.ph_type() == ProgramType::DYNAMIC {
        panic!("Kernel is somehow a dynamically linked executable!");
    }

    // Assuming that the section addrs never require a new table to be created(that'd require
    // the section to be obscenely big!)
    let higher_half_pml4_index = (first_phdr.vaddr() >> 39 & 0b111111111) as usize;
    let higher_half_pdpt_index = (first_phdr.vaddr() >> 30 & 0b111111111) as usize;
    let higher_half_pdt_index = (first_phdr.vaddr() >> 21 & 0b111111111) as usize;

    PML4[higher_half_pml4_index] = (addr_of!(HIGHER_HALF_PDPT) as u32 | 0b11) as u64;
    HIGHER_HALF_PDPT[higher_half_pdpt_index] =
        (addr_of!(HIGHER_HALF_PDT) as u32 | 0b11) as u64;


    let kernel_content_start = addr_of!(KERNEL_CONTENT) as usize;
    let kernel_content_end = kernel_content_start + KERNEL_CONTENT.len() - 1;
    let mut cur_addr = kernel_content_start;
    let mut entry_idx = 0;

    while cur_addr <= kernel_content_end {
        let entry = cur_addr | 0b10000011;
        HIGHER_HALF_PDT[higher_half_pdt_index + entry_idx as usize] = entry as u64;

        cur_addr += 0x200000;
        entry_idx += 1;
    }

    for phdr in kernel_elf.program_header_iter() {
        if phdr.ph_type() == ProgramType::LOAD {
            map_kernel_from_phdr(phdr.offset() as usize - first_phdr_offset, &phdr);
        } else if phdr.ph_type() == ProgramType::DYNAMIC {
            panic!("Kernel is somehow a dynamically linked executable!");
        }
    }
}

unsafe fn enable_paging() {
    let pml4_addr = (addr_of!(PML4) as *const u64) as u32;

    asm!(
    "// load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, {}
    mov cr3, eax

    // enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    // set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    // enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    ", in(reg) pml4_addr)
}

unsafe fn load_gdt() {
    #[repr(C, packed(2))]
    struct DescriptorTablePointer {
        limit: u16,
        base: u64,
    }

    let pointer = DescriptorTablePointer {
        limit: (GDT.len() * size_of::<u64>() - 1) as u16,
        base: GDT.as_ptr() as u64,
    };

    asm!("lgdt [{}]", in(reg) addr_of!(pointer));
}

unsafe fn call_kernel(mb_ptr: u32, kernel_addr: u32) {
    asm!(
    "
    mov eax, {}
    push 0x8
    push {}
    retf", in(reg) mb_ptr, in(reg) kernel_addr)
}

#[no_mangle]
pub extern "cdecl" fn prekernel_main(mb_ptr: *const BootInformationHeader) -> ! {
    let mb_info = unsafe { BootInformation::load(mb_ptr).unwrap() };
    BOOT_INFO.call_once(|| mb_info);

    unsafe {
        let kernel_start_addr = &_binary_kernel_bin_start as *const u16;
        let kernel_end_addr = &_binary_kernel_bin_end as *const u16;
        let kernel_size = kernel_end_addr as usize - kernel_start_addr as usize;

        serial_println!("Kernel ELF start: {:p}", kernel_start_addr);
        serial_println!("Kernel ELF size: {}KB", kernel_size as f32 / KB as f32);

        let kernel = slice::from_raw_parts(
            kernel_start_addr as *const u8,
            kernel_end_addr as usize - kernel_start_addr as usize,
        );
        let kernel_elf = Elf::from_bytes(kernel).unwrap();

        map_kernel_to_higher_half(kernel_start_addr as u32, &kernel_elf);
        enable_paging();
        load_gdt();

        let kernel_text_section = kernel_elf.lookup_section(b".text").unwrap();
        let kernel_text_section_content = kernel_text_section.content().unwrap();
        for i in 0..4 {
            serial_print!("{:x} ", kernel_text_section_content[i])
        }
        serial_println!();

        call_kernel(
            // offset shouldn't be big enough to cause issues when casting to 32bit unsigned integer
            mb_ptr as u32,
            addr_of!(KERNEL_CONTENT) as u32,
        );
    }

    unreachable!()
}
