use crate::arch::PhysAddr;
use core::arch::asm;

pub unsafe fn read_cr3() -> (PhysAddr, u16) {
    let value: u64;

    asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));

    (
        PhysAddr((value & 0x_000f_ffff_ffff_f000) as usize),
        (value & 0xFFF) as u16,
    )
}

pub unsafe fn write_cr3(addr: PhysAddr, val: u16) {
    let value = addr.0 as u64 | val as u64;

    asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
}
