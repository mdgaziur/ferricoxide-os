#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]
extern crate alloc;

mod arch;
mod dbg;
mod display;
mod ds;
mod fs;
mod io;
mod kprintf;
mod kutils;
mod posix;
mod process;
mod time;

use crate::arch::rand;
use crate::dbg::{D_EMERGENCY, D_INFO, dmesgln, is_dmesg_available};
use crate::fs::path::Path;
use crate::fs::ramfs::RAMFS_NAME;
use crate::fs::{DEntryKind, Mode, OpenFlags, SeekOffset, get_vfs};
use alloc::string::{String, ToString};
use alloc::{format, vec};
use core::panic::PanicInfo;
use multiboot2::BootInformation;
use spin::Once;

pub static BOOT_INFO: Once<BootInformation> = Once::new();

pub fn kernel_main() -> ! {
    // Previously we used serial_println, but starting from `kernel_main`, we will use
    // `dmesgln` to print kernel messages
    dmesgln(d!(D_INFO "Hello from Ferricoxide OS!"));
    dmesgln(d!(D_INFO "is_rdrand_supported = {}", rand::is_rdrand_supported()));

    fs::init();
    {
        let vfs = get_vfs();
        {
            let mut vfs_write = vfs.write();
            let root = vfs_write
                .open(Path::new("/"), None, OpenFlags::O_RDONLY)
                .unwrap();
            dmesgln(d!(D_INFO "Root FS: {:?}", vfs_write.stat(&root)));
            vfs_write.mkdir(Path::new("/mnt"), None, Mode(0)).unwrap();
            vfs_write
                .mount(Path::new("/mnt"), None, RAMFS_NAME)
                .unwrap();

            for i in 0..5 {
                vfs_write
                    .open(
                        Path::new(&format!("/testfile{i}.txt")),
                        None,
                        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
                    )
                    .unwrap();
            }

            for i in 0..5 {
                vfs_write
                    .open(
                        Path::new(&format!("/mnt/mnttestfile{i}.txt")),
                        None,
                        OpenFlags::O_CREAT | OpenFlags::O_RDWR,
                    )
                    .unwrap();
            }
            vfs_write.umount(Path::new("/mnt"), None).unwrap();
            dmesgln(d!(D_INFO "Listing root directory:"));
            for entry in vfs_write.readdir(&root).unwrap() {
                dmesgln(d!(D_INFO " - {:?}", entry.name));

                if entry.name == "mnt" {
                    let mnt_handle = vfs_write
                        .open(Path::new("/mnt"), None, OpenFlags::O_RDONLY)
                        .unwrap();
                    for entry1 in vfs_write.readdir(&mnt_handle).unwrap() {
                        dmesgln(d!(D_INFO "   - {}", entry1.name));
                    }
                }
            }

            let test_path = Path::new("/././././../../mnt/../////testfile0.txt");
            dmesgln(
                d!(D_INFO "Cannonicalizing path: {}: {}", test_path, match vfs_write.canonicalize_path(&test_path, None) {
                    Ok(path) => path.to_string(),
                    Err(e) => format!("(error: {:?})", e),
                }),
            );

            vfs_write.mkdir(Path::new("/test"), None, Mode(0)).unwrap();
            let test_dir = vfs_write
                .open(Path::new("/test"), None, OpenFlags::O_RDONLY)
                .unwrap();
            dmesgln(d!(D_INFO "test_dir.stat = {:?}", vfs_write.stat(&test_dir)));
            vfs_write.rmdir(Path::new("/test"), None).unwrap();
            dmesgln(d!(D_INFO "test_dir.stat = {:?}", vfs_write.stat(&test_dir)));

            let mut test_txt = vfs_write
                .open(Path::new("/testfile0.txt"), None, OpenFlags::O_RDWR)
                .unwrap();
            dmesgln(d!(D_INFO "test_txt.stat = {:?}", vfs_write.stat(&test_txt)));

            // write "hello world" to test_txt
            let hello = b"Hello World";
            vfs_write
                .write(&mut test_txt, hello, hello.len() as u64)
                .expect("Failed to write to test.txt");
            dmesgln(d!(D_INFO "test_txt.stat = {:?}", vfs_write.stat(&test_txt)));

            test_txt
                .lseek(0, SeekOffset::Set)
                .expect("Failed to lseek testfile0.txt");

            let mut read_buf = [0u8; 11];
            let read_buf_len = read_buf.len() as u64;
            vfs_write
                .read(&mut test_txt, &mut read_buf, read_buf_len)
                .expect("Failed to read from test.txt");

            dmesgln(
                d!(D_INFO "Read from test_txt: {:?}", core::str::from_utf8(&read_buf).unwrap()),
            );
            dmesgln(d!(D_INFO "test_txt.stat = {:?}", vfs_write.stat(&test_txt)));

            vfs_write.close(test_txt).expect("Failed to close test.txt");

            dmesgln(d!(D_INFO "Creating symlink /testlink -> /testfile0.txt"));
            vfs_write
                .symlink("/testfile0.txt", &Path::new("/testlink"), None)
                .expect("Failed to symlink testfile0.txt");

            dmesgln(d!(D_INFO "Opening at symlink /testlink"));
            let mut testlink = vfs_write
                .open(Path::new("/testlink"), None, OpenFlags::O_RDWR)
                .expect("Failed to read testlink");

            dmesgln(d!(D_INFO "Writing to symlink /testlink"));
            let symlink_hello = b"Hello Symlink";
            vfs_write
                .write(&mut testlink, symlink_hello, symlink_hello.len() as u64)
                .expect("Failed to write to testlink");
            testlink
                .lseek(0, SeekOffset::Set)
                .expect("Failed to lseek testfile0.txt");

            let testlink_stat = vfs_write.stat(&testlink).expect("Failed to stat testlink");
            dmesgln(d!(D_INFO "testlink.stat = {:?}", testlink_stat));
            let mut read_buf = vec![0u8; testlink_stat.size as usize];
            let read_buf_len = read_buf.len() as u64;
            vfs_write
                .read(&mut testlink, &mut read_buf, read_buf_len)
                .expect("Failed to read from testlink");
            let testlink_content = core::str::from_utf8(&read_buf).unwrap().to_string();
            dmesgln(d!(D_INFO "Read from testlink: {:?}", testlink_content));

            // read from testfile0.txt
            dmesgln(d!(D_INFO "Opening at /testfile0.txt"));
            let mut testfile0_read = vfs_write
                .open(Path::new("/testfile0.txt"), None, OpenFlags::O_RDWR)
                .expect("Failed to read testfile0.txt");
            vfs_write
                .read(&mut testfile0_read, &mut read_buf, read_buf_len)
                .expect("Failed to read from testfile0.txt");
            let testfile0_content = core::str::from_utf8(&read_buf).unwrap().to_string();
            dmesgln(
                d!(D_INFO "testfile0.txt content: `{:?}`, testlink content: `{:?}`", testfile0_content, testlink_content),
            );
            assert_eq!(testlink_content, testfile0_content);

            dmesgln(d!(D_INFO "Listing root directory:"));
            for entry in vfs_write.readdir(&root).unwrap() {
                dmesgln(d!(D_INFO " - {:?}: {:?} {}", entry.name, entry.kind,
                match entry.kind {
                    DEntryKind::Symlink => format!("-> {}", vfs_write.readlink(&Path::new(&format!("/{}", entry.name)), None).unwrap()),
                    _ => String::new(),
                }));
            }
        }
    }

    arch::halt_loop()
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    if is_dmesg_available() {
        dmesgln(d!(D_EMERGENCY "PANIC: {}", pi));
    } else {
        serial_println!("PANIC: {}", pi);
    }

    arch::halt_loop();
}
