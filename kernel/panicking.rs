use crate::arch::cpu::Cpu;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    Cpu::disable_interrupts();
    error!("Disabled interrupts");

    error!("Kernel panic");
    if let Some(message) = panic_info.message() {
        print_raw!("{}\n", message);
    }

    if let Some(location) = panic_info.location() {
        print_raw!("{}\n", location);
    }

    Cpu::dump_registers();
    Cpu::halt();
}
