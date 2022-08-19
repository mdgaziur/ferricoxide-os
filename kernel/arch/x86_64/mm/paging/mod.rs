use crate::arch::x86_64::mm::paging::entry::EntryFlags;
use crate::arch::x86_64::mm::paging::mapper::Mapper;
use crate::arch::x86_64::mm::paging::temporary_page::TemporaryPage;
use crate::arch::x86_64::mm::FrameAllocator;
use crate::arch::x86_64::mm::{Frame, PAGE_SIZE};

use core::ops::{Add, Deref, DerefMut};
pub use entry::*;
use multiboot2::BootInformation;
use x86_64::PhysAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::PhysFrame;
use utils::multiboot::get_multiboot_info_start_end;

pub mod entry;
mod mapper;
mod table;
mod temporary_page;

const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}",
            address
        );

        Page {
            number: address / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter { start, end }
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Self::Output {
        Page {
            number: self.number + rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }
}
pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn with<F>(
        &mut self,
        table: &mut InactivePageTable,
        temporary_page: &mut TemporaryPage,
        f: F,
    ) where
        F: FnOnce(&mut Mapper),
    {
        {
            let backup = Frame::containing_address(Cr3::read().0.start_address().as_u64() as usize);

            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            self.p4_mut()[511].set(
                table.p4_frame.clone(),
                EntryFlags::PRESENT | EntryFlags::WRITABLE,
            );
            x86_64::instructions::tlb::flush_all();

            f(self);

            p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            x86_64::instructions::tlb::flush_all();
        }

        temporary_page.unmap(self);
    }

    pub fn switch(&mut self, new_table: &mut InactivePageTable) -> InactivePageTable {
        let old_vals = Cr3::read();
        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(old_vals.0.start_address().as_u64() as usize),
        };

        unsafe {
            Cr3::write(
                PhysFrame::containing_address(PhysAddr::new(new_table.p4_frame.start_address() as u64)),
                old_vals.1,
            );
        }

        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        frame: Frame,
        active_page_table: &mut ActivePageTable,
        temporary_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        {
            let table = temporary_page.map_table_frame(frame.clone(), active_page_table);

            table.zero();
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }
        temporary_page.unmap(active_page_table);

        InactivePageTable { p4_frame: frame }
    }
}

pub fn remap_the_kernel<A>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable
where
    A: FrameAllocator,
{
    let mut temporary_page = TemporaryPage::new(Page { number: 0xdeadbeef }, allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("failed to get elf sections tag");

        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                continue;
            }
            assert_eq!(section.start_address() as usize % PAGE_SIZE, 0);

            info!(
                "mapping section at addr: {:#x}, size: {:#x}",
                section.start_address(),
                section.size()
            );

            let flags = EntryFlags::from_elf_section_flags(&section);

            let start_frame = Frame::containing_address(section.start_address() as usize);
            let end_frame = Frame::containing_address(section.end_address() as usize - 1);
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, flags, allocator);
            }
        }

        let vga_framebuffer_tag = boot_info.framebuffer_tag().unwrap();
        let vga_framebuffer_end = (vga_framebuffer_tag.address
            + (vga_framebuffer_tag.height * vga_framebuffer_tag.pitch as u32) as u64)
            as usize;
        info!(
            "{:?}",
            Frame::range_inclusive(
                Frame::containing_address(vga_framebuffer_tag.address as usize),
                Frame::containing_address(vga_framebuffer_end),
            )
        );
        for frame in Frame::range_inclusive(
            Frame::containing_address(vga_framebuffer_tag.address as usize),
            Frame::containing_address(vga_framebuffer_end),
        ) {
            mapper.identity_map(frame, EntryFlags::WRITABLE, allocator);
        }

        let (multiboot_start, multiboot_end) = get_multiboot_info_start_end(&boot_info);
        for frame in Frame::range_inclusive(
            Frame::containing_address(multiboot_start),
            Frame::containing_address(multiboot_end),
        ) {
            mapper.identity_map(frame, EntryFlags::PRESENT, allocator);
        }
    });

    let old_table = active_table.switch(&mut new_table);
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);
    info!("guard page at {:#x}", old_p4_page.start_address());

    active_table
}
