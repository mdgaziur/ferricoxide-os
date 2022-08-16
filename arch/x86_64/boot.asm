global start
extern long_mode_start

section .text
bits 32
start:
	mov esp, stack_top								; set stack pointer to our reserved 64 bytes stack
	mov edi, ebx									; move multiboot information to edi

	; do checks
	call check_multiboot
	call check_cpuid
	call check_long_mode

	; set up paging
	call set_up_page_table
	call enable_paging

	; load the 64-bit GDT
	lgdt [gdt64.pointer]
	jmp gdt64.code:long_mode_start					; jump to long mode code

check_multiboot:
	cmp eax, 0x36d76289
	jne .no_multiboot
	ret

.no_multiboot:
	mov al, "0"
	jmp error

check_cpuid:
	; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
	; in the FLAGS register. If we can flip it, CPUID is available

	; Copy FLAGS to EAX
	pushfd
	pop eax

	; Copy EAX to ECX for comparison
	mov ecx, eax

	; Flip the ID bit
	xor eax, 1 << 21

	; Copy EAX to FLAGS via the stack
	push eax
	popfd

	; Copy FLAGS back to EAX(to see if it's flipped)
	pushfd
	pop eax

	; Restore the FLAGS stored in ECX
	push ecx
	popfd

	cmp eax, ecx
	je .no_cpuid
	ret

.no_cpuid:
	mov al, "1"
	jmp error

check_long_mode:
	; test if extended processor info is available
	mov eax, 0x80000000									; implicit argument for cpuid
	cpuid												; get highest supported argument
	cmp eax, 0x80000000									; must be at least 0x80000001
	jb .no_long_mode									; cpu too old for long mode if it's less

	; use extended info to check if long mode is available
	mov eax, 0x80000001									; argument for extended info
	cpuid												; returns various feature bits in eax and edx
	test edx, 1 << 29									; test if LM-bit is set in the D-register
	jz .no_long_mode
	ret

.no_long_mode:
	mov al, "2"
	jmp error

set_up_page_table:
	; map first P4 table entry to P3 table
    mov eax, p3_table
	or eax, 0b11										; present + writable
	mov [p4_table], eax

	; map first P3 table entry to P2 table
	mov eax, p2_table
	or eax, 0b11										; present + writable
	mov [p3_table], eax

	; map each P2 entry to a huge 2MiB page
	mov ecx, 0

	; Point the 511th entry to the P4 Table itself
	mov eax, p4_table
	or eax, 0b11
	mov [p4_table + 511 * 8], eax

.map_p2_table:
	; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
	mov eax, 0x200000									; 2MiB
	mul ecx												; start address of ecx-th range
	or eax, 0b10000011									; present + writable + huge
	mov [p2_table + ecx * 8], eax						; map ecx-th entry

	inc ecx												; increase counter
	cmp ecx, 512										; if counter == 512, we've mapped whole p2 table
	jne .map_p2_table									; else map next table

	ret

enable_paging:
	; load P4 to Cr3 register
	mov eax, p4_table
	mov cr3, eax

	; enable PAE-flag in Cr4(Physical Address Extension)
	mov eax, cr4
	or eax, 1 << 5
	mov cr4, eax

	; set the long mode bit in EFER MSR(model specific register)
	mov ecx, 0xC0000080
	rdmsr
	or eax, 1 << 8
	wrmsr

	; enable paging in the Cr0 register
	mov eax, cr0
	or eax, 1 << 31
	mov cr0, eax

	ret

error:
	; Prints rudimentary "ERR: X" message where X is an error code letter
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt


section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
stack_bottom:
    resb 4096 * 4
stack_top:

section .rodata
gdt64:
    dq 0 ; zero entry
.code: equ $ - gdt64 ; new
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
