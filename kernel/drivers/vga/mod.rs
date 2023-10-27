/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2023  MD Gaziur Rahman Noor
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::BOOT_INFO;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use core::ptr;
use lazy_static::lazy_static;
use multiboot2::{FramebufferField, FramebufferType};
use spin::Mutex;

lazy_static! {
    pub static ref VGA_DRIVER_INSTANCE: Mutex<VGADriver> = Mutex::new(VGADriver::new());
}

#[derive(Debug)]
pub struct VGADriver {
    buffer: *mut u8,
    pitch: usize,
    height: usize,
    width: usize,
    bpp: usize,
    red_pos: FramebufferField,
    green_pos: FramebufferField,
    blue_pos: FramebufferField,
    second_buffer: Vec<u8>
}

impl VGADriver {
    pub fn new() -> Self {
        let boot_info = BOOT_INFO.get().unwrap();
        let framebuffer_tag = boot_info
            .framebuffer_tag()
            .expect("VGA driver requires the framebuffer tag")
            .unwrap();

        if let FramebufferType::RGB { blue, red, green } = framebuffer_tag.buffer_type().unwrap() {
            assert_eq!(
                framebuffer_tag.bpp(),
                24,
                "Only 24 bits per pixel framebuffer is supported"
            );

            Self {
                buffer: framebuffer_tag.address() as *mut u8,
                pitch: framebuffer_tag.pitch() as usize,
                height: framebuffer_tag.height() as usize,
                width: framebuffer_tag.width() as usize,
                bpp: framebuffer_tag.bpp() as usize,
                red_pos: red,
                green_pos: green,
                blue_pos: blue,
                second_buffer: vec![
                    0;
                    (framebuffer_tag.height() * framebuffer_tag.pitch()) as usize
                ],
            }
        } else {
            panic!("VGA driver only supports RGB framebuffer");
        }
    }

    pub fn fill(&mut self, mut pixel: Pixel) {
        for y in 0..self.height {
            for x in 0..self.width {
                pixel.x = x;
                pixel.y = y;

                self.plot_pixel(pixel);
            }
        }
    }

    pub fn plot_pixel(&mut self, pixel: Pixel) {
        assert!(pixel.x < self.width);
        assert!(pixel.y < self.height);

        let pixel_addr = (self.second_buffer.as_ptr() as usize
            + pixel.x * (self.bpp / 8)
            + pixel.y * self.pitch) as *mut u8;

        unsafe {
            *pixel_addr.offset((self.red_pos.position / self.red_pos.size) as isize) = pixel.r;
            *pixel_addr.offset((self.green_pos.position / self.green_pos.size) as isize) = pixel.g;
            *pixel_addr.offset((self.blue_pos.position / self.blue_pos.size) as isize) = pixel.b;
        }
    }

    pub fn swap(&mut self) {
        unsafe {
            ptr::copy_nonoverlapping(
                self.second_buffer.as_ptr(),
                self.buffer,
                self.height * self.width,
            )
        }
        // assert!(self.second_buffer == *unsafe { &*(self.buffer as *const [u8; 1440000]) });
    }
}

unsafe impl Send for VGADriver {}

#[derive(Debug, Copy, Clone)]
pub struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    x: usize,
    y: usize,
}

impl Pixel {
    pub fn new(r: u8, g: u8, b: u8, x: usize, y: usize) -> Self {
        Self { r, g, b, x, y }
    }
}
