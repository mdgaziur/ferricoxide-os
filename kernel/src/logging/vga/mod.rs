#[forbid(unsafe_code)]

mod bufferwriter;
mod textmodewriter;

use crate::logging::vga::bufferwriter::BufferWriter;
use crate::logging::vga::textmodewriter::TextModeWriter;
use core::fmt::Write;
use lazy_static::lazy_static;
use multiboot2::{BootInformation, FramebufferType};
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;




lazy_static! {
    pub static ref WRITER: Mutex<TextWriter> = Mutex::new(TextWriter::uninit());
}

pub struct TextWriter {
    writer: Writer,
}

impl TextWriter {
    pub fn uninit() -> Self {
        Self {
            writer: Writer::Uninitialized,
        }
    }

    pub fn init(boot_info: &BootInformation) {
        if let Some(framebuffer_tag) = boot_info.framebuffer_tag() {
            if let FramebufferType::RGB { .. } = framebuffer_tag.buffer_type {
                *WRITER.lock() = Self {
                    writer: Writer::FrameBuffer(BufferWriter::new(
                        framebuffer_tag.height as usize,
                        framebuffer_tag.width as usize,
                    )),
                };

                return;
            }
        }

        *WRITER.lock() = Self {
            writer: Writer::TextMode(unsafe { TextModeWriter::init() }),
        };
    }
}

impl Write for TextWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match &mut self.writer {
            Writer::FrameBuffer(writer) => writer.write_str(s),
            Writer::TextMode(writer) => writer.write_str(s),
            _ => Ok(()), // Will get printed to serial output instead
        }
    }
}

enum Writer {
    FrameBuffer(BufferWriter),
    TextMode(TextModeWriter),
    Uninitialized,
}

#[doc(hidden)]
pub fn print(args: core::fmt::Arguments) {
    without_interrupts(|| WRITER.lock().write_fmt(args).unwrap());
}

#[macro_export]
macro_rules! vprintln {
    () => (vprint!("\n"));
    ($fmt:expr) => (vprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (vprint!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! vprint {
    ($($arg:tt)*) => ({
        $crate::logging::vga::print(format_args!($($arg)*));
    });
}
