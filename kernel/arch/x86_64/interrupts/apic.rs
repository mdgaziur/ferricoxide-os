#![allow(unused)]

use crate::arch::x86_64::cpu::cpuid::{CPUIDEDXFeature, cpuid_getfeatures};
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::{PhysAddr, identity_map, allocate_page_and_map};
use crate::serial_println;
use spin::Once;
use x86_64::registers::model_specific::Msr;
use crate::arch::x86_64::mm::paging::PAGE_SIZE;

const IA32_APIC_BASE_MSR: u32 = 0x1B;
const EOI_REG: u32 = 0xB0;
const SPURIOUS_INTERRUPT_VECTOR_REG: u32 = 0xF0;
const TASK_PRIORITY_REG: u32 = 0x80;

static APIC_BASE: Once<u64> = Once::new();

fn check_apic() -> bool {
    cpuid_getfeatures().1.contains(CPUIDEDXFeature::APIC)
}

fn get_apic_base() -> u64 {
    let msr_val = unsafe { Msr::new(IA32_APIC_BASE_MSR).read() };

    // We're in long mode, so there's no PAE to worry about
    msr_val & 0xfffff000
}

/// # Safety
///
/// Make sure that there's nothing residing at the APIC base address
/// other than the APIC itself.
unsafe fn set_apic_base(apic_base: u64) {
    unsafe {
        Msr::new(IA32_APIC_BASE_MSR).write(apic_base);
    }
}

/// # Safety
///
/// Make sure that the APIC base address is valid and that
/// the APIC is enabled.
unsafe fn write_reg_apic(apic_base: u64, reg: u32, value: u32) {
    let addr = (apic_base + reg as u64) as *mut u32;
    unsafe {
        addr.write_volatile(value);
    }
}

/// # Safety
///
/// Make sure that the APIC base address is valid and that
/// the APIC is enabled.
unsafe fn read_reg_apic(apic_base: u64, reg: u32) -> u32 {
    let addr = (apic_base + reg as u64) as *const u32;
    unsafe { addr.read_volatile() }
}

pub fn notify_end_of_interrupt() {
    // Safety
    // We're using the global `APIC_BASE` which
    // is set to a valid address after `apic::init()` is
    // called. `spin::Once` ensures that `APIC_BASE` is initialized
    // before it is used
    unsafe {
        write_reg_apic(*APIC_BASE.get().unwrap(), EOI_REG, 0);
    }
}

pub fn init() {
    // Check if APIC is supported
    if !check_apic() {
        panic!("APIC is not supported on this system");
    }

    // Hardware enable APIC if not enabled already
    let apic_base = get_apic_base();
    let virtual_apic_base = allocate_page_and_map(
        apic_base as PhysAddr,
        PAGE_SIZE,
        PageTableEntryFlags::PRESENT
            | PageTableEntryFlags::WRITABLE
            | PageTableEntryFlags::DISABLE_CACHE,
    ).unwrap().1;
    serial_println!("APIC base: {:x?}", virtual_apic_base);
    // Safety:
    // We're just setting the APIC base address to the value we just read
    unsafe {
        set_apic_base(apic_base);
    }

    // Safety:
    // We're using the APIC base address we just set
    unsafe {
        write_reg_apic(
            virtual_apic_base as u64,
            SPURIOUS_INTERRUPT_VECTOR_REG,
            read_reg_apic(virtual_apic_base as u64, SPURIOUS_INTERRUPT_VECTOR_REG) | 0x100,
        );
    }

    APIC_BASE.call_once(|| virtual_apic_base as u64);
}
