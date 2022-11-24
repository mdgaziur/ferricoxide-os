global start
extern long_mode_start

section .text
bits 32
start:
    mov esp, stack_top                                     ; set stack pointer to reserved 64 bytes stack
    mov edi, ebx                                           ; move multiboot info pointer to edi
    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_paging
    call enable_paging

    lgdt [gdt64.pointer]

    jmp gdt64.code:long_mode_start
    hlt

check_multiboot:
    cmp eax, 0x36d76289                                    ; magic
    jne .no_multiboot
    ret

.no_multiboot:
    mov al, "0"
    jmp error

check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    ; in the FLAGS register. Successful flip means CPUID is available

    ; Copy FLAGS to eax
    pushfd
    pop eax

    ; Copy eax to ebx for comparison
    mov ebx, eax

    ; Flip bit 21
    xor eax, 1 << 21

    ; Push it to FLAGS register
    push eax
    popfd

    ; Copy FLAGS to eax again
    pushfd
    pop eax

    ; Push ebx to FLAGS
    push ebx
    popfd

    ; Restore FLAGS to ebx
    cmp eax, ebx
    je .no_cpuid
    ret

.no_cpuid:
    mov al, "1"
    jmp error

check_long_mode:
    ; check if extended processor info is available
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000000
    jb .no_long_mode

    ; use extended info to check whether long mode is available
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz .no_long_mode
    ret

.no_long_mode:
    mov al, "2"
    jmp error

set_up_paging:
    ; map first P4 table entry to P3 table
    mov eax, p3_table
    or eax, 0b11                                           ; present + writable
    mov [p4_table], eax

    ; map first P3 table entry to P2 table
    mov eax, p2_table
    or eax, 0b11                                           ; present + writable
    mov [p3_table], eax

    mov ecx, 0                                             ; counter
    
    ; Point 511th entry of P4 Table to itself
    mov eax, p4_table
    or eax, 0b11
    mov [p4_table + 511 * 8], eax
  
.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
    mov eax, 0x200000                                      ; 2MiB
    mul ecx                                                ; start address of ecx-th page
    or eax, 0b10000011                                     ; present + writable + huge
    mov [p2_table + ecx * 8], eax                          ; map ecx-th entry(each entry is 8 bytes large)

    inc ecx                                                ; increase counter
    cmp ecx, 512                                           ; if ecx == 512, then the whole thing is mapped
    jne .map_p2_table                                      ; else map the next entry

    ret

enable_paging:
    ; load P4 table to cr3 register
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 register
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in EFER MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

error:
	  ; Prints rudimentary "ERR: X" message where X is an error code letter
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
stack_bottom:
    resb 4096 * 4
stack_top:

section .rodata
gdt64:
    dq 0                                                   ; zero entry
.code: equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)       ; code segment
.pointer: 
    dw $ - gdt64 - 1
    dq gdt64
