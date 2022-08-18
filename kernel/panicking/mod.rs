use crate::arch::commands::halt_loop;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    error!("Disabled interrupts");

    error!("Kernel panic: {}", panic_info);
    if let Some(message) = panic_info.message() {
        error!("{}", message);
    }

    if let Some(location) = panic_info.location() {
        error!("Location: {}", location);
    }

    halt_loop();
}
