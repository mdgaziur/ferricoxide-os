section .multiboot_header
header_start:
	dd 0xE85250D6													; magic
	dd 0															; i386 arch
	dd header_end - header_start									; header length
	dd -(0xE85250D6 + 0 + header_end - header_start)				; checksum

	; tags

	; Framebuffer tag(https://github.com/LemonOSProject/LemonOS/blob/master/Kernel/src/Arch/x86_64/Entry.asm#L113)
	dw 5															; Type: Framebuffer
	dw 0															; Flags
	dd 20															; size
	dd 1280															; Width
	dd 700															; Height
	dd 32															; Depth

	align 8

	; End tag
	dw 0															; type
	dw 0															; flags
	dd 8															; size

header_end: