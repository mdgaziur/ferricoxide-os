use crate::arch::x86_64_utils::initial_setup_x86_64;

#[cfg(target_arch = "x86_64")]
pub mod x86_64_utils;

pub mod commands;

pub struct VirtAddr(pub usize);
pub struct PhysAddr(pub usize);

pub fn initial_setup() {
    #[cfg(target_arch = "x86_64")]
    initial_setup_x86_64()
}
