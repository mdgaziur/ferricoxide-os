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
section .multiboot_header
header_start:
    dd 0xe85250d6                   ; magic number
    dd 0                            ; architecture 0 (protected mode i386)
    dd header_end - header_start    ; header_length
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; Framebuffer stuff
    dw 5                                                                 ; Framebuffer Tag
    dw 0                                                                 ; Flags
    dd 20                                                                ; Size
    dd 0                                                                 ; Height
    dd 0                                                                 ; Width
    dd 0                                                                 ; Depth

    dd 0                            ; type
    dd 0                            ; flags
    dd 8                            ; size
header_end: