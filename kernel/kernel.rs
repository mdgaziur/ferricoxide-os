#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
extern crate alloc;

mod arch;
mod ds;
mod kprintf;
mod kutils;
mod display;
mod dbg;

use core::panic::PanicInfo;
use spin::Once;
use multiboot2::BootInformation;
use crate::arch::sleep;
use crate::dbg::{dmesgln, dmesg_get_all, D_INFO};
use crate::display::{FRAMEBUFFER, TEXT_RENDERER};
use crate::display::framebuffer::Pixel;

pub static BOOT_INFO: Once<BootInformation> = Once::new();

pub fn kernel_main() -> ! {
    // Previously we used serial_println, but starting from `kernel_main`, we will use
    // `dmesgln` to print kernel messages
    dmesgln(d!(D_INFO "Hello from Ferricoxide OS!"));

    arch::halt_loop()
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", pi);

    arch::halt_loop();
}