#![allow(unused)]

use crate::arch::x86_64::acpi::{ACPISDTHeader, RawACPISDTHeader};
use crate::arch::x86_64::mm::PhysAddr;
use crate::serial_println;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::bitflags;
use core::mem::transmute;

#[repr(C, packed)]
pub struct RawAPICSDT {
    _signature: u32,
    length: u32,
    _revision: u8,
    _checksum: u8,
    _oem_id: [u8; 6],
    _oem_table_id: [u8; 8],
    _oem_revision: u32,
    _creator_id: u32,
    _creator_revision: u32,
    lapic_address: u32,
    flags: u32,
    interrupt_control_structure: u8,
}

#[repr(C, packed)]
pub struct RawInterruptControllerStructure {
    r#type: u8,
    length: u8,
}

pub struct APICSDT {
    pub header: ACPISDTHeader,
    pub lapic_address: u32,
    pub flags: MultipleAPICFlags,
    pub interrupt_control_structure: Vec<InterruptControllerStructure>,
}

pub enum InterruptControllerStructure {
    LocalAPIC(LocalAPIC),
    IOAPIC(IOAPIC),
    InterruptSourceOverride(InterruptSourceOverride),
    LocalAPICNMI(LocalAPICNMI),
    Other {
        structure: &'static RawInterruptControllerStructure,
    },
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct MultipleAPICFlags: u32 {
        const PCAT_COMPAT = 1 << 0;
    }
}

#[repr(C, packed)]
struct RawLocalAPIC {
    r#type: u8,
    length: u8,
    processor_uid: u8,
    apic_id: u8,
    flags: LocalAPICFlags,
}

#[derive(Debug, Copy, Clone)]
pub struct LocalAPIC {
    pub processor_uid: u8,
    pub apic_id: u8,
    pub flags: LocalAPICFlags,
    pub addr: PhysAddr,
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct LocalAPICFlags: u32 {
        const ENABLED = 1 << 0;
        const ONLINE = 1 << 1;
    }
}

#[repr(C, packed)]
struct RawIOAPIC {
    r#type: u8,
    length: u8,
    ioapic_id: u8,
    reserved: u8,
    ioapic_address: u32,
    global_system_interrupt_base: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct IOAPIC {
    pub ioapic_id: u8,
    pub ioapic_address: u32,
    pub global_system_interrupt_base: u32,
    pub addr: PhysAddr,
}

#[repr(C, packed)]
struct RawInterruptSourceOverride {
    r#type: u8,
    length: u8,
    bus_source: u8,
    interrupt_source: u8,
    global_system_interrupt: u32,
    flags: u16,
}

#[derive(Debug, Copy, Clone)]
pub struct InterruptSourceOverride {
    pub bus_source: u8,
    pub interrupt_source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
    pub addr: PhysAddr,
}

#[repr(C, packed)]
struct RawLocalAPICNMI {
    r#type: u8,
    length: u8,
    processor_uid: u8,
    flags: u16,
    local_apic_nmi_lint: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct LocalAPICNMI {
    pub processor_uid: u8,
    pub flags: u16,
    pub local_apic_nmi_lint: u8,
    pub addr: u64,
}

const ICS_LOCAL_APIC: u8 = 0;
const ICS_IO_APIC: u8 = 1;
const ICS_INTERRUPT_SOURCE_OVERRIDE: u8 = 2;
const ICS_LOCAL_APIC_NMI: u8 = 4;

unsafe fn parse_interrupt_control_structure(
    raw_interrupt_controller_structure: &'static RawInterruptControllerStructure,
) -> InterruptControllerStructure {
    match raw_interrupt_controller_structure.r#type {
        ICS_LOCAL_APIC => {
            serial_println!("APIC: got processor local APIC structure");
            let raw_local_apic: &RawLocalAPIC =
                unsafe { transmute(raw_interrupt_controller_structure) };
            let local_apic = LocalAPIC {
                processor_uid: raw_local_apic.processor_uid,
                apic_id: raw_local_apic.apic_id,
                flags: raw_local_apic.flags,
                addr: &raw const *raw_interrupt_controller_structure as _,
            };

            serial_println!("APIC: Local APIC: {:?}", local_apic);
            InterruptControllerStructure::LocalAPIC(local_apic)
        }
        ICS_IO_APIC => {
            serial_println!("APIC: got IO APIC structure");
            let raw_ioapic: &RawIOAPIC = unsafe { transmute(raw_interrupt_controller_structure) };
            let ioapic = IOAPIC {
                ioapic_id: raw_ioapic.ioapic_id,
                ioapic_address: raw_ioapic.ioapic_address,
                global_system_interrupt_base: raw_ioapic.global_system_interrupt_base,
                addr: &raw const *raw_interrupt_controller_structure as _,
            };

            serial_println!("APIC: IOAPIC: {:?}", ioapic);
            InterruptControllerStructure::IOAPIC(ioapic)
        }
        ICS_INTERRUPT_SOURCE_OVERRIDE => {
            serial_println!("APIC: got interrupt source override structure");
            let raw_interrupt_source_override: &RawInterruptSourceOverride =
                unsafe { transmute(raw_interrupt_controller_structure) };
            let interrupt_source_override = InterruptSourceOverride {
                bus_source: raw_interrupt_source_override.bus_source,
                interrupt_source: raw_interrupt_source_override.interrupt_source,
                global_system_interrupt: raw_interrupt_source_override.global_system_interrupt,
                flags: raw_interrupt_source_override.flags,
                addr: &raw const *raw_interrupt_controller_structure as _,
            };

            serial_println!(
                "APIC: Interrupt source override: {:?}",
                interrupt_source_override
            );
            InterruptControllerStructure::InterruptSourceOverride(interrupt_source_override)
        }
        ICS_LOCAL_APIC_NMI => {
            serial_println!("APIC: got Local APIC NMI structure");
            let raw_local_apic_nmi: &RawLocalAPICNMI =
                unsafe { transmute(raw_interrupt_controller_structure) };
            let local_apic_nmi = LocalAPICNMI {
                processor_uid: raw_local_apic_nmi.processor_uid,
                flags: raw_local_apic_nmi.flags,
                local_apic_nmi_lint: raw_local_apic_nmi.local_apic_nmi_lint,
                addr: &raw const *raw_interrupt_controller_structure as _,
            };

            serial_println!("APIC: Local APIC NMI: {:?}", local_apic_nmi);
            InterruptControllerStructure::LocalAPICNMI(local_apic_nmi)
        }
        _ => {
            serial_println!(
                "APIC: skipping unknown interrupt controller structure: type: {}, length: {}",
                raw_interrupt_controller_structure.r#type,
                raw_interrupt_controller_structure.length
            );

            InterruptControllerStructure::Other {
                structure: raw_interrupt_controller_structure,
            }
        }
    }
}

/// # Safety:
///
/// Make sure that the pointer to `raw_sdt_ptr` is valid
pub unsafe fn parse_apic_sdt(sdt_header: ACPISDTHeader) -> APICSDT {
    let raw_apic_sdt: &RawAPICSDT = unsafe { &*(sdt_header.raw_addr as *const RawAPICSDT) };
    let lapic_address = raw_apic_sdt.lapic_address;
    let flags = MultipleAPICFlags::from_bits_truncate(raw_apic_sdt.flags);

    serial_println!("APIC: LAPIC address: {:#x?}", lapic_address);
    serial_println!("APIC: Flags: {:?}", flags);

    let mut cur_addr = &raw const raw_apic_sdt.interrupt_control_structure as PhysAddr;
    let end_addr = sdt_header.raw_addr as PhysAddr + raw_apic_sdt.length as PhysAddr;

    let mut interrupt_control_structures = vec![];
    while cur_addr < end_addr {
        let structure = unsafe { &*(cur_addr as *const RawInterruptControllerStructure) };
        let type_ = structure.r#type;
        let length = structure.length;
        serial_println!(
            "APIC: Interrupt controller structure: type: {}, length: {}",
            type_,
            length
        );

        // Safety:
        // We're using the pointer to the interrupt controller structure
        // which is valid because we just read it from the APIC SDT
        interrupt_control_structures.push(unsafe { parse_interrupt_control_structure(structure) });

        cur_addr += length as PhysAddr;
    }

    APICSDT {
        header: sdt_header,
        lapic_address,
        flags,
        interrupt_control_structure: interrupt_control_structures,
    }
}
