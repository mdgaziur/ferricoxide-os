#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![no_std]

extern crate alloc;

use conquer_once::spin::OnceCell;
use multiboot2::BootInformation;
use crate::task::executor::Executor;
use crate::task::keyboard::print_keypresses;
use crate::task::Task;
use logging::vga::TextWriter;

use crate::vga::{VGA_DRAWER, VGADrawer};
use utils::multiboot::load_multiboot_info;
use utils::unsafe_sync::UnsafeSync;
use crate::arch::mm::MemoryController;

#[macro_use]
mod logging;

mod arch;
mod panicking;
mod task;
mod vga;

#[allow(unused)]
static NAME: &str = "FerricOxide OS";
static MEMORY_CONTROLLER: OnceCell<UnsafeSync<MemoryController>> = OnceCell::uninit();
static BOOT_INFO: OnceCell<UnsafeSync<BootInformation>> = OnceCell::uninit();

#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info_addr: usize) -> ! {
    let multiboot_info = load_multiboot_info(multiboot_info_addr);
    BOOT_INFO.init_once(move || {
        unsafe { UnsafeSync::new(multiboot_info) }
    });
    MEMORY_CONTROLLER.init_once(|| {
        unsafe { UnsafeSync::new(arch::initial_setup(BOOT_INFO.try_get().unwrap())) }
    });
    info!("Initialized architecture specific stuff");

    VGADrawer::init(BOOT_INFO.try_get().unwrap());
    info!("Initialized VGA drawer");

    VGA_DRAWER.lock().buffer.clear();
    info!("Cleared VGA drawer");

    TextWriter::init(BOOT_INFO.try_get().unwrap());
    info!("Initialized VGA Text writer");

    info!("Welcome to {}!", NAME);

    let mut executor = Executor::new();
    executor.spawn(Task::new(print_keypresses()));
    executor.run();
}
