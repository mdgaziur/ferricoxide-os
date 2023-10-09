global kmain0

section .text
bits 64
kmain0:
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    extern kmain1
    call kmain1

    hlt