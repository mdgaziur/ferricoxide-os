#![allow(dead_code)]

use crate::arch::x86_64::acpi::apic::InterruptControllerStructure;
use crate::arch::x86_64::acpi::{ACPISDT, SDT_LIST};
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::{PhysAddr, VirtAddr, identity_map, translate_addr};
use spin::Once;

static IOAPIC_BASE: Once<VirtAddr> = Once::new();

pub fn ioapic_write(reg: u32, value: u32) {
    let ioapic_base = *IOAPIC_BASE.get().unwrap();
    unsafe {
        core::ptr::write_volatile(ioapic_base as *mut u32, reg);
        core::ptr::write_volatile((ioapic_base + 0x10) as *mut u32, value);
    }
}

pub fn ioapic_read(reg: u32) -> u32 {
    let ioapic_base = *IOAPIC_BASE.get().unwrap();
    unsafe {
        core::ptr::write_volatile(ioapic_base as *mut u32, reg);
        core::ptr::read_volatile((ioapic_base + 0x10) as *const u32)
    }
}

pub fn set_ioapic_irq(irq: u8, vector: u8, lapic_id: u8) {
    let index = irq as u32 * 2;
    let low = (vector as u32) & 0xFF;
    let high = (lapic_id as u32) << 24;

    ioapic_write(0x10 + index + 1, high); // Set destination field
    ioapic_write(0x10 + index, low); // Set vector and flags
}

pub fn init() {
    let sdt_list = SDT_LIST.lock();

    // TODO: this won't work on SMP systems
    let ioapic_id = 0;
    for sdt in &*sdt_list {
        if let ACPISDT::APIC(apic_sdt) = sdt {
            for ics in &apic_sdt.interrupt_control_structure {
                if let InterruptControllerStructure::IOAPIC(ioapic) = ics {
                    if ioapic.ioapic_id == ioapic_id {
                        if translate_addr(ioapic.ioapic_address as VirtAddr).is_none() {
                            identity_map(
                                ioapic.ioapic_address as PhysAddr,
                                PageTableEntryFlags::PRESENT
                                    | PageTableEntryFlags::WRITABLE
                                    | PageTableEntryFlags::DISABLE_CACHE,
                            );
                        }

                        IOAPIC_BASE.call_once(|| ioapic.ioapic_address as VirtAddr);

                        return;
                    }
                }
            }
        }
    }

    panic!("IOAPIC: No IOAPIC found");
}
