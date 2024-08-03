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

use crate::{dbg, serial_println};
use core::arch::asm;
use multiboot2::{BootInformation, BootInformationHeader};
use spin::Once;

pub(super) static BOOT_INFO: Once<BootInformation> = Once::new();

#[no_mangle]
unsafe extern "C" fn kernel_start(boot_information_header: *const BootInformationHeader) -> ! {
    asm!(
        "mov ax, 0
        mov ss, ax
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax"
    );

    let mb_info = unsafe { BootInformation::load(boot_information_header).unwrap() };
    BOOT_INFO.call_once(|| mb_info);

    serial_println!("The kernel is aliveeeeeeee!!!!!!!!!!!");

    dbg!(BOOT_INFO.get().unwrap());

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
