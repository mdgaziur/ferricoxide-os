#![feature(panic_info_message)]
#![feature(naked_functions)]
#![no_std]
#![no_main]

mod arch;
mod kprintf;

use core::panic::PanicInfo;

fn print_to_screen(s: &str) {
    let mut x = 0;
    let mut y = 0;

    for ch in s.chars() {
        if x >= 80 {
            y += 1;
            x = 0;
        }

        if y >= 60 {
            x = 0;
            y = 0;
        }

        let ptr = (0xb8000 + (y * 80 + x) * 2) as *mut u16;
        let attrib = 0xF;
        unsafe {
            *ptr = ch as u16 | attrib << 8;
        }

        x += 1;
    }
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}
