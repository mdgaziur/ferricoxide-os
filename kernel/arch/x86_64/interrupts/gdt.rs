use bit_field::BitField;
use core::mem::size_of;
use x86_64::instructions::tables::lgdt;
use x86_64::structures::gdt::{Descriptor, DescriptorFlags, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::DescriptorTablePointer;
use x86_64::{PrivilegeLevel, VirtAddr};

pub struct Gdt {
    table: [u64; 8],
    next_free: usize,
}

impl Gdt {
    pub fn new() -> Self {
        Gdt {
            table: [0; 8],
            next_free: 1,
        }
    }

    pub fn add_entry(&mut self, entry: Descriptor) -> SegmentSelector {
        let index = match entry {
            Descriptor::UserSegment(value) => self.push(value),
            Descriptor::SystemSegment(value_low, value_high) => {
                let index = self.push(value_low);
                self.push(value_high);
                index
            }
        };

        SegmentSelector::new(index as u16, PrivilegeLevel::Ring0)
    }

    pub fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            base: VirtAddr::new(self.table.as_ptr() as u64),
            limit: (self.table.len() * size_of::<u64>() - 1) as u16,
        };

        unsafe { lgdt(&ptr) }
    }

    fn push(&mut self, value: u64) -> usize {
        if self.next_free < self.table.len() {
            let index = self.next_free;
            self.table[index] = value;
            self.next_free += 1;
            index
        } else {
            panic!("GDT full");
        }
    }

    pub fn kernel_code_segment() -> Descriptor {
        let flags = DescriptorFlags::USER_SEGMENT
            | DescriptorFlags::PRESENT
            | DescriptorFlags::EXECUTABLE
            | DescriptorFlags::LONG_MODE;
        Descriptor::UserSegment(flags.bits())
    }

    pub fn tss_segment(tss: &'static TaskStateSegment) -> Descriptor {
        let ptr = tss as *const _ as u64;

        let mut low = DescriptorFlags::PRESENT.bits();
        low.set_bits(16..40, ptr.get_bits(0..24));
        low.set_bits(56..64, ptr.get_bits(24..32));
        low.set_bits(0..16, (size_of::<TaskStateSegment>() - 1) as u64);
        low.set_bits(40..44, 0b1001);

        let mut high = 0;
        high.set_bits(0..32, ptr.get_bits(32..64));

        Descriptor::SystemSegment(low, high)
    }
}
