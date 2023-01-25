pub const STACK_SIZE: usize = 10 * MB;

pub struct ArchProcess {
    pub ctx: Context,
    pub stack: [u8; STACK_SIZE],
    pub stack_pointer: u64,
    pub xmm: [u8; 512],
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(packed)]
pub struct Context {
    pub rbx: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

#[naked]
pub unsafe extern "C" fn switch_context(
    new_context: &Context,
    new_cr3: usize,
    rip: usize,
) {
    asm!(
        "\
        mov rax, rsp
        mov rsp, rdi
        pop rbx
        pop rbp
        pop r12
        pop r13
        pop r14
        pop r15
        mov rsp, rax
        mov cr3, rsi
        push rdx
        ret
    ",
        options(noreturn)
    )
}
