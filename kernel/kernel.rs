#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
extern crate alloc;

mod arch;
mod dbg;
mod display;
mod ds;
mod kprintf;
mod kutils;
mod fs;
mod process;

use crate::arch::sleep;
use crate::dbg::{D_INFO, dmesgln};
use core::panic::PanicInfo;
use multiboot2::BootInformation;
use spin::Once;

pub static BOOT_INFO: Once<BootInformation> = Once::new();

pub fn kernel_main() -> ! {
    // Previously we used serial_println, but starting from `kernel_main`, we will use
    // `dmesgln` to print kernel messages
    dmesgln(d!(D_INFO "Hello from Ferricoxide OS!"));

    let mut secs = 0;
    loop {
        dmesgln(d!(D_INFO "Secs: {:0>4}", secs));
        sleep(1000);
        secs += 1;
    }

    arch::halt_loop()
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);

    arch::halt_loop();
}
