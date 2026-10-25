use crate::arch::x86_64::mm::frame::{FRAME_ALLOCATOR, FrameAllocator};
use crate::arch::x86_64::mm::paging::{InactivePML4, Page, TemporaryPage};
use crate::arch::x86_64::mm::{ACTIVE_PML4, KERNEL_PAGE_ALLOCATOR};

pub struct ArchProcess {
    address_space: InactivePML4,
}

impl ArchProcess {
    pub fn new() -> Self {
        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        let mut active_pml4 = ACTIVE_PML4.get().unwrap().lock();

        let mut temporary_page = TemporaryPage::new(
            Page::containing_address(
                KERNEL_PAGE_ALLOCATOR
                    .get()
                    .unwrap()
                    .lock()
                    .allocate_page()
                    .unwrap(),
            ),
            &mut *frame_allocator,
        );

        let inactive_pml4 = {
            let frame = frame_allocator
                .allocate()
                .expect("OOM: could not allocate frame for new process page table");
            InactivePML4::new(frame, &mut active_pml4, &mut temporary_page)
        };

        Self {
            address_space: inactive_pml4,
        }
    }
}
