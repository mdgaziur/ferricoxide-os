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

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

pub fn get_global_ms() -> u64 {
    #[cfg(target_arch = "x86_64")]
    interrupts::pit8254::get_global_ms()
}

pub fn get_global_secs() -> f64 {
    get_global_ms() as f64 / 1000_f64
}

pub fn sleep(millis: u64) {
    #[cfg(target_arch = "x86_64")]
    interrupts::pit8254::pit_sleep(millis);
}
