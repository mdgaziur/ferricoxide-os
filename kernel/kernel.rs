#![feature(panic_info_message)]
#![feature(default_alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(prelude_import)]
#![no_std]

extern crate alloc;

use crate::task::executor::Executor;
use crate::task::keyboard::print_keypresses;
use crate::task::Task;
use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use logging::vga::TextWriter;
use multiboot2::BootInformation;
use spin::Mutex;

use crate::arch::mm::MemoryController;
use crate::vga::{VGADrawer, VGA_DRAWER};
use kutils::multiboot::load_multiboot_info;
use kutils::unsafe_sync::UnsafeSync;

#[macro_use]
mod logging;

mod arch;
mod fs;
mod kprelude;
mod kutils;
mod panicking;
mod task;
mod vga;

use crate::fs::path::Path;
use crate::fs::ramfs::RamFS;
use crate::fs::vfs::VFS;
use crate::fs::FSNodeType;
#[prelude_import]
use kprelude::*;

#[allow(unused)]
static NAME: &str = "FerricOxide OS";
static MEMORY_CONTROLLER: OnceCell<UnsafeSync<MemoryController>> = OnceCell::uninit();
static BOOT_INFO: OnceCell<UnsafeSync<BootInformation>> = OnceCell::uninit();

#[no_mangle]
pub extern "C" fn kmain(multiboot_info_addr: usize) -> ! {
    let multiboot_info = load_multiboot_info(multiboot_info_addr);
    BOOT_INFO.init_once(move || unsafe { UnsafeSync::new(multiboot_info) });
    MEMORY_CONTROLLER.init_once(|| unsafe {
        UnsafeSync::new(arch::initial_setup(BOOT_INFO.try_get().unwrap()))
    });
    info!("Initialized architecture specific stuff");

    VGADrawer::init(BOOT_INFO.try_get().unwrap());
    info!("Initialized VGA drawer");

    if VGA_DRAWER.lock().is_init() {
        VGA_DRAWER.lock().buffer.clear();
    }
    info!("Cleared VGA drawer");

    TextWriter::init(BOOT_INFO.try_get().unwrap());
    info!("Initialized VGA Text writer");

    info!("Welcome to {}!", NAME);

    VFS.lock()
        .mount(Path::new("/"), Arc::new(Mutex::new(Box::new(RamFS::new()))));

    for i in 0..10 {
        VFS.lock()
            .create_file(Path::new(&format!("/file-{}.txt", i)))
            .expect("Failed to create file");
    }

    VFS.lock()
        .create_dir(Path::new("/a_dir"))
        .expect("Failed to create dir");
    for fsnode in VFS.lock().list_path(Path::new("/a_dir")).unwrap() {
        info!("{}", fsnode);
    }

    for i in 0..10 {
        VFS.lock()
            .create_file(Path::new(&format!("/a_dir/file-{}.txt", i)))
            .expect("Failed to create file");
    }

    let fsnodes = VFS.lock().list_path(Path::new("/")).unwrap();
    for fsnode in fsnodes {
        info!("{}", fsnode);
        if fsnode.typ() == FSNodeType::Dir {
            for fsnode in VFS.lock().list_path(fsnode.path()).unwrap() {
                info!("  - {}", fsnode);
            }
        }
    }
    info!(
        "File size: {}",
        VFS.lock().fsize(Path::new("/file-0.txt")).unwrap()
    );
    let file = VFS.lock().open(Path::new("/file-0.txt")).unwrap();
    info!(
        "Increased {} bytes in size",
        VFS.lock()
            .write(&file, b"Hello world!".to_vec(), 0, 11)
            .unwrap()
    );
    info!(
        "Increased {} bytes in size",
        VFS.lock().write(&file, vec![], 0, 0).unwrap()
    );
    info!(
        "File content: {:?}",
        String::from_utf8(VFS.lock().read(&file, 0, 11).unwrap()).unwrap()
    );
    info!(
        "Replacing `H` with `R` for on reason. Increased {} bytes in size",
        VFS.lock().write(&file, b"R".to_vec(), 0, 0).unwrap()
    );
    info!(
        "File content: {:?}",
        String::from_utf8(VFS.lock().read(&file, 0, 11).unwrap()).unwrap()
    );

    let mut executor = Executor::new();
    executor.spawn(Task::new(print_keypresses()));
    executor.run();
}
