use multiboot2::BootInformation;
use crate::arch::x86_64::initial_setup_x86_64;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;
pub use self::x86_64::*;

pub fn initial_setup(boot_info: &BootInformation) {
    #[cfg(target_arch = "x86_64")]
    initial_setup_x86_64(boot_info)
}
