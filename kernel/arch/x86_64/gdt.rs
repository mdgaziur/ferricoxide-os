use core::arch::asm;
use core::ptr::addr_of;

pub fn load_gdt(gdt: &[u64]) {
    unsafe {
        #[repr(C, packed(2))]
        struct DescriptorTablePointer {
            limit: u16,
            base: u64,
        }

        let pointer = DescriptorTablePointer {
            limit: (size_of_val(gdt) - 1) as u16,
            base: gdt.as_ptr() as u64,
        };

        asm!("lgdt [{}]", in(reg) addr_of!(pointer));
    }
}
