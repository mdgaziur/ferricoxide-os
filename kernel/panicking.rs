use crate::arch::cpu::Cpu;
use crate::logging::serial::QEMU_SERIAL;
use crate::logging::vga::WRITER;
use crate::vga::VGA_DRAWER;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic_handler(panic_info: &PanicInfo) -> ! {
    Cpu::disable_interrupts();

    // Force unlock to prevent deadlocks while displaying error information
    unsafe {
        QEMU_SERIAL.force_unlock();
        WRITER.force_unlock();
        VGA_DRAWER.force_unlock();
    }

    error!("Kernel panic");
    error!("Disabled interrupts");

    if let Some(message) = panic_info.message() {
        error!("Message: {}", message);
    }

    if let Some(location) = panic_info.location() {
        error!("Location: {}", location);
    }

    error!("Dumping registers");
    Cpu::dump_registers();

    error!("Kernel will not continue!");
    Cpu::halt();
}
