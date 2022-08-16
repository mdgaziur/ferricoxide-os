#[macro_use]
pub mod vga;

#[macro_use]
pub mod serial;

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! print {
    ($prefix_colored:literal, $prefix:literal, $($arg:tt)*) => {
        (|msg: core::fmt::Arguments| {
            serial_print!("{} {}", $prefix_colored, msg);
            vprint!("{} {}", $prefix, msg);
        })(format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! print_raw {
    ($($arg:tt)*) => {
        (|msg: core::fmt::Arguments| {
            serial_print!("{}", msg);
            vprint!("{}", msg);
        })(format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! info {
    ($fmt:expr) => (print!("\x1B[1;34m[ Info  ]\x1B[0m", "[ Info  ]", concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!("\x1B[1;34m[ Info  ]\x1B[0m", "[ Info  ]", concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! error {
    ($fmt:expr) => (print!("\x1B[1;31m[ Error ]\x1B[0m", "[ Error ]", concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!("\x1B[1;31m[ Error ]\x1B[0m", "[ Error ]", concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! warn {
    ($fmt:expr) => (print!("\x1B[1;33m[ Warn  ]\x1B[0m", "[ Warn  ]", concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!("\x1B[1;33m[ Warn  ]\x1B[0m", "[ Warn  ]", concat!($fmt, "\n"), $($arg)*));
}
