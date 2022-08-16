pub mod cr3;
pub mod instructions;
pub mod msr;
pub mod tlb;
pub mod utils;

pub fn initial_setup_x86_64() {
    utils::enable_nxe_bit();
    info!("Enabled nxe bit");

    utils::enable_write_protect_bit();
    info!("Enabled write protection bit");
}
