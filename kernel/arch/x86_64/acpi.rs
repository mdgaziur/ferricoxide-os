#![allow(unused)]
#![allow(clippy::upper_case_acronyms)]

pub mod apic;

use crate::BOOT_INFO;
use crate::arch::x86_64::acpi::apic::APICSDT;
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::{PhysAddr, VirtAddr, identity_map, translate_addr};
use crate::serial_println;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::slice;
use spin::Mutex;

pub static SDT_LIST: Mutex<Vec<ACPISDT>> = Mutex::new(Vec::new());

#[repr(C, packed)]
pub struct RawACPISDTHeader {
    signature: u32,
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
    first_entry: u64,
}

#[derive(Debug, Clone)]
pub struct ACPISDTHeader {
    pub signature: String,
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: String,
    pub oem_table_id: String,
    pub oem_revision: u32,
    pub creator_id: String,
    pub creator_revision: u32,
    pub raw_addr: PhysAddr,
}

pub enum ACPISDT {
    APIC(APICSDT),
    Unknown { header: ACPISDTHeader },
}

/// Safety:
/// Make sure that the pointer is valid
unsafe fn enumerate_xsdt(root_xsdt_ptr: *const RawACPISDTHeader) {
    identity_map(root_xsdt_ptr as PhysAddr, PageTableEntryFlags::PRESENT);

    let root_xsdt = unsafe { &*root_xsdt_ptr };
    let entries_count = (root_xsdt.length - size_of::<RawACPISDTHeader>() as u32) as usize / 8 + 1;
    let sdt_ptrs =
        unsafe { slice::from_raw_parts(&raw const root_xsdt.first_entry, entries_count) };

    let mut sdt_list = SDT_LIST.lock();
    for sdt_ptr in sdt_ptrs {
        identity_map(*sdt_ptr as PhysAddr, PageTableEntryFlags::PRESENT);

        let current_raw_sdt = unsafe { &*(*sdt_ptr as *const RawACPISDTHeader) };
        let current_sdt = ACPISDTHeader {
            signature: str::from_utf8(&current_raw_sdt.signature.to_le_bytes())
                .unwrap()
                .to_string(),
            length: current_raw_sdt.length,
            revision: current_raw_sdt.revision,
            checksum: current_raw_sdt.checksum,
            oem_id: str::from_utf8(&current_raw_sdt.oem_id).unwrap().to_string(),
            oem_table_id: str::from_utf8(&current_raw_sdt.oem_table_id)
                .unwrap()
                .to_string(),
            oem_revision: current_raw_sdt.oem_revision,
            creator_id: str::from_utf8(&current_raw_sdt.creator_id.to_le_bytes())
                .unwrap()
                .to_string(),
            creator_revision: current_raw_sdt.creator_revision,
            raw_addr: *sdt_ptr as PhysAddr,
        };

        match &*current_sdt.signature {
            "APIC" => {
                serial_println!("ACPI: Parsing APIC SDT: {:?}", current_sdt);
                // Safety:
                // The pointer is valid because we just parsed it.
                let acpi_sdt = unsafe { apic::parse_apic_sdt(current_sdt) };
                sdt_list.push(ACPISDT::APIC(acpi_sdt));
            }
            _ => {
                serial_println!("ACPI: skipping SDT: {:?}", current_sdt);
                sdt_list.push(ACPISDT::Unknown {
                    header: current_sdt,
                });
            }
        }
    }
}

pub fn init() {
    let boot_info = BOOT_INFO.get().unwrap();

    if let Some(rsdp_v2) = boot_info.rsdp_v2_tag() {
        serial_println!("ACPI: XSDT found");
        serial_println!("ACPI: - OEM ID: {:?}", rsdp_v2.oem_id());
        serial_println!("ACPI: - Signature: {:?}", rsdp_v2.signature());
        serial_println!("ACPI: - XSDT Address: {:x?}", rsdp_v2.xsdt_address());
        serial_println!("ACPI: - Checksum is valid: {}", rsdp_v2.checksum_is_valid());
        assert!(rsdp_v2.checksum_is_valid());

        // Safety:
        // The pointer is valid because we checked the checksum
        unsafe {
            enumerate_xsdt(rsdp_v2.xsdt_address() as *const RawACPISDTHeader);
        }
    } else {
        panic!("ACPI: XSDT not found in boot info");
    }
}
