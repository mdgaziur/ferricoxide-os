arch ?= x86_64
kernel := build/codename_hammer-$(arch).bin
iso := build/codename_hammer-$(arch).iso
target := codename_hammer-$(arch)
rust_kernel := target/$(target)/debug/libkernel.a

linker_script := arch/$(arch)/linker.ld
grub_cfg := arch/$(arch)/grub/grub.cfg
assembly_source_files := $(wildcard arch/$(arch)/*.asm)
assembly_object_files := $(patsubst arch/$(arch)/%.asm, \
		build/arch/$(arch)/%.o, $(assembly_source_files))

.PHONY: all clean run iso kernel

all: $(kernel)

clean:
	@rm -rf build

run: $(iso)
	@qemu-system-x86_64 -bios /usr/share/edk2-ovmf/x64/OVMF.fd -cdrom $(iso) -d cpu_reset -serial stdio -no-reboot -no-shutdown

run_bios: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -d cpu_reset -serial stdio -no-reboot -no-shutdown

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null

$(kernel): kernel $(assembly_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) $(assembly_object_files) $(rust_kernel)

kernel:
	@RUST_TARGET_PATH=$(shell pwd) cargo build --target $(target).json

build/arch/$(arch)/%.o: arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
