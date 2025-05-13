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
use crate::kernel_memsz::KERNEL_CONTENT_TOTAL_MEMSZ;
use crate::kutils::{KB, MB};
use crate::{_binary_kernel_bin_end, _binary_kernel_bin_start, serial_print, serial_println};
use core::arch::asm;
use core::mem::size_of;
use core::ops::AddAssign;
use core::ptr::{addr_of, slice_from_raw_parts, slice_from_raw_parts_mut};
use core::slice;
use elf_rs::*;
use multiboot2::{BootInformation, BootInformationHeader, FramebufferType};
use spin::Once;

pub(super) static BOOT_INFO: Once<BootInformation> = Once::new();

#[unsafe(link_section = ".pml4")]
static mut PML4: [u64; 512] = [0; 512];

#[unsafe(link_section = ".pdpt")]
static mut PDPT: [u64; 512] = [0; 512];

#[unsafe(link_section = ".pdt")]
static mut PDT: [u64; 512] = [0; 512];

#[unsafe(link_section = ".higher_half_pdpt")]
static mut HIGHER_HALF_PDPT: [u64; 512] = [0; 512];

#[unsafe(link_section = ".higher_half_pdt")]
static mut HIGHER_HALF_PDT: [u64; 512] = [0; 512];

static GDT: [u64; 2] = [0, (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)];

#[unsafe(link_section = ".kernel_content")]
static mut KERNEL_CONTENT: [u8; KERNEL_CONTENT_TOTAL_MEMSZ] = [0; KERNEL_CONTENT_TOTAL_MEMSZ];

/// Maps the kernel to higher half and returns the starting virtual address
unsafe fn map_kernel_to_higher_half(kernel_elf: &Elf) -> u64 {
    // 1. Identity map the first 2GB of the address space
    PML4[0] = (addr_of!(PDPT) as u32 | 0b11) as u64;
    PDPT[0] = (addr_of!(PDT) as u32 | 0b11) as u64;

    #[allow(static_mut_refs)]
    for (entry_idx, pdt_entry) in PDT.iter_mut().enumerate() {
        let entry = (0x200000 * entry_idx) | 0b10000011;
        *pdt_entry = entry as u64;
    }

    // 2. Map the kernel using 2MB pages to keep the paging structure simple.
    unsafe fn map_kernel_from_phdr(offset: &mut usize, phdr: &ProgramHeaderEntry) {
        let Some(phdr_content) = phdr.content() else {
            serial_println!(
                "Skipping program header because it has no content: {:?}",
                phdr
            );
            return;
        };
        serial_println!("Mapping kernel from program header: {:?}", phdr);

        fn align_up(value: u64, alignment: u64) -> u64 {
            (value + alignment - 1) & !(alignment - 1)
        }

        let phdr_vaddr = phdr.vaddr();
        let phdr_size = align_up(phdr.memsz(), 0x200000);
        let higher_half_pdt_index = (phdr_vaddr >> 21 & 0b111111111) as usize;

        let mut cur_addr = &raw const KERNEL_CONTENT[*offset] as u64;
        assert_eq!(cur_addr % 0x200000, 0);
        let end_addr = cur_addr + phdr_size;
        let mut entry_idx = 0;

        while cur_addr < end_addr {
            serial_println!("Mapping kernel from phdr = {:#x?}, entry_idx = {:#x?}, higher_half_pdt_index = {:#x?}", cur_addr, entry_idx, higher_half_pdt_index);
            serial_println!(
                "cur_addr(bin) = {:b}, cur_addr(hex) = {:#x?}",
                cur_addr,
                cur_addr
            );
            let entry = cur_addr | 0b10000011;
            HIGHER_HALF_PDT[higher_half_pdt_index + entry_idx as usize] = entry;

            cur_addr += 0x200000;
            entry_idx += 1;
        }

        KERNEL_CONTENT[*offset..(*offset + phdr_content.len())].copy_from_slice(phdr_content);
        offset.add_assign(phdr_size as usize);
    }

    let mut last_kernel_content_offset = 0;
    let mut phdr_iter = kernel_elf.program_header_iter();
    let first_phdr = phdr_iter.next().unwrap();
    let starting_vaddr = first_phdr.vaddr();

    if first_phdr.ph_type() == ProgramType::LOAD {
        map_kernel_from_phdr(&mut last_kernel_content_offset, &first_phdr);
    } else if first_phdr.ph_type() == ProgramType::DYNAMIC {
        panic!("Kernel is somehow a dynamically linked executable!");
    }

    let higher_half_pml4_index = (starting_vaddr >> 39 & 0b111111111) as usize;
    let higher_half_pdpt_index = (starting_vaddr >> 30 & 0b111111111) as usize;

    PML4[higher_half_pml4_index] = (addr_of!(HIGHER_HALF_PDPT) as u32 | 0b11) as u64;
    HIGHER_HALF_PDPT[higher_half_pdpt_index] = (addr_of!(HIGHER_HALF_PDT) as u32 | 0b11) as u64;

    for phdr in phdr_iter {
        if phdr.ph_type() == ProgramType::LOAD {
            map_kernel_from_phdr(&mut last_kernel_content_offset, &phdr);
        } else if phdr.ph_type() == ProgramType::DYNAMIC {
            panic!("Kernel is somehow a dynamically linked executable!");
        }
    }

    PML4[510] = addr_of!(PML4) as u64 | 0b11;

    #[allow(static_mut_refs)]
    let kernel_size = KERNEL_CONTENT.len();
    serial_println!(
        "Kernel size in memory = {} bytes or {} MB",
        kernel_size,
        kernel_size as f64 / MB as f64
    );

    let zero_out_section = |section_name: &str| {
        if let Some(section) = kernel_elf.lookup_section(section_name.as_bytes()) {
            serial_println!("Zeroing out section: {}", section_name);
            let section_vaddr = section.addr();
            let section_size = section.size();

            let base_offset = section_vaddr - first_phdr.vaddr();
            unsafe {
                KERNEL_CONTENT[base_offset as usize..(base_offset + section_size) as usize].fill(0);
            }
        } else {
            serial_println!("Skipping zeroing out section: {}", section_name);
        }
    };

    zero_out_section(".bss");
    zero_out_section(".kernel_stack");

    first_phdr.vaddr()
}

unsafe fn enable_paging() {
    unsafe {
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
}

unsafe fn load_gdt() {
    unsafe {
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
}

/// Passes information about the kernel's physical address after it is copied to [`KERNEL_CONTENT`].
///
/// NOTE:
/// Definition *must* match with `kernel::kutils::KernelContentInfo`
#[derive(Debug, Copy, Clone)]
#[repr(C, align(8))]
pub struct KernelContentInfo {
    pub virt_start_addr: u64,
    pub phys_start_addr: u32,
    pub phys_end_addr: u32,
}

// Inlining this function causes fault.
#[inline(never)]
unsafe fn call_kernel(mb_ptr: u32, kernel_content_info: u32, kernel_addr: u32) {
    unsafe {
        asm!(
    "mov edi, {}
    mov esi, {}
    // Offset for the code segment in the GDT
    push 0x8
    push {}
    retf", in(reg) mb_ptr, in(reg) kernel_content_info, in(reg) kernel_addr)
    }
}

unsafe fn display_logo() {
    let boot_info = BOOT_INFO.get().unwrap();
    let fb_tag = match boot_info
        .framebuffer_tag()
        .expect("FerricOxide cannot continue without a framebuffer")
    {
        Ok(fb_tag) => fb_tag,
        Err(e) => panic!("Got an unknown framebuffer type: {e}"),
    };

    let r;
    let g;
    let b;

    match fb_tag.buffer_type() {
        Ok(fb_type) => match fb_type {
            FramebufferType::RGB { red, green, blue } => {
                r = red;
                g = green;
                b = blue;
            }
            typ => panic!("Unsupported framebuffer type: {:?}", typ),
        },
        Err(e) => panic!("Unknown framebuffer type: {e}"),
    }

    let logo = include_bytes!("../../assets/FerricOxide.tga");

    // The logo is assumed to be a valid TGA file. Hence, there is no validation.
    // This is also the reason this function is marked unsafe.
    #[repr(packed, C)]
    struct TGAHeader {
        _magic1: u8,
        _colormap: u8,
        _encoding: u8,
        _cmaporig: u16,
        _cmaplen: u16,
        _cmapent: u8,
        _x: u16,
        _y: u16,
        w: u16,
        h: u16,
        bpp: u8,
        _pixeltype: u8,
    }

    let header = unsafe { &*(logo.as_ptr() as *const TGAHeader) };

    let img_height = header.h as u32;
    let img_width = header.w as u32;
    let img_bpp = header.bpp as u32;
    let img_pitch = (img_bpp / 8) * img_width;

    let image = unsafe {
        &*slice_from_raw_parts(
            (header as *const TGAHeader).offset(1) as *const u8,
            (img_height * img_pitch) as usize,
        )
    };

    let screen_height = fb_tag.height();
    let screen_width = fb_tag.width();

    assert!(img_height <= screen_height);
    assert!(img_width <= screen_width);

    let center_x = screen_width / 2;
    let center_y = screen_height / 2;

    let logo_x = center_x - (img_width / 2);
    let logo_y = center_y - (img_height / 2);

    let fb = unsafe {
        &mut *slice_from_raw_parts_mut(
            fb_tag.address() as *mut u8,
            (screen_height * fb_tag.pitch()) as usize,
        )
    };

    for y in 0..img_height {
        for x in 0..img_width {
            // find the position for pixel on screen
            let pixel_screen_x = logo_x + x;
            let pixel_screen_y = logo_y + y;

            // grab the pixel from the image
            let pixel_pos = (y * img_pitch + x * (img_bpp / 8)) as usize;
            let pixel_r = image[pixel_pos + 2];
            let pixel_g = image[pixel_pos + 1];
            let pixel_b = image[pixel_pos];

            let fb_pixel_pos = ((pixel_screen_y * fb_tag.pitch())
                + (pixel_screen_x * (fb_tag.bpp() as u32 / 8)))
                as usize;
            fb[(r.position / 8) as usize + fb_pixel_pos] = pixel_r;
            fb[(g.position / 8) as usize + fb_pixel_pos] = pixel_g;
            fb[(b.position / 8) as usize + fb_pixel_pos] = pixel_b;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "cdecl" fn prekernel_main(mb_ptr: *const BootInformationHeader) -> ! {
    let mb_info = unsafe { BootInformation::load(mb_ptr).unwrap() };
    BOOT_INFO.call_once(|| mb_info);

    unsafe {
        display_logo();
    }

    let kernel_start_addr = unsafe { &_binary_kernel_bin_start } as *const u16;
    let kernel_end_addr = unsafe { &_binary_kernel_bin_end } as *const u16;
    let kernel_size = kernel_end_addr as usize - kernel_start_addr as usize;

    serial_println!("Kernel ELF start: {:p}", kernel_start_addr);
    serial_println!("Kernel ELF size: {}KB", kernel_size as f32 / KB as f32);

    let kernel = unsafe {
        slice::from_raw_parts(
            kernel_start_addr as *const u8,
            kernel_end_addr as usize - kernel_start_addr as usize,
        )
    };

    let kernel_elf = Elf::from_bytes(kernel).unwrap();

    let kernel_higher_half_start_addr;
    unsafe {
        kernel_higher_half_start_addr = map_kernel_to_higher_half(&kernel_elf);
        enable_paging();
        load_gdt();
    }

    let kernel_text_section = kernel_elf.lookup_section(b".text").unwrap();
    let kernel_text_section_content = kernel_text_section.content().unwrap();
    for byte in &kernel_text_section_content[0..4] {
        serial_print!("{:x} ", byte);
    }
    serial_println!();

    let kernel_content_info;
    unsafe {
        let kernel_content_start = addr_of!(KERNEL_CONTENT) as u32;
        #[allow(static_mut_refs)]
        let kernel_content_end = kernel_content_start + KERNEL_CONTENT.len() as u32 - 1;
        kernel_content_info = KernelContentInfo {
            virt_start_addr: kernel_higher_half_start_addr,
            phys_start_addr: kernel_content_start,
            phys_end_addr: kernel_content_end,
        };
    }

    serial_println!("Calling the kernel...");
    unsafe {
        call_kernel(
            // offset shouldn't be big enough to cause issues when casting to 32bit unsigned integer
            mb_ptr as u32,
            addr_of!(kernel_content_info) as u32,
            addr_of!(KERNEL_CONTENT) as u32,
        );
    }

    unreachable!()
}
