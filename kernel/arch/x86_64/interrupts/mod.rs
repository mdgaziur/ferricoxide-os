use crate::arch::x86_64::cpu::halt_loop;
use crate::arch::x86_64::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::arch::x86_64::interrupts::pit::{TIMER_VECTOR, pit_handler, pit_sleep};
use crate::serial_println;
use core::arch::asm;
use lazy_static::lazy_static;
use x86_64::instructions::interrupts;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

mod apic;
mod ioapic;
mod pit;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(pagefault_handler);
        idt.divide_error.set_handler_fn(divide_by_zero);
        idt[TIMER_VECTOR].set_handler_fn(pit_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        };
        idt
    };
}

extern "x86-interrupt" fn divide_by_zero(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: DIVIDE BY ZERO");
    serial_println!("Stack frame: {:#?}", stack_frame);

    halt_loop()
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT");
    serial_println!("Stack frame: {:#?}", stack_frame);

    halt_loop()
}

extern "x86-interrupt" fn pagefault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Error code: {:?}", error_code);
    serial_println!("Stack frame: {:#?}", stack_frame);

    halt_loop()
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    serial_println!("EXCEPTION: DOUBLE FAULT");
    serial_println!("Stack frame: {:#?}", stack_frame);
    serial_println!("Error code: {}", error_code);

    halt_loop()
}

fn mask_pic() {
    unsafe {
        asm!(
            "
                mov al, 0xFF
                out 0xA1, al        // Mask all IRQs on PIC2
                out 0x21, al        // Mask all IRQs on PIC1
            "
        )
    }
}

pub fn sleep(millis: u64) {
    pit_sleep(millis);
}

pub fn init() {
    mask_pic();

    IDT.load();

    apic::init();
    ioapic::init();
    pit::init();

    interrupts::enable();
}
