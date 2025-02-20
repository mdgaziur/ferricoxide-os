#![feature(naked_functions)]
#![no_std]
#![no_main]

mod arch;
mod ds;
mod kprintf;
mod kutils;

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
