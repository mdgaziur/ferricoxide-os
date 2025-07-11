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
pub const TIMER_FREQUENCY: u32 = 1000; // 1 kHz
// PIT is running in square wave generator mode, so we need to double the frequency to get the correct timer
// count.
pub const TIMER_COUNT: u16 = (PIT_FREQUENCY / (TIMER_FREQUENCY * 2)) as u16;
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
        let mut command = 0;
        command |= 0; // BCD/Binary mode: Binary mode
        command |= 010 << 1; // Operating mode: Square wave generator
        command |= 11 << 4; // Access mode: lobyte/hibyte
        command |= 00 << 5; // Channel select: Channel 0
        outb(0x43, command);
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
    let target_ticks = millis;

    while (TICKS.load(Ordering::Relaxed) - start) <= target_ticks {
        halt();
    }
}

pub fn init() {
    if PIT_FREQUENCY % TIMER_FREQUENCY > TIMER_FREQUENCY / 2 {
        set_pit_count(TIMER_COUNT + 1);
    } else {
        set_pit_count(TIMER_COUNT);
    }
    set_ioapic_irq(TIMER_IRQ, TIMER_VECTOR, 0);
}
