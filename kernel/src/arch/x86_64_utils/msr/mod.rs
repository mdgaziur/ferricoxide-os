use core::arch::asm;

pub struct MSR(pub u32);

impl MSR {
    pub fn rdmsr(&self) -> u64 {
        let (high, low): (u32, u32);

        unsafe {
            asm!(
            "rdmsr",
            in("ecx") self.0,
            out("eax") low, out("edx") high,
            options(nomem, nostack, preserves_flags),
            );
        };

        (high as u64) << 32 | low as u64
    }

    pub fn wrmsr(&mut self, value: u64) {
        let low = value as u32;
        let high = (value >> 32) as u32;

        unsafe {
            asm!(
            "wrmsr",
            in("ecx") self.0,
            in("eax") low, in("edx") high,
            options(nostack, preserves_flags),
            );
        }
    }
}
