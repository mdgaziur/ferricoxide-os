/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2023  MD Gaziur Rahman Noor
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::arch::mm::frame_alloc::{Frame, FrameAllocator};
use core::arch::asm;
use core::fmt::Debug;

use crate::arch::mm::paging::{PageTableEntryFlags, TemporaryPage, PAGE_SIZE};
use crate::arch::mm::{PhysicalAddress, VirtualAddress};
use crate::arch::x86_64::mm::paging::{Page, PageTableEntry, PAGE_COUNT};

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut, Index, IndexMut};
use x86_64::instructions::tlb;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::PhysFrame;
use x86_64::PhysAddr;

pub const PML4_ADDR: *mut PageTable<PML4> = 0xffff_ffff_ffff_f000 as *mut _;

#[derive(Debug)]
pub struct Mapper<'a> {
    pml4: &'a mut PageTable<PML4>,
}

impl<'a> Mapper<'a> {
    /// ## SAFETY:
    ///
    /// [PML4_ADDR] must be a valid virtual address pointing to the PML4 table
    pub unsafe fn new() -> Self {
        Self {
            pml4: unsafe { &mut *PML4_ADDR },
        }
    }

    pub fn pml4(&self) -> &PageTable<PML4> {
        self.pml4
    }

    pub fn pml4_mut(&mut self) -> &mut PageTable<PML4> {
        self.pml4
    }

    pub fn translate(&self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virt_addr.0 % PAGE_SIZE;
        self.translate_page(Page::containing_address(virt_addr))
            .map(|frame| PhysicalAddress(frame.start_address().0 + offset))
    }

    fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.pml4().next_table(page.pml4_index());

        let huge_page = || {
            p3.and_then(|pdpt| {
                let pdpt_entry = &pdpt[page.pdpt_index()];

                // 1GiB page?
                if let Some(start_frame) = pdpt_entry.pointed_frame() {
                    if pdpt_entry.flags().contains(PageTableEntryFlags::PS) {
                        // 1 GiB aligned
                        assert_eq!(start_frame.0 % (PAGE_COUNT * PAGE_COUNT), 0);
                        return Some(Frame(
                            start_frame.0 + page.pdt_index() * PAGE_COUNT + page.pt_index(),
                        ));
                    }
                }

                if let Some(pdt) = pdpt.next_table(page.pdpt_index()) {
                    let pdt_entry = &pdt[page.pdt_index()];
                    // 2MiB page?
                    if let Some(start_frame) = pdt_entry.pointed_frame() {
                        if pdt_entry.flags().contains(PageTableEntryFlags::PS) {
                            // 2 MiB aligned
                            assert_eq!(start_frame.0 % (PAGE_COUNT * PAGE_COUNT), 0);
                            return Some(Frame(start_frame.0 + page.pt_index()));
                        }
                    }
                }

                None
            })
        };

        p3.and_then(|pdpt| pdpt.next_table(page.pdpt_index()))
            .and_then(|pdt| pdt.next_table(page.pdt_index()))
            .and_then(|pt| pt[page.pt_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub fn map_to(
        &mut self,
        page: Page,
        frame: Frame,
        flags: PageTableEntryFlags,
        frame_allocator: &mut FrameAllocator,
    ) {
        let pml4 = self.pml4_mut();
        let pdpt = pml4.next_table_create(page.pml4_index(), frame_allocator);
        let pdt = pdpt.next_table_create(page.pdpt_index(), frame_allocator);
        let pt = pdt.next_table_create(page.pdt_index(), frame_allocator);

        assert!(pt[page.pt_index()].is_unused());
        pt[page.pt_index()].set(frame, flags | PageTableEntryFlags::PRESENT);
    }

    pub fn map(
        &mut self,
        page: Page,
        flags: PageTableEntryFlags,
        frame_allocator: &mut FrameAllocator,
    ) {
        let frame = frame_allocator
            .alloc(1)
            .expect("OOM: sucks dude! (maybe implement swap?)");
        self.map_to(page, frame, flags, frame_allocator);
    }

    pub fn identity_map(
        &mut self,
        frame: Frame,
        flags: PageTableEntryFlags,
        frame_allocator: &mut FrameAllocator,
    ) {
        self.map_to(
            Page::containing_address(VirtualAddress(frame.start_address().0)),
            frame,
            flags,
            frame_allocator,
        );
    }

    /// ## SAFETY
    ///
    /// Caller must ensure that given page is not used by anyone else
    pub unsafe fn unmap_without_freeing_frame(&mut self, page: Page) {
        assert!(self.translate(page.start_address()).is_some());

        let pt = self
            .pml4_mut()
            .next_table_mut(page.pml4_index())
            .and_then(|pdpt| pdpt.next_table_mut(page.pdpt_index()))
            .and_then(|pdt| pdt.next_table_mut(page.pdt_index()))
            .expect("huge pages aren't supported, duh");

        pt[page.pt_index()].set_unused();
        asm!("invlpg [{}]", in(reg) page.start_address().0, options(nostack, preserves_flags));
    }

    /// ## SAFETY
    ///
    /// Caller must ensure that given page is not used by anyone else
    pub unsafe fn unmap(&mut self, page: Page, frame_allocator: &mut FrameAllocator) {
        assert!(self.translate(page.start_address()).is_some());

        let pt = self
            .pml4_mut()
            .next_table_mut(page.pml4_index())
            .and_then(|pdpt| pdpt.next_table_mut(page.pdpt_index()))
            .and_then(|pdt| pdt.next_table_mut(page.pdt_index()))
            .expect("huge pages aren't supported, duh");
        let frame = pt[page.pt_index()].pointed_frame().unwrap();

        pt[page.pt_index()].set_unused();
        tlb::flush(x86_64::VirtAddr::new(page.start_address().0 as u64));
        frame_allocator.free(frame, 1);
    }
}

pub struct ActivePageTable<'a> {
    mapper: Mapper<'a>,
}

impl<'a> ActivePageTable<'a> {
    pub(crate) unsafe fn new() -> Self {
        Self {
            mapper: Mapper::new(),
        }
    }

    pub fn with<F>(
        &mut self,
        table: &InactivePageTable,
        temporary_page: &mut TemporaryPage,
        frame_allocator: &mut FrameAllocator,
        f: F,
    ) where
        F: FnOnce(&mut Mapper, &mut FrameAllocator),
    {
        {
            let backup = Frame::containing_address(PhysicalAddress(
                Cr3::read().0.start_address().as_u64() as usize,
            ));

            let pml4_table = temporary_page.map_table_frame(backup.clone(), self, frame_allocator);

            self.pml4_mut()[511].set(
                table.pml4_frame.clone(),
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::RW,
            );
            tlb::flush_all();
            f(&mut self.mapper, frame_allocator);

            pml4_table[511].set(
                backup,
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::RW,
            );
            tlb::flush_all();
        }

        unsafe {
            temporary_page.unmap(self);
        }
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        let cr3_flags;
        let old_table = InactivePageTable {
            pml4_frame: Frame::containing_address(PhysicalAddress({
                let (old_table, flags) = Cr3::read();
                cr3_flags = flags;

                old_table.start_address().as_u64() as usize
            })),
        };

        unsafe {
            Cr3::write(
                PhysFrame::containing_address(PhysAddr::new(
                    new_table.pml4_frame.start_address().0 as u64,
                )),
                cr3_flags,
            );
        }
        old_table
    }
}

impl<'a> Deref for ActivePageTable<'a> {
    type Target = Mapper<'a>;

    fn deref(&self) -> &Self::Target {
        &self.mapper
    }
}

impl<'a> DerefMut for ActivePageTable<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mapper
    }
}

#[derive(Debug)]
pub struct PageTable<L> {
    pub entries: [PageTableEntry; PAGE_COUNT],
    pub _level: PhantomData<L>,
}

impl<PT: TableLevel> PageTable<PT> {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L: TableHierarchy> PageTable<L> {
    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(PageTableEntryFlags::PRESENT)
            && !entry_flags.contains(PageTableEntryFlags::PS)
        {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    pub fn next_table(&self, index: usize) -> Option<&PageTable<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe { &*(addr as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut PageTable<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe { &mut *(addr as *mut _) })
    }

    pub fn next_table_create(
        &mut self,
        index: usize,
        allocator: &mut FrameAllocator,
    ) -> &mut PageTable<L::NextLevel> {
        if self.next_table(index).is_none() {
            assert!(!self[index].flags().contains(PageTableEntryFlags::PS));
            let frame = allocator.alloc(1).expect("no frames available");
            self[index].set(
                frame,
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::RW,
            );
            self.next_table_mut(index).unwrap().zero();
        }
        self.next_table_mut(index).unwrap()
    }
}

impl<L> Index<usize> for PageTable<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for PageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub trait TableLevel: Debug {}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum PML4 {}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum PDPT {}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum PDT {}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum PT {}

impl TableLevel for PML4 {}
impl TableLevel for PDPT {}
impl TableLevel for PDT {}
impl TableLevel for PT {}

pub trait TableHierarchy: TableLevel {
    type NextLevel: TableLevel;
}

impl TableHierarchy for PML4 {
    type NextLevel = PDPT;
}

impl TableHierarchy for PDPT {
    type NextLevel = PDT;
}

impl TableHierarchy for PDT {
    type NextLevel = PT;
}

#[derive(Debug)]
pub struct InactivePageTable {
    pub(crate) pml4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        temporary_page: &mut TemporaryPage,
        frame: Frame,
        active_table: &mut ActivePageTable,
        frame_allocator: &mut FrameAllocator,
    ) -> Self {
        {
            let table =
                temporary_page.map_table_frame(frame.clone(), active_table, frame_allocator);
            table.zero();
            table[511].set(
                frame.clone(),
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::RW,
            );
        }
        unsafe {
            temporary_page.unmap(active_table);
        }
        Self {
            pml4_frame: frame.clone(),
        }
    }
}
