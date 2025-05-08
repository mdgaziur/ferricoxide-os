/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2025  MD Gaziur Rahman Noor
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

use crate::arch::x86_64::mm::VirtAddr;
use core::arch::asm;

pub fn flush_tlb(addr: VirtAddr) {
    unsafe {
        asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

pub fn flush_tlb_all() {
    write_cr3(unsafe { read_cr3() });
}

pub unsafe fn read_cr3() -> u64 {
    unsafe {
        let mut value: u64;
        asm!("mov {}, cr3", out(reg) value, options(nostack, preserves_flags));

        value
    }
}

pub fn write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

pub fn halt_loop() -> ! {
    unsafe {
        asm!("cli", options(nostack, preserves_flags));
    }

    loop {
        unsafe {
            asm!("hlt", options(nostack, preserves_flags));
        }
    }
}
