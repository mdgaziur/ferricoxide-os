/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2025  MD Gaziur Rahman Noor
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
use crate::arch::x86_64::cpu::{flush_tlb, flush_tlb_all, read_cr3, write_cr3};
use crate::arch::x86_64::mm::frame::{Frame, FrameAllocator, FRAME_SIZE};
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::{PhysAddr, VirtAddr};
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut, Index, IndexMut};

pub mod flags;

pub const PAGE_SIZE: usize = FRAME_SIZE;
const PML4_ADDR: *mut PageTable<P4> = 0xffff_ff7f_bfdf_e000 as *mut _;

pub struct ActivePML4<'a> {
    pub mapper: Mapper<'a>,
}

impl<'a> ActivePML4<'a> {
    /// # SAFETY
    /// There must always be only one instance of this
    pub unsafe fn new() -> Self {
        Self {
            mapper: Mapper::new(),
        }
    }

    pub fn with<F>(&mut self, table: &mut InactivePML4, temporary_page: &mut TemporaryPage, f: F)
    where
        F: FnOnce(&mut Mapper),
    {
        {
            let pml4_backup = Frame::containing_address(unsafe { read_cr3() as usize });

            let p4_table = temporary_page.map_table_frame(pml4_backup, self);

            self.pml4[510].set(
                table.pml4_frame,
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
            );
            flush_tlb_all();

            f(&mut self.mapper);

            p4_table[510].set(
                pml4_backup,
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
            );
            flush_tlb_all();
        }

        temporary_page.unmap(self);
    }

    pub unsafe fn switch(&mut self, new_table: InactivePML4) -> InactivePML4 {
        let old_table = InactivePML4 {
            pml4_frame: Frame::containing_address(read_cr3() as PhysAddr),
        };

        write_cr3(new_table.pml4_frame.start_address() as u64);
        old_table
    }
}

impl<'a> Deref for ActivePML4<'a> {
    type Target = Mapper<'a>;
    fn deref(&self) -> &Self::Target {
        &self.mapper
    }
}

impl<'a> DerefMut for ActivePML4<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mapper
    }
}

pub struct Mapper<'a> {
    pub(crate) pml4: &'a mut PageTable<P4>,
}

impl<'a> Mapper<'a> {
    /// # SAFETY
    /// There must always be only one instance of this
    pub unsafe fn new() -> Self {
        Self {
            pml4: unsafe { &mut *PML4_ADDR },
        }
    }

    pub fn translate(&self, virt_addr: VirtAddr) -> Option<PhysAddr> {
        let offset = virt_addr % PAGE_SIZE;
        self.translate_page(Page::containing_address(virt_addr))
            .map(|frame| frame.number * FRAME_SIZE + offset)
    }

    fn translate_page(&self, page: Page) -> Option<Frame> {
        let pdpt = self.pml4.next_table(page.pml4_index());

        let huge_page = || {
            pdpt.and_then(|pdpt| {
                let pdpt_entry = &pdpt[page.pdpt_index()];
                // 1GiB page?
                if let Some(start_frame) = pdpt_entry.pointed_frame() {
                    if pdpt_entry.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
                        assert_eq!(start_frame.number % (512 * 512), 0);
                        return Some(Frame {
                            number: start_frame.number + page.pdt_index() * 512 + page.pt_index(),
                        });
                    }
                }

                if let Some(pdt) = pdpt.next_table(page.pdpt_index()) {
                    let pdt_entry = &pdt[page.pdt_index()];

                    // 2GiB page?
                    if let Some(start_frame) = pdt_entry.pointed_frame() {
                        if pdt_entry.flags().contains(PageTableEntryFlags::HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert_eq!(start_frame.number % 512, 0);
                            return Some(Frame {
                                number: start_frame.number + page.pt_index(),
                            });
                        }
                    }
                }

                None
            })
        };

        pdpt.and_then(|pdpt| pdpt.next_table(page.pdpt_index()))
            .and_then(|pdt| pdt.next_table(page.pdt_index()))
            .and_then(|pt| pt[page.pt_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub fn map_to(
        &mut self,
        page: Page,
        frame: Frame,
        flags: PageTableEntryFlags,
        frame_allocator: &mut impl FrameAllocator,
    ) {
        let pml4 = &mut *self.pml4;
        let pdpt = pml4.next_table_create(page.pml4_index(), frame_allocator);
        let pdt = pdpt.next_table_create(page.pdpt_index(), frame_allocator);
        let pt = pdt.next_table_create(page.pdt_index(), frame_allocator);

        assert!(pt[page.pt_index()].is_unused());
        pt[page.pt_index()].set(
            frame,
            flags | PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
        );
    }

    pub fn map(
        &mut self,
        page: Page,
        flags: PageTableEntryFlags,
        frame_allocator: &mut impl FrameAllocator,
    ) {
        let frame = frame_allocator.allocate().expect("OOM: sucks!");
        self.map_to(page, frame, flags, frame_allocator);
    }

    pub fn identity_map(
        &mut self,
        frame: Frame,
        flags: PageTableEntryFlags,
        frame_allocator: &mut impl FrameAllocator,
    ) {
        let page = Page::containing_address(frame.start_address());
        self.map_to(page, frame, flags, frame_allocator);
    }

    pub unsafe fn unmap(&mut self, page: Page, frame_allocator: &mut impl FrameAllocator) {
        assert!(self.translate(page.start_address()).is_some());

        let pt = self
            .pml4
            .next_table_mut(page.pml4_index())
            .and_then(|pdpt| pdpt.next_table_mut(page.pdpt_index()))
            .and_then(|pdt| pdt.next_table_mut(page.pdt_index()))
            .expect("TODO: huge page");

        let frame = pt[page.pt_index()].pointed_frame().unwrap();
        pt[page.pt_index()].set_unused();

        frame_allocator.deallocate(frame);
        flush_tlb(page.start_address());
    }
}

#[derive(Debug)]
pub struct InactivePML4 {
    pml4_frame: Frame,
}

impl InactivePML4 {
    pub fn new(
        frame: Frame,
        active_pml4: &mut ActivePML4,
        temporary_page: &mut TemporaryPage,
    ) -> InactivePML4 {
        {
            let table = temporary_page.map_table_frame(frame, active_pml4);
            table.zero();
            table[510].set(
                frame,
                PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
            );
        }
        temporary_page.unmap(active_pml4);

        InactivePML4 { pml4_frame: frame }
    }
}

pub struct TemporaryPage {
    page: Page,
    frame_allocator: ThreeFramesAllocator,
}

impl TemporaryPage {
    pub fn new(page: Page, allocator: &mut impl FrameAllocator) -> Self {
        Self {
            page,
            frame_allocator: ThreeFramesAllocator::new(allocator),
        }
    }

    fn map(&mut self, frame: Frame, active_pml4: &mut ActivePML4) -> VirtAddr {
        assert!(
            active_pml4.translate_page(self.page).is_none(),
            "temporary page is already mapped"
        );
        active_pml4.map_to(
            self.page,
            frame,
            PageTableEntryFlags::WRITABLE,
            &mut self.frame_allocator,
        );
        self.page.start_address()
    }

    pub fn map_table_frame(
        &mut self,
        frame: Frame,
        active_pml4: &mut ActivePML4,
    ) -> &mut PageTable<P1> {
        unsafe { &mut *(self.map(frame, active_pml4) as *mut PageTable<P1>) }
    }

    pub fn unmap(&mut self, active_table: &mut ActivePML4) {
        unsafe { active_table.unmap(self.page, &mut self.frame_allocator) }
    }
}

struct ThreeFramesAllocator([Option<Frame>; 3]);

impl ThreeFramesAllocator {
    fn new(alloctor: &mut impl FrameAllocator) -> Self {
        let mut f = || alloctor.allocate();
        Self([f(), f(), f()])
    }
}

impl FrameAllocator for ThreeFramesAllocator {
    fn allocate(&mut self) -> Option<Frame> {
        for frame_option in &mut self.0 {
            if frame_option.is_some() {
                return frame_option.take();
            }
        }

        None
    }

    unsafe fn deallocate(&mut self, frame: Frame) {
        for frame_option in &mut self.0 {
            if frame_option.is_none() {
                *frame_option = Some(frame);
                return;
            }
        }
    }
}

#[derive(Debug)]
pub struct PageTable<T: PageTableLevel> {
    entries: [PageTableEntry; 512],
    _marker: PhantomData<T>,
}

impl<T: PageTableLevel> PageTable<T> {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<T: PageTableLevel + HierarchicalLevel> PageTable<T> {
    fn next_table_addr_generic(
        &self,
        next_table_addr: usize,
        index: usize,
    ) -> Option<*mut PageTable<T::NextLevel>>
    where
        <T as HierarchicalLevel>::NextLevel: PageTableLevel,
    {
        let entry_flags = self[index].flags();
        if entry_flags.contains(PageTableEntryFlags::PRESENT)
            && !entry_flags.contains(PageTableEntryFlags::HUGE_PAGE)
        {
            Some(next_table_addr as *mut _)
        } else {
            None
        }
    }

    fn next_table_create_inner(&mut self, index: usize, frame_allocator: &mut impl FrameAllocator) {
        let frame = frame_allocator.allocate().expect("OOM: sucks!");
        self.entries[index].set(
            frame,
            PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE,
        );
    }
}

impl PageTable<P4> {
    pub fn next_table_addr(&self, index: usize) -> Option<*mut PageTable<P3>> {
        let index_mask = index << 12;
        self.next_table_addr_generic(0o1_777_777_767_767_760_000_000 | index_mask, index)
    }

    pub fn next_table(&self, index: usize) -> Option<&PageTable<P3>> {
        self.next_table_addr(index).map(|addr| unsafe { &*addr })
    }

    pub fn next_table_mut(&self, index: usize) -> Option<&mut PageTable<P3>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &mut *addr })
    }

    pub fn next_table_create(
        &mut self,
        index: usize,
        frame_allocator: &mut impl FrameAllocator,
    ) -> &mut PageTable<P3> {
        if self.next_table(index).is_none() {
            assert!(
                !self.entries[index]
                    .flags()
                    .contains(PageTableEntryFlags::PRESENT),
                "TODO: mapping huge page"
            );
            self.next_table_create_inner(index, frame_allocator);
            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap()
    }
}

impl PageTable<P3> {
    pub fn next_table_addr(&self, index: usize) -> Option<*mut PageTable<P2>> {
        // preserve the index of pdpt
        let previous_index_mask = (self as *const _ as usize & 0o777 << 12) >> 12;
        let index_mask = previous_index_mask << 9 | index;
        self.next_table_addr_generic(0o1_777_777_767_760_000_000_000 | index_mask << 12, index)
    }

    pub fn next_table(&self, index: usize) -> Option<&PageTable<P2>> {
        self.next_table_addr(index).map(|addr| unsafe { &*addr })
    }

    pub fn next_table_mut(&self, index: usize) -> Option<&mut PageTable<P2>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &mut *addr })
    }

    pub fn next_table_create(
        &mut self,
        index: usize,
        frame_allocator: &mut impl FrameAllocator,
    ) -> &mut PageTable<P2> {
        if self.next_table(index).is_none() {
            assert!(
                !self.entries[index]
                    .flags()
                    .contains(PageTableEntryFlags::PRESENT),
                "TODO: mapping huge page"
            );
            self.next_table_create_inner(index, frame_allocator);
            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap()
    }
}

impl PageTable<P2> {
    pub fn next_table_addr(&self, index: usize) -> Option<*mut PageTable<P1>> {
        // preserve the index of pdpt and pdt
        let previous_index_mask = (self as *const _ as usize & 0o777_777 << 12) >> 12;
        let index_mask = previous_index_mask << 9 | index;
        self.next_table_addr_generic(0o1_777_777_760_000_000_000_000 | index_mask << 12, index)
    }

    pub fn next_table(&self, index: usize) -> Option<&PageTable<P1>> {
        self.next_table_addr(index).map(|addr| unsafe { &*addr })
    }

    pub fn next_table_mut(&self, index: usize) -> Option<&mut PageTable<P1>> {
        self.next_table_addr(index)
            .map(|addr| unsafe { &mut *addr })
    }

    pub fn next_table_create(
        &mut self,
        index: usize,
        frame_allocator: &mut impl FrameAllocator,
    ) -> &mut PageTable<P1> {
        if self.next_table(index).is_none() {
            assert!(
                !self.entries[index]
                    .flags()
                    .contains(PageTableEntryFlags::PRESENT),
                "TODO: mapping huge page"
            );
            self.next_table_create_inner(index, frame_allocator);
            self.next_table_mut(index).unwrap().zero();
        }

        self.next_table_mut(index).unwrap()
    }
}

impl<T: PageTableLevel> Index<usize> for PageTable<T> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<T: PageTableLevel> IndexMut<usize> for PageTable<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub trait PageTableLevel {}
pub trait HierarchicalLevel {
    type NextLevel;
}

#[derive(Debug)]
pub enum P4 {}

#[derive(Debug)]
pub enum P3 {}

#[derive(Debug)]
pub enum P2 {}

#[derive(Debug)]
pub enum P1 {}

impl PageTableLevel for P4 {}
impl PageTableLevel for P3 {}
impl PageTableLevel for P2 {}
impl PageTableLevel for P1 {}

impl HierarchicalLevel for P4 {
    type NextLevel = P3;
}
impl HierarchicalLevel for P3 {
    type NextLevel = P2;
}
impl HierarchicalLevel for P2 {
    type NextLevel = P1;
}

#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(PageTableEntryFlags::PRESENT) {
            Some(Frame::containing_address(
                self.0 as usize & 0x000fffff_fffff000,
            ))
        } else {
            None
        }
    }

    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn set(&mut self, frame: Frame, flags: PageTableEntryFlags) {
        assert_eq!(frame.start_address() & !0x000fffff_fffff000, 0);
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "PageTableEntry(0x{:x})", self.0)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Page {
    pub(crate) number: usize,
}

impl Page {
    pub fn containing_address(addr: usize) -> Self {
        assert!(
            addr <= 0x0000_8000_0000_0000 || addr >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}",
            addr
        );

        Self {
            number: addr / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn pml4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn pdpt_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn pdt_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn pt_index(&self) -> usize {
        self.number & 0o777
    }
}
