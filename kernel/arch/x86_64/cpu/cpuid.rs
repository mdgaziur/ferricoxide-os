use crate::serial_println;
use bitflags::bitflags;
use core::arch::asm;

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct CPUIDECXFeature: u32 {
        const SSE3 = 1 << 0;
        const PCLMUL = 1 << 1;
        const DTES64 = 1 << 2;
        const MONITOR = 1 << 3;
        const DS_CPL = 1 << 4;
        const VMX = 1 << 5;
        const SMX = 1 << 6;
        const EST = 1 << 7;
        const TM2 = 1 << 8;
        const SSSE3 = 1 << 9;
        const CID = 1 << 10;
        const SDBG = 1 << 11;
        const FMA = 1 << 12;
        const CX16 = 1 << 13;
        const XTPR = 1 << 14;
        const PDCM = 1 << 15;
        const PCID = 1 << 17;
        const DCA = 1 << 18;
        const SSE4_1 = 1 << 19;
        const SSE4_2 = 1 << 20;
        const X2APIC = 1 << 21;
        const MOVBE = 1 << 22;
        const POPCNT = 1 << 23;
        const TSC = 1 << 24;
        const AES = 1 << 25;
        const XSAVE = 1 << 26;
        const OSXSAVE = 1 << 27;
        const AVX = 1 << 28;
        const F16C = 1 << 29;
        const RDRAND = 1 << 30;
        const HYPERVISOR = 1 << 31;
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct CPUIDEDXFeature: u32 {
        const FPU          = 1 << 0;
        const VME          = 1 << 1;
        const DE           = 1 << 2;
        const PSE          = 1 << 3;
        const TSC          = 1 << 4;
        const MSR          = 1 << 5;
        const PAE          = 1 << 6;
        const MCE          = 1 << 7;
        const CX8          = 1 << 8;
        const APIC         = 1 << 9;
        const SEP          = 1 << 11;
        const MTRR         = 1 << 12;
        const PGE          = 1 << 13;
        const MCA          = 1 << 14;
        const CMOV         = 1 << 15;
        const PAT          = 1 << 16;
        const PSE36        = 1 << 17;
        const PSN          = 1 << 18;
        const CLFLUSH      = 1 << 19;
        const DS           = 1 << 21;
        const ACPI         = 1 << 22;
        const MMX          = 1 << 23;
        const FXSR         = 1 << 24;
        const SSE          = 1 << 25;
        const SSE2         = 1 << 26;
        const SS           = 1 << 27;
        const HTT          = 1 << 28;
        const TM           = 1 << 29;
        const IA64         = 1 << 30;
        const PBE          = 1 << 31;
    }
}

pub fn cpuid_get_vendor(vendor: &mut [u8; 13]) {
    let ebx: u32;
    let edx: u32;
    let ecx: u32;

    unsafe {
        asm!(
            "
                push rbx
                push rax
                mov eax, 0
                cpuid
                pop rax
                mov {:e}, ebx
                pop rbx
            ",
            out(reg) ebx,
            out("edx") edx,
            out("ecx") ecx,
        );
    };

    serial_println!("CPUID EBX: {:x?}", ebx);
    vendor[0] = (ebx & 0xFF) as u8;
    vendor[1] = ((ebx >> 8) & 0xFF) as u8;
    vendor[2] = ((ebx >> 16) & 0xFF) as u8;
    vendor[3] = ((ebx >> 24) & 0xFF) as u8;
    vendor[4] = (edx & 0xFF) as u8;
    vendor[5] = ((edx >> 8) & 0xFF) as u8;
    vendor[6] = ((edx >> 16) & 0xFF) as u8;
    vendor[7] = ((edx >> 24) & 0xFF) as u8;
    vendor[8] = (ecx & 0xFF) as u8;
    vendor[9] = ((ecx >> 8) & 0xFF) as u8;
    vendor[10] = ((ecx >> 16) & 0xFF) as u8;
    vendor[11] = ((ecx >> 24) & 0xFF) as u8;
    vendor[12] = 0;
}

pub fn cpuid_getfeatures() -> (CPUIDECXFeature, CPUIDEDXFeature) {
    let ecx: u32;
    let edx: u32;

    unsafe {
        asm!(
            "
                push rax
                mov eax, 1
                cpuid
                pop rax
                mov {:e}, ecx
                mov {:e}, edx
            ",
            out(reg) ecx,
            out(reg) edx,
        );
    }

    (
        CPUIDECXFeature::from_bits(ecx).unwrap(),
        CPUIDEDXFeature::from_bits(edx).unwrap(),
    )
}
