; FerricOxide OS is an operating system that aims to be posix compliant and memory safe
; Copyright (C) 2023  MD Gaziur Rahman Noor
;
; This program is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License, or
; (at your option) any later version.
;
; This program is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with this program.  If not, see <https://www.gnu.org/licenses/>.
global start
extern kmain0

section .text
bits 32
start:
    mov esp, stack_top                              ; set stack pointer to reserved 64 bytes
    mov edi, ebx

    call multiboot_check
    call cpuid_check
    call long_mode_check

    call set_up_page_tables
    call enable_paging

    lgdt [gdt64.pointer]

    jmp gdt64.code:kmain0
    hlt

multiboot_check:
    cmp eax, 0x36d76289
    jne freeze
    ret

cpuid_check:
    pushfd
    pop eax

    mov ecx, eax

    xor eax, 1 << 21

    push eax
    popfd

    pushfd
    pop eax

    push ecx
    popfd

    xor eax, ecx
    jz freeze
    ret

long_mode_check:
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb freeze

    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz freeze
    ret

set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, p4_table
    or eax, 0b11
    mov [p4_table + 511 * 8], eax

    mov eax, p3_table
    or eax, 0b11
    mov [p4_table], eax

    mov eax, p2_table
    or eax, 0b11
    mov [p3_table], eax

    mov ecx, 0
.map_p2_table:
    mov eax, 0x200000
    mul ecx
    or eax, 0b10000011
    mov [p2_table + ecx * 8], eax

    inc ecx
    cmp ecx, 512
    jne .map_p2_table

    ret

enable_paging:
    mov eax, p4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

freeze:
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
    dq 0
.code: equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
