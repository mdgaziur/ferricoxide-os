use crate::arch::mm::{
    display_heap_stats, MemoryController, HEAP_ALLOCATOR, HEAP_SIZE, HEAP_START,
};
use multiboot2::BootInformation;

pub mod cpu;
pub mod interrupts;
pub mod mm;
pub mod thread;
pub mod utils;

pub fn initial_setup_x86_64(boot_info: &BootInformation) -> MemoryController {
    utils::enable_nxe_bit();
    info!("Enabled nxe bit");

    utils::enable_write_protect_bit();
    info!("Enabled write protection bit");

    let mut memory_controller = mm::init(boot_info);
    info!("Initialized memory related stuff and remapped the kernel");
    unsafe {
        HEAP_ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
    info!("Initialized heap allocator");
    display_heap_stats();

    interrupts::init_interrupts(&mut memory_controller);
    info!("Initialized interrupts");

    memory_controller
}
