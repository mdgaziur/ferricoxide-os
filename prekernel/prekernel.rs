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
/// The idea is kind of loosely borrowed from SerenityOS. The goal is to map the kernel into
/// `0xFFFFFFFF80000000` and start the kernel's execution from there.
///
/// At first, the prekernel identity maps the first 2GB of the address space. Then, the kernel is
/// copied into a buffer of which the address is 2MB aligned. After that, that buffer is mapped into
/// `0xFFFFFFFF80000000`. Then paging and long mode is enabled. Finally, the prekernel calls `kernel_start`
/// to start execution of the kernel.
///
/// Some assumptions:
/// - The kernel will never exceed 2GB in size(duh!). Why: the page table setup for higher half kernel
///   is done in such a way that it only requires 2 additional PDPT and PDT. This limits the kernel to
///   be at most 2GB in size.
/// - The kernel is at most 8MB in size(for now). Why: the `KERNEL_CONTENT` buffer is 8MB in size. In
///   case the kernel exceeds that size, the size of `KERNEL_CONTENT` must be increased to ensure that
///   the kernel fits there.
/// - The kernel is assumed to be a statically linked executable.
///
///
mod arch;
mod kprintf;
mod kutils;

use core::panic::PanicInfo;

extern "C" {
    static _binary_kernel_bin_start: u16;

    static _binary_kernel_bin_end: u16;
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);
    loop {}
}
