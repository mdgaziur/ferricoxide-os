use crate::kutils::possibly_uninit::PossiblyUninit;
use multiboot2::{BootInformation, FramebufferType};
use spin::Mutex;

pub static VGA_DRAWER: Mutex<PossiblyUninit<VGADrawer>> = Mutex::new(PossiblyUninit::Uninit);

#[derive(Debug)]
pub struct VGADrawer {
    pub buffer: VGAFramebuffer,
}

impl VGADrawer {
    pub fn init(boot_info: &BootInformation) {
        if let Some(framebuffer_tag) = boot_info.framebuffer_tag() {
            if let FramebufferType::RGB { .. } = framebuffer_tag.buffer_type {
                unsafe {
                    *VGA_DRAWER.lock() = PossiblyUninit::Init(Self {
                        buffer: VGAFramebuffer::new(
                            framebuffer_tag.address,
                            framebuffer_tag.height as usize,
                            framebuffer_tag.width as usize,
                            framebuffer_tag.pitch as usize,
                            framebuffer_tag.bpp as usize,
                        ),
                    });
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct VGAFramebuffer {
    secondary_buffer: Vec<u8>,
    buffer: *mut u8,
    height: usize,
    width: usize,
    pitch: usize,
    bpp: usize,
}

impl VGAFramebuffer {
    pub unsafe fn new(addr: u64, height: usize, width: usize, pitch: usize, bpp: usize) -> Self {
        Self {
            secondary_buffer: vec![0; height * pitch],
            buffer: addr as *mut u8,
            height,
            width,
            pitch,
            bpp,
        }
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.write_pixel(Pixel { r: 0, g: 0, b: 0 }, x, y);
            }
        }
    }

    fn clear_y(&mut self, pos_y: usize) {
        for pos_x in 0..self.width {
            self.write_pixel(Pixel { r: 0, g: 0, b: 0 }, pos_x, pos_y);
        }
    }

    fn get_pixel(&self, pos_x: usize, pos_y: usize) -> Pixel {
        assert!(pos_x < self.width);
        assert!(pos_y < self.height);

        let pixel_addr =
            (self.buffer as usize + pos_x * (self.bpp / 8) + pos_y * self.pitch) as *mut u8;

        let r;
        let g;
        let b;

        unsafe {
            r = *pixel_addr;
            g = *pixel_addr.offset(1);
            b = *pixel_addr.offset(2);
        }

        Pixel { r, g, b }
    }

    pub fn write_pixel(&mut self, pixel: Pixel, pos_x: usize, pos_y: usize) {
        assert!(pos_x < self.width);
        assert!(pos_y < self.height);

        let pixel_addr = (self.secondary_buffer.as_ptr() as usize
            + pos_x * (self.bpp / 8)
            + pos_y * self.pitch) as *mut u8;

        unsafe {
            *pixel_addr = pixel.r;
            *pixel_addr.offset(1) = pixel.g;
            *pixel_addr.offset(2) = pixel.b;
        }
    }

    pub fn commit(&mut self) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.secondary_buffer.as_ptr(),
                self.buffer,
                self.height * self.pitch,
            );
        }
    }
}

/// # SAFETY
/// It is safe to share vga framebuffer between threads
unsafe impl Send for VGAFramebuffer {}

#[derive(Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
