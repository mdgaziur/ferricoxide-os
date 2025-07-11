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

mod acpi;
mod cpu;
mod gdt;
pub(super) mod interrupts;
mod io;
mod mm;

use crate::kutils::{KERNEL_STACK_SIZE, KernelContentInfo};
use crate::{BOOT_INFO, display, kernel_main, serial_println};
use core::arch::naked_asm;
use core::ptr::addr_of;
use multiboot2::{BootInformation, BootInformationHeader};
use spin::Once;

use crate::dbg::dmesg_init;
pub use cpu::halt_loop;

pub(super) static KERNEL_CONTENT_INFO: Once<KernelContentInfo> = Once::new();

#[unsafe(link_section = ".kernel_stack")]
static KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

#[allow(dead_code)]
#[unsafe(link_section = ".stackoverflow_guard")]
static STACKOVERFLOW_GUARD: [u8; 4096] = [0; 4096];

#[unsafe(no_mangle)]
static KERNEL_STACK_TOP: &u8 = &KERNEL_STACK[KERNEL_STACK.len() - 1];

#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn kernel_start() {
    naked_asm!(
        "mov ax, 0
        mov ss, ax
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
    
        // set the NXE bit
        mov rcx, 0xC0000080
        rdmsr
        or rax, 1 << 11
        wrmsr
    
        // enable write protection
        mov rax, cr0
        or rax, 1 << 16
        mov cr0, rax
    
        mov rsp, KERNEL_STACK_TOP
    
        // Jump to the higher half address of `actually_kernel_start`
        // so that gdb can point out which part of the kernel we are executing
        lea rax, [actually_kernel_start]
        push rax
        ret"
    );
}

#[unsafe(no_mangle)]
fn actually_kernel_start(
    boot_information_header: *const BootInformationHeader,
    kernel_content_info: *const KernelContentInfo,
) -> ! {
    gdt::init();

    let mb_info = unsafe { BootInformation::load(boot_information_header).unwrap() };
    BOOT_INFO.call_once(|| mb_info);
    serial_println!("{:p}", kernel_content_info);
    KERNEL_CONTENT_INFO.call_once(|| unsafe { *kernel_content_info });

    serial_println!("The kernel is aliveeeeeeee!!!!!!!!!!!");
    serial_println!("KernelContentInfo:");
    serial_println!(
        "    phys_start_addr: 0x{:x}",
        KERNEL_CONTENT_INFO.get().unwrap().phys_start_addr
    );
    serial_println!(
        "    phys_end_addr: 0x{:x}",
        KERNEL_CONTENT_INFO.get().unwrap().phys_end_addr
    );
    serial_println!("KERNEL_STACK: {:p}", addr_of!(KERNEL_STACK));
    serial_println!("KERNEL_STACK end: {:p}", KERNEL_STACK_TOP);

    let mut vendor = [0; 13];
    cpu::cpuid::cpuid_get_vendor(&mut vendor);
    serial_println!("Vendor: {}", str::from_utf8(&vendor).unwrap());
    serial_println!("Vendor(bytes): {:?}", vendor);

    mm::init();
    acpi::init();
    interrupts::init();
    unsafe {
        display::init();
    }
    dmesg_init();

    kernel_main();
}
