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

    .pml4 : ALIGN(4K)
    {
        *(.pml4)
    }

    .pdpt : ALIGN(4K)
    {
        *(.pdpt)
    }

    .pdt : ALIGN(4K)
    {
        *(.pdt)
    }

    .higher_half_pdpt : ALIGN(4K)
    {
        *(.higher_half_pdpt)
    }

    .higher_half_pdt : ALIGN(4K)
    {
        *(.higher_half_pdt)
    }

    .kernel_content : ALIGN(2M)
    {
        *(.kernel_content)
    }
}