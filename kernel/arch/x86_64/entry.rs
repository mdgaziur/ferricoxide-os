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

use core::fmt::Debug;
use multiboot2::BootInformation;
use uart_16550::SerialPort;
use crate::{print_to_screen, serial_println};

#[inline(never)]
pub fn really_kernel_start(boot_information: *const BootInformation) -> ! {
    print_to_screen("test123");

    let addr = 0xffffffff80000000 as *mut u8;
    unsafe { *addr = 0x66; }

    serial_println!("The kernel is aliveeeeeeee!!!!!!!!!!!");
    loop {}
}
