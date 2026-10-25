ENTRY(kernel_start)

SECTIONS {
    . = 0xFFFFFFFF80000000;

    .text :
    {
        *(.text.kernel_start)
        *(.text .text.*)
        . = ALIGN(2M);
    }

    .kernel_stack : ALIGN(2M)
    {
        *(.kernel_stack)
    }

    .stackoverflow_guard : ALIGN(2M)
    {
        *(.stackoverflow_guard)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
        . = ALIGN(2M);
    }

    .data :
    {
        *(.data .data.*)
        . = ALIGN(2M);
    }

    .bss :
    {
        *(.bss .bss.*)
        . = ALIGN(2M);
    }
}