use core::arch::asm;

pub unsafe fn flush_all() {
    let value: u64;

    asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
}
