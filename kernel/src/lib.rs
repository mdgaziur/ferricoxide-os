#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![no_std]

extern crate alloc;

use crate::task::executor::Executor;
use crate::task::keyboard::print_keypresses;
use crate::task::Task;



use logging::vga::{TextWriter};
use mm::{HEAP_ALLOCATOR, HEAP_SIZE, HEAP_START};

use crate::vga::VGADrawer;
use utils::multiboot::load_multiboot_info;

#[macro_use]
mod logging;

mod arch;
mod interrupts;
mod mm;
mod panicking;
mod task;
mod vga;

#[allow(unused)]
static NAME: &str = "FerricOxide OS";

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_addr: usize) -> ! {
    let multiboot_info = load_multiboot_info(multiboot_info_addr);

    arch::initial_setup();
    info!("Initialized architecture specific stuff");

    let mut memory_controller = mm::init(&multiboot_info);
    info!("Initialized memory related stuff and remapped the kernel");
    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
    info!("Initialized heap allocator");

    interrupts::init_interrupts(&mut memory_controller);
    info!("Initialized interrupts");

    VGADrawer::init(&multiboot_info);
    TextWriter::init(&multiboot_info);
    info!("Initialized VGA Text writer");

    info!("Welcome to {}!", NAME);
    let mut executor = Executor::new();
    executor.spawn(Task::new(print_keypresses()));
    executor.run();
}
