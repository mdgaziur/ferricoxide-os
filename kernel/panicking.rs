use crate::arch::cpu::CPU;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    CPU::disable_interrupts();
    error!("Disabled interrupts");

    error!("Kernel panic");
    if let Some(message) = panic_info.message() {
        print_raw!("{}\n", message);
    }

    if let Some(location) = panic_info.location() {
        print_raw!("{}\n", location);
    }

    CPU::dump_registers();
    CPU::halt();
}
