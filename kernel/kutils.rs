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

pub const ADDRESS_SPACE_SIZE: usize = 256 * GB;
pub const KERNEL_STACK_SIZE: usize = 4 * MB;

#[allow(unused)]
pub const GB: usize = MB * 1024;

#[allow(unused)]
pub const MB: usize = KB * 1024;

#[allow(unused)]
pub const KB: usize = 1024;

#[macro_export]
macro_rules! verify_called_once {
    () => {
        {
            use spin::Once;
            static HAS_BEEN_CALLED: Once<bool> = Once::new();
            if HAS_BEEN_CALLED.get().is_some() {
                panic!("Attempt to call a function that was supposed to call only once more than one time!");
            } else {
                HAS_BEEN_CALLED.call_once(|| {
                    true
                });
            }
        }
    };
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct KernelContentInfo {
    pub phys_start_addr: u32,
    pub phys_end_addr: u32,
}
