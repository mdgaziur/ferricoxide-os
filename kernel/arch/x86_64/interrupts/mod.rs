use crate::arch::x86_64::cpu::halt_loop;
use crate::arch::x86_64::gdt::DOUBLE_FAULT_IST_INDEX;
use crate::serial_println;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(pagefault_handler);
        idt.divide_error.set_handler_fn(divide_by_zero);
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

pub fn init() {
    IDT.load();
}
