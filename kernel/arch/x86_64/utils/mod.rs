use core::arch::asm;
use x86_64::registers::model_specific::Msr;

pub fn enable_nxe_bit() {
    let nxe_bit = 1 << 11;
    let mut msr = Msr::new(3221225600);

    unsafe {
        let efer = msr.read();
        msr.write(efer | nxe_bit);
    }
}

pub fn enable_write_protect_bit() {
    let write_protect_bit = 1 << 16;
    let value: u64;

    unsafe {
        asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }

    unsafe {
        asm!("mov cr0, {}", in(reg) value | write_protect_bit, options(nostack, preserves_flags));
    }
}
