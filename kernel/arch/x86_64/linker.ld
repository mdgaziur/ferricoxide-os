ENTRY(start)

SECTIONS {
    . = 0xFFFFFFFF80000000;

    .text :
    {
        *(.text.kernel_start)
        *(.text .text.*)
        . = ALIGN(4K);
    }

    .kernel_stack : ALIGN(4K)
    {
        *(.kernel_stack)
    }

    .stackoverflow_guard : ALIGN(4K)
    {
        *(.stackoverflow_guard)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
        . = ALIGN(4K);
    }

    .data :
    {
        *(.data .data.*)
        . = ALIGN(4K);
    }

    .bss :
    {
        *(.bss .bss.*)
        . = ALIGN(4K);
    }
}