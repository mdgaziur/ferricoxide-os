use crate::arch::VirtAddr;
use core::arch::asm;

pub unsafe fn hlt() {
    asm!("hlt");
}

pub unsafe fn invlpg(addr: VirtAddr) {
    asm!("invlpg [{}]", in(reg) addr.0 as u64, options(nostack, preserves_flags));
}
