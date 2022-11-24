use multiboot2::BootInformation;

pub fn load_multiboot_info(addr: usize) -> BootInformation {
    unsafe { multiboot2::load(addr) }.expect("Failed to load multiboot information")
}

pub fn get_kernel_start_end(boot_info: &BootInformation) -> (usize, usize) {
    let elf_sections_tag = boot_info
        .elf_sections_tag()
        .expect("Failed to get elf sections tag");

    let kernel_start = elf_sections_tag
        .sections()
        .map(|s| s.start_address())
        .min()
        .unwrap();

    let kernel_end = elf_sections_tag
        .sections()
        .map(|s| s.start_address() + s.size())
        .max()
        .unwrap();

    (kernel_start as usize, kernel_end as usize)
}

pub fn get_multiboot_info_start_end(boot_info: &BootInformation) -> (usize, usize) {
    let multiboot_start = boot_info.start_address();
    let multiboot_end = boot_info.start_address() + boot_info.total_size() - 1;

    (multiboot_start, multiboot_end)
}
