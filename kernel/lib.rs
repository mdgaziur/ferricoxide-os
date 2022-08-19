#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![no_std]

extern crate alloc;

use crate::task::executor::Executor;
use crate::task::keyboard::print_keypresses;
use crate::task::Task;
use logging::vga::TextWriter;

use crate::vga::{VGA_DRAWER, VGADrawer};
use utils::multiboot::load_multiboot_info;

#[macro_use]
mod logging;

mod arch;
mod panicking;
mod task;
mod vga;
mod process;

#[allow(unused)]
static NAME: &str = "FerricOxide OS";

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_addr: usize) -> ! {
    let multiboot_info = load_multiboot_info(multiboot_info_addr);

    arch::initial_setup(&multiboot_info);
    info!("Initialized architecture specific stuff");

    VGADrawer::init(&multiboot_info);
    info!("Initialized VGA drawer");

    VGA_DRAWER.lock().unwrap_ref_mut().buffer.clear();
    info!("Cleared VGA drawer");

    TextWriter::init(&multiboot_info);
    info!("Initialized VGA Text writer");

    info!("Welcome to {}!", NAME);

    let mut executor = Executor::new();
    executor.spawn(Task::new(print_keypresses()));
    executor.run();
}
