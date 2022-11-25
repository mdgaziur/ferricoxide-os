pub mod registers;

use crate::arch::cpu::registers::Registers;
use core::arch::asm;

pub struct Cpu;

impl Cpu {
    pub fn enable_interrupts_and_halt() {
        x86_64::instructions::interrupts::enable_and_hlt();
    }

    pub fn enable_interrupts() {
        x86_64::instructions::interrupts::enable();
    }

    pub fn disable_interrupts() {
        x86_64::instructions::interrupts::disable();
    }

    pub fn without_interrupts<F>(f: F)
    where
        F: Fn(),
    {
        x86_64::instructions::interrupts::without_interrupts(f);
    }

    pub fn dump_registers() {
        Registers::read_regs().dump_regs();
    }

    pub fn halt() -> ! {
        loop {
            unsafe {
                asm!("hlt");
            }
        }
    }
}
