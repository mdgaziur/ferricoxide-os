use crate::arch::x86_64::cpu::cpuid::{CPUIDECXFeature, cpuid_getfeatures};
use crate::serial_println;
use core::arch::asm;
use spin::Once;

pub static RDRAND_FN: Once<fn() -> Option<u64>> = Once::new();
pub static RDRAND_SUPPORTED: Once<bool> = Once::new();

fn rdrand_unsupported() -> Option<u64> {
    panic!("RDRAND not supported on this CPU");
}

fn rdrand_supported() -> Option<u64> {
    let mut value: u64 = 0;
    let mut carry: u8 = 0;

    let mut retries = 100;
    while retries > 0 {
        unsafe {
            asm!(
                "rdrand {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) carry,
            );
        }

        if carry != 0 {
            break;
        }

        retries -= 1;
    }

    if carry != 0 { Some(value) } else { None }
}

pub fn rand() -> Option<u64> {
    let rdrand_fn = *RDRAND_FN.get().unwrap();
    rdrand_fn()
}

pub fn is_rdrand_supported() -> bool {
    *RDRAND_SUPPORTED.get().unwrap()
}

pub fn init() {
    let (ecx_features, _) = cpuid_getfeatures();
    if ecx_features.contains(CPUIDECXFeature::RDRAND) {
        serial_println!("RDRAND is supported");
        RDRAND_SUPPORTED.call_once(|| true);
        RDRAND_FN.call_once(|| rdrand_supported);
    } else {
        serial_println!("RDRAND not is supported");
        RDRAND_SUPPORTED.call_once(|| false);
        RDRAND_FN.call_once(|| rdrand_unsupported);
    }
}
