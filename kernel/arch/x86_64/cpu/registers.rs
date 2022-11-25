use core::arch::asm;
use x86_64::registers::read_rip;

#[derive(Default)]
pub struct Registers {
    rip: u64,
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    rsp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

impl Registers {
    pub fn read_regs() -> Self {
        let rip = read_rip().as_u64();
        let rax;
        let rbx;
        let rcx;
        let rdx;
        let rsi;
        let rdi;
        let rbp;
        let rsp;
        let r8;
        let r9;
        let r10;
        let r11;
        let r12;
        let r13;
        let r14;
        let r15;

        unsafe {
            asm!("mov {}, rax", out(reg) rax, options(nostack));
            asm!("mov {}, rbx", out(reg) rbx, options(nostack));
            asm!("mov {}, rcx", out(reg) rcx, options(nostack));
            asm!("mov {}, rdx", out(reg) rdx, options(nostack));
            asm!("mov {}, rsi", out(reg) rsi, options(nostack));
            asm!("mov {}, rdi", out(reg) rdi, options(nostack));
            asm!("mov {}, rbp", out(reg) rbp, options(nostack));
            asm!("mov {}, rsp", out(reg) rsp, options(nostack));
            asm!("mov {}, r8", out(reg) r8, options(nostack));
            asm!("mov {}, r9", out(reg) r9, options(nostack));
            asm!("mov {}, r10", out(reg) r10, options(nostack));
            asm!("mov {}, r11", out(reg) r11, options(nostack));
            asm!("mov {}, r12", out(reg) r12, options(nostack));
            asm!("mov {}, r13", out(reg) r13, options(nostack));
            asm!("mov {}, r14", out(reg) r14, options(nostack));
            asm!("mov {}, r15", out(reg) r15, options(nostack));
        }

        Self {
            rip,
            rax,
            rbx,
            rcx,
            rdx,
            rsi,
            rdi,
            rbp,
            rsp,
            r8,
            r9,
            r10,
            r11,
            r12,
            r13,
            r14,
            r15,
        }
    }

    pub fn dump_regs(&self) {
        print_raw!("Registers");
        print_raw!("Instruction pointer=0x{:x}\n", self.rip);
        print_raw!(
            "rax=0x{:x}, rbx=0x{:x}, rcx=0x{:x}, rdx=0x{:x},\n",
            self.rax,
            self.rbx,
            self.rcx,
            self.rdx
        );
        print_raw!(
            "rsi=0x{:x}, rdi=0x{:x}, rbp=0x{:x}, rsp=0x{:x},\n",
            self.rsi,
            self.rdi,
            self.rbp,
            self.rsp
        );
        print_raw!(
            "r8=0x{:x}, r9=0x{:x}, r10=0x{:x}, r11=0x{:x},\n",
            self.r8,
            self.r9,
            self.r10,
            self.r11
        );
        print_raw!(
            "r12=0x{:x}, r13=0x{:x}, r14=0x{:x}, r15=0x{:x}\n",
            self.r12,
            self.r13,
            self.r14,
            self.r15
        );
    }
}
