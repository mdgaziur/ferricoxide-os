#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
extern crate alloc;

mod arch;
mod ds;
mod kprintf;
mod kutils;

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);

    arch::halt_loop();
}
