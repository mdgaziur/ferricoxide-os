use crate::arch::commands::halt_loop;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    serial_println!("Disabled interrupts");

    serial_println!("Kernel panic: {}", panic_info);
    if let Some(message) = panic_info.message() {
        serial_println!("{}", message);
    }

    if let Some(location) = panic_info.location() {
        serial_println!("Location: {}", location);
    }

    halt_loop();
}
