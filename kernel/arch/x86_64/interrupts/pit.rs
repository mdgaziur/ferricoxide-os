use crate::arch::x86_64::cpu::halt;
use crate::arch::x86_64::interrupts::apic::notify_end_of_interrupt;
use crate::arch::x86_64::interrupts::ioapic::set_ioapic_irq;
use crate::arch::x86_64::io::{inb, outb};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::idt::InterruptStackFrame;

pub const PIT_FREQUENCY: u32 = 1193182;
pub const TIMER_VECTOR: u8 = 0x20;
pub const TIMER_IRQ: u8 = 0x2;
pub const TIMER_FREQUENCY: u32 = 100; // 100 Hz
// FIXME: timer freq is twice as fast for some reason
pub const TIMER_COUNT: u16 = (PIT_FREQUENCY / TIMER_FREQUENCY * 2) as _;
static TICKS: AtomicU64 = AtomicU64::new(0);

#[allow(dead_code)]
pub fn read_pit_count() -> u16 {
    let mut count: u16;
    unsafe {
        outb(0x43, 0x00);
        count = inb(0x40) as u16;
        count |= (inb(0x40) as u16) << 8;
    }
    count
}

pub fn set_pit_count(count: u16) {
    without_interrupts(|| unsafe {
        outb(0x43, 0x36);
        outb(0x40, (count & 0xFF) as u8);
        outb(0x40, ((count & 0xFF00) >> 8) as u8);
    })
}

pub extern "x86-interrupt" fn pit_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);

    notify_end_of_interrupt();
}

pub fn pit_sleep(millis: u64) {
    let start = TICKS.load(Ordering::Relaxed);
    let target_ticks = millis / 10;

    while (TICKS.load(Ordering::Relaxed) - start) <= target_ticks {
        halt();
    }
}

pub fn init() {
    set_pit_count(TIMER_COUNT);
    set_ioapic_irq(TIMER_IRQ, TIMER_VECTOR, 0);
}
