use alloc::vec;
use crate::BOOT_INFO;
use alloc::vec::Vec;
use core::{ptr, slice};
use multiboot2::{FramebufferField, FramebufferType};
#[derive(Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    pub const WHITE: Pixel = Pixel { r: 255, g: 255, b: 255 };
    
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Pixel { r, g, b }
    }

    pub fn new_with_intensity(r: u8, g: u8, b: u8, intensity: f32) -> Self {
        Pixel { r: (r as f32 * intensity) as u8, g: (g as f32 * intensity) as u8, b: (b as f32 * intensity) as u8 }
    }

    pub fn white(intensity: u8) -> Self {
        Pixel { r: intensity, g: intensity, b: intensity }
    }
    
    pub fn apply_intensity(&mut self, intensity: u8) -> Self {
        let intensity_clamped_to_1 = intensity as f32 / 255.0;
        
        Self {
            r: (self.r as f32 * intensity_clamped_to_1) as u8,
            g: (self.g as f32 * intensity_clamped_to_1) as u8,
            b: (self.b as f32 * intensity_clamped_to_1) as u8,
        }
    }
}

#[derive(Debug)]
pub struct Framebuffer<'fb> {
    first_buffer: &'fb mut [u8],
    second_buffer: Vec<u8>,
    pitch: u32,
    pub height: u32,
    pub width: u32,
    bpp: u8,
    advance_per_pixel: u8,
    red_pos: FramebufferField,
    green_pos: FramebufferField,
    blue_pos: FramebufferField,
}

impl<'fb> Framebuffer<'fb> {
    /// Creates a new framebuffer with the information from [`BOOT_INFO`]
    /// 
    /// # Panics:
    /// This will panic if:
    /// - There's no framebuffer tag in the boot info
    /// - The framebuffer type is unknown
    /// - The bpp(bits per pixel) of the framebuffer is not 24 bpp
    /// 
    /// # Safety: 
    /// Caller *must* ensure that the boot info is valid and the framebuffer is mapped
    /// correctly into the virtual address space.
    pub unsafe fn new() -> Framebuffer<'fb> {
        let boot_info = BOOT_INFO.get().unwrap();
        let fb_tag = boot_info
            .framebuffer_tag()
            .expect("Framebuffer tag not available")
            .expect("Got unknown framebuffer type");

        let fb_type = fb_tag.buffer_type().expect("Got unknown framebuffer type");
        if let FramebufferType::RGB { blue, red, green } = fb_type {
            assert!(fb_tag.bpp() >= 24, "Framebuffer bpp must be at least 24 bpp");

            let first_buffer = unsafe {
                slice::from_raw_parts_mut(
                    fb_tag.address() as *mut u8,
                    (fb_tag.height() * fb_tag.pitch()) as usize,
                )
            };
            
            let second_buffer = first_buffer[..first_buffer.len()].to_vec();
            Self {
                second_buffer,
                first_buffer,
                pitch: fb_tag.pitch(),
                height: fb_tag.height(),
                width: fb_tag.width(),
                bpp: fb_tag.bpp(),
                advance_per_pixel: fb_tag.bpp() / 8,
                red_pos: red,
                green_pos: green,
                blue_pos: blue,
            }
        } else {
            panic!("Only RGB framebuffers are supported");
        }
    }

    pub fn fill(&mut self, color: Pixel) {
        let mut idx = 0;

        while (idx + self.advance_per_pixel as usize) < self.second_buffer.len() {
            self.second_buffer[idx + (self.red_pos.position / 8) as usize] = color.r;
            self.second_buffer[idx + (self.green_pos.position / 8) as usize] = color.g;
            self.second_buffer[idx + (self.blue_pos.position / 8) as usize] = color.b;

            idx += self.advance_per_pixel as usize;
        }
    }

    #[inline(always)]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: Pixel) {
        assert!(x < self.width as usize);
        assert!(y < self.height as usize);

        let pos = y * self.pitch as usize + (x * (self.bpp as usize / 8));
        self.second_buffer[pos + (self.red_pos.position / 8) as usize] = color.r;
        self.second_buffer[pos + (self.blue_pos.position / 8) as usize] = color.g;
        self.second_buffer[pos + (self.green_pos.position / 8) as usize] = color.b;
    }

    pub fn scroll_up(&mut self, rows: usize) {
        let bytes = rows * self.pitch as usize;

        for i in bytes..self.second_buffer.len() {
            self.second_buffer[i - bytes] = self.second_buffer[i];
        }
    }

    pub fn flush(&mut self) {
        unsafe {
            ptr::copy(self.second_buffer.as_ptr(), self.first_buffer.as_mut_ptr(), self.second_buffer.len())
        }
    }
}
