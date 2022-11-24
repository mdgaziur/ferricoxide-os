section .multiboot_header
header_start:
    dd 0xe85250d6                                                        ; magic number
    dd 0                                                                 ; protected mode(i386)
    dd header_end - header_start                                         ; header_length
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))      ; checksum
    
    ; Framebuffer stuff
    dw 5                                                                 ; Framebuffer Tag
    dw 0                                                                 ; Flags
    dd 20                                                                ; Size
    dd 0                                                                 ; Height
    dd 0                                                                 ; Width
    dd 0                                                                 ; Depth
    
    ; end tag
    dd 0
    dd 0
    dd 8
header_end:
