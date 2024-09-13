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

section .multiboot_header
header_start:
    dd 0xe85250d6                                                        ; magic number
    dd 0                                                                 ; protected mode(x86)
    dd header_end - header_start                                         ; header_length
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))      ; checksum

    ; Framebuffer stuff
    dw 5                                                                 ; Framebuffer Tag
    dw 0                                                                 ; Flags
    dd 20                                                                ; Size
    dd 1024                                                              ; Width
    dd 768                                                               ; Height
    dd 32                                                                ; Depth

    ; end tag
    dd 0
    dd 0
    dd 8
header_end:

section .text
start:
    mov esp, stack_top
    push ebx                ; pass multiboot information pointer as the first argument
                            ; pushing it to the stack because `prekernel_main` follows the "cdecl" ABI

    extern prekernel_main
    call prekernel_main

    ; halt the processor if we somehow end up here
    cli
    hlt

section .bss
align 4096
stack_bottom:
    resb 1024 * 1024 * 10
stack_top:
