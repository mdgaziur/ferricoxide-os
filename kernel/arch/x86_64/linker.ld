ENTRY(start)

SECTIONS {
    . = 0xFFFFFFFF80000000;

    .text :
    {
        *(.text.kernel_start)
        *(.text .text.*)
        . = ALIGN(4K);
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