pub use alloc::boxed::Box;
pub use alloc::format;
pub use alloc::string::String;
pub use alloc::vec;
pub use alloc::vec::Vec;
pub use core::arch::asm;
pub use core::arch::global_asm;
pub use core::prelude::rust_2021::*;

#[allow(unused)]
pub const BYTE: usize = 1;
#[allow(unused)]
pub const KB: usize = 1024 * BYTE;
#[allow(unused)]
pub const MB: usize = 1024 * KB;
#[allow(unused)]
pub const GB: usize = 1024 * MB;
