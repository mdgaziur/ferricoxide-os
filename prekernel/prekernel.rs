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

#![no_std]
#![feature(naked_functions)]

/// # Prekernel
///
/// The idea is kind of loosely borrowed from SerenityOS. The main goal is to load the main
/// kernel into memory, put it into [0xFFFFFFFF80000000]. For that, the
/// prekernel creates necessary page tables and maps the kernel to that address. After that,
/// it activates the page table, enables long mode and jumps to the kernel code.
///
/// The entrypoint resides in `arch/x86/boot/boot.asm` in the x86 architecture. That code
/// then calls `prekernel_main` located in `arch/x86/main.rs`.
///
/// The plan is to put the main kernel image in the `kernel_image` section
mod arch;
mod kprintf;
mod kutils;

use core::panic::PanicInfo;

extern "C" {
    static _binary_kernel_bin_start: u16;

    static _binary_kernel_bin_end: u16;

    static pml4: u8;

    static pdpt: u8;

    static pdt: u8;

    static pt: u8;
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);
    loop {}
}
