use crate::arch::x86_64::mm::area_frame_allocator::AreaFrameAllocator;
use crate::arch::x86_64::mm::paging::entry::EntryFlags;
use crate::arch::x86_64::mm::paging::{Page, PhysicalAddress};
use crate::arch::x86_64::mm::stack_allocator::{Stack, StackAllocator};
use crate::kutils::multiboot::get_kernel_start_end;
use linked_list_allocator::LockedHeap;
use multiboot2::BootInformation;
use once::assert_has_not_been_called;

pub mod area_frame_allocator;
pub mod paging;
pub mod stack_allocator;

pub const HEAP_START: usize = 0o_000_001_000_000_0000;
pub const HEAP_SIZE: usize = 100 * 1024 * 1024; // 100 MiB
#[allow(non_upper_case_globals)]
pub const MiB: usize = 1024 * 1024;

#[global_allocator]
pub static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn display_heap_stats() {
    let alloc = HEAP_ALLOCATOR.lock();

    info!(
        "Heap size: {}MiB, Free: {}MiB",
        alloc.size() / MiB,
        alloc.free() / MiB
    );
}

pub struct MemoryController<'a> {
    active_table: paging::ActivePageTable,
    frame_allocator: AreaFrameAllocator<'a>,
    stack_allocator: StackAllocator,
}

impl<'a> MemoryController<'a> {
    pub fn alloc_stack(&mut self, size_in_pages: usize, flags: EntryFlags) -> Option<Stack> {
        self.stack_allocator.alloc_stack(
            &mut self.active_table,
            &mut self.frame_allocator,
            size_in_pages,
            flags,
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

pub const PAGE_SIZE: usize = 4096;

impl Frame {
    fn containing_address(address: usize) -> Self {
        return Self {
            number: address / PAGE_SIZE,
        };
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter { start, end }
    }

    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    fn clone(&self) -> Self {
        Self {
            number: self.number,
        }
    }
}

#[derive(Debug)]
pub struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        } else {
            None
        }
    }
}

pub fn init(boot_info: &BootInformation) -> MemoryController {
    assert_has_not_been_called!();

    let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");
    let (kernel_start, kernel_end) = get_kernel_start_end(&boot_info);

    info!(
        "kernel start: {:#x}, kernel end: {:#x}",
        kernel_start, kernel_end
    );
    info!(
        "multiboot start: {:#x}, multiboot end: {:#x}",
        boot_info.start_address(),
        boot_info.end_address()
    );

    let mut frame_allocator = AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        boot_info.start_address(),
        boot_info.end_address(),
        &memory_map_tag,
    );

    let mut active_table = paging::remap_the_kernel(&mut frame_allocator, boot_info);
    let heap_start_page = Page::containing_address(HEAP_START);
    let heap_end_page = Page::containing_address(HEAP_START + HEAP_SIZE - 1);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
    }

    let stack_allocator = {
        let stack_alloc_start = heap_end_page + 1;
        let stack_alloc_end = stack_alloc_start + 100;
        let stack_alloc_range = Page::range_inclusive(stack_alloc_start, stack_alloc_end);
        StackAllocator::new(stack_alloc_range)
    };

    MemoryController {
        active_table,
        frame_allocator,
        stack_allocator,
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
