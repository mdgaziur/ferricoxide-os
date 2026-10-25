use crate::arch::process::ArchProcess;
use core::sync::atomic::{AtomicU64, Ordering};

pub static NEXT_PID: AtomicU64 = AtomicU64::new(0);

pub struct Process {
    pid: u64,
    arch_process: ArchProcess,
}

impl Process {
    pub fn new() -> Self {
        Process {
            pid: NEXT_PID.fetch_add(0, Ordering::Relaxed),
            arch_process: ArchProcess::new(),
        }
    }
}
