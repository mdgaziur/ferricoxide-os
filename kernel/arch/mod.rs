use crate::arch::mm::MemoryController;
use crate::arch::x86_64::initial_setup_x86_64;
use multiboot2::BootInformation;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;
pub use self::x86_64::*;

pub fn initial_setup(boot_info: &BootInformation) -> MemoryController {
    #[cfg(target_arch = "x86_64")]
    initial_setup_x86_64(boot_info)
}
