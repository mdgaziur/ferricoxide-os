use crate::arch::{PhysAddr, VirtAddr};

pub fn halt_loop() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            crate::arch::x86_64_utils::instructions::hlt();
        }
    }
}

pub fn tlb_flush(addr: VirtAddr) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        crate::arch::x86_64_utils::instructions::invlpg(addr)
    }
}

pub fn tlb_flush_all() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        crate::arch::x86_64_utils::tlb::flush_all();
    }
}

pub fn read_cr3() -> (PhysAddr, u16) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        crate::arch::x86_64_utils::cr3::read_cr3()
    }
}

pub unsafe fn write_cr3(addr: PhysAddr, val: u16) {
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64_utils::cr3::write_cr3(addr, val);
}
