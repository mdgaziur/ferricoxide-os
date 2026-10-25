ENTRY(start)

SECTIONS {
    . = 1M;

    .rodata :
    {
        /* ensure that the multiboot header is at the beginning */
        KEEP(*(.multiboot_header))
        *(.rodata .rodata.*)
        . = ALIGN(4K);
    }

    .text :
    {
        *(.text .text.*)
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

    .pml4 (NOLOAD) : ALIGN(4K)
    {
        *(.pml4)
    }

    .pdpt (NOLOAD) : ALIGN(4K)
    {
        *(.pdpt)
    }

    .pdt (NOLOAD) : ALIGN(4K)
    {
        *(.pdt)
    }

    .higher_half_pdpt (NOLOAD) : ALIGN(4K)
    {
        *(.higher_half_pdpt)
    }

    .higher_half_pdt (NOLOAD) : ALIGN(4K)
    {
        *(.higher_half_pdt)
    }

    .kernel_content (NOLOAD) : ALIGN(2M)
    {
        *(.kernel_content)
    }
}