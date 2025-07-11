use crate::display::display_text;
use crate::ds::RingBuffer;
use crate::kutils::DMESG_SIZE;
use crate::serial_print;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::{Mutex, Once};

pub static DMESG_RINGBUFFER: Once<Mutex<RingBuffer<String>>> = Once::new();
pub const D_DEBUG: &str = "Debug";
pub const D_INFO: &str = "Info";
pub const D_WARNING: &str = "Warning";
pub const D_ERROR: &str = "Error";
pub const D_EMERGENCY: &str = "Emergency";

pub fn dmesgln(msg: core::fmt::Arguments) {
    let msg_str = msg.to_string();
    display_text(&msg_str);
    serial_print!("{}", msg_str);
    DMESG_RINGBUFFER.get().unwrap().lock().insert(msg_str);
}

pub fn dmesg_get_all() -> Vec<String> {
    DMESG_RINGBUFFER.get().unwrap().lock().get_all().to_vec()
}

pub fn dmesg_init() {
    DMESG_RINGBUFFER.call_once(|| Mutex::new(RingBuffer::new(DMESG_SIZE)));
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! d {
    () => (format_args!("\n"));
    ($level:ident $fmt:expr) => (format_args!(concat!("[{} {}] ", concat!($fmt, "\n")), $level, $crate::arch::get_global_secs()));
    ($level:ident $fmt:expr, $($arg:tt)*) => (format_args!(
        concat!("[{} {}] ", concat!($fmt, "\n")), $level, $crate::arch::get_global_secs(), $($arg)*));
}
