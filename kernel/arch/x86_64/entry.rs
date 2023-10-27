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

use crate::arch::{mm, mm_bck};
use crate::{serial_println, BOOT_INFO};

pub unsafe fn arch_entry() {
    let boot_info = BOOT_INFO.get().unwrap();
    serial_println!(
        "Kernel was loaded by \"{}\"",
        boot_info
            .boot_loader_name_tag()
            .unwrap()
            .name()
            .unwrap_or("<invalid bootloader>")
    );

    mm::init(&boot_info);
    // mm_bck::init(&boot_info);
}
