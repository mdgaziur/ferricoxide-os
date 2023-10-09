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

pub use core::arch::asm;
pub use core::prelude::v1::*;

pub use crate::serial_println;

pub const BYTE: usize = 1;

#[allow(non_upper_case_globals)]
pub const KiB: usize = BYTE * 1024;

#[allow(non_upper_case_globals)]
pub const MiB: usize = KiB * 1024;

#[allow(non_upper_case_globals)]
pub const GiB: usize = MiB * 1024;
