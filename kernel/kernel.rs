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

#![no_std]
#![feature(prelude_import)]

mod arch;
mod kprelude;
mod serial;

use crate::arch::entry::arch_entry;
use core::hint::spin_loop;
use core::panic::PanicInfo;
#[prelude_import]
use kprelude::*;

/// This is the Rust entry point of the kernel. At first it calls the architecture specific entry
/// function to set up various architecture specific stuff like memory management, interrupt handlers,
/// GDT(x86_64) etc.
///
/// The architecture specific entrypoint is supposed to set up a valid heap for the kernel, the heap
/// allocator, interrupt handlers, real time clock, etc. Such stuff are done to make sure that kmain1
/// can proceed with performing further advanced tasks like initializing the VGA driver, loading file
/// system, enabling virtual console, initializing SMP, initializing the task scheduler and spawning
/// the first process of the kernel which has PID 0. That entry of that process is kmain2 which
/// handles other stuff like enabling the mouse and keyboard, loading up all the necessary TTYs, and then
/// spawning init.
///
/// ## NOTE
///
/// The kernel will refuse to boot in case it detects that the VGA framebuffer address is not present
/// in the multiboot info structure. This limitation is present because for now the kernel solely depends
/// on the bootloader to initialize VGA mode. The kernel does *not* support both text mode and VGA mode
/// to keep things simple. One consequence this approach has is that for a big chunk of the early boot
/// stage the kernel will basically "hang" if anything fails(from the perspective of the user).
/// Unfortunately the only way to debug the kernel in this case is to check out the output from QEMU
/// in terminal(if used). If anything fails when running on bare metal, the user is out of luck unless
/// s/he can read the outputs from COM1 port(0x3F8).
#[no_mangle]
fn kmain1(multiboot_info_addr: usize) -> ! {
    // SAFETY: the address is loaded directly into edi from which the first argument(multiboot_info_addr)
    // is loaded. The code in boot.s also ensures that the proper startup sequence is being followed before
    // jumping to kmain1.
    unsafe {
        arch_entry(multiboot_info_addr);
    }

    loop {
        spin_loop()
    }
}

#[panic_handler]
fn panic_handler(_panic_info: &PanicInfo) -> ! {
    loop {
        spin_loop()
    }
}
