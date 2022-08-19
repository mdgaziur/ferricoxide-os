use core::panic::PanicInfo;
use crate::arch::cpu::CPU;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    CPU::disable_interrupts();
    error!("Disabled interrupts");

    error!("Kernel panic: {}", panic_info);
    if let Some(message) = panic_info.message() {
        error!("{}", message);
    }

    if let Some(location) = panic_info.location() {
        error!("Location: {}", location);
    }

    CPU::halt();
}
