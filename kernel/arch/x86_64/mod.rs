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

mod entry;

use crate::{print_to_screen, serial_println};
use core::arch::asm;
use core::fmt::{Arguments, Write};
use multiboot2::BootInformation;
use uart_16550::SerialPort;
use crate::arch::x86_64::entry::really_kernel_start;

#[no_mangle]
unsafe extern "C" fn kernel_start(boot_information: *const BootInformation) -> ! {
    asm!(
        "mov ax, 0
        mov ss, ax
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax"
    );

    really_kernel_start(boot_information);
}
