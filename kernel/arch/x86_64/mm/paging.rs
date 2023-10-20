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
use crate::arch::mm::frame_alloc::{Frame, FrameAllocator, FrameIter};
use crate::arch::mm::page_table::{ActivePageTable, InactivePageTable, PageTable, PT};
use crate::arch::mm::{KernelMM, VirtualAddress, KERNEL_HEAP_SIZE, KERNEL_HEAP_START};
use crate::serial_println;
use crate::units::KiB;
use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use multiboot2::{BootInformation, TagTrait};

pub const PAGE_SIZE: usize = 4 * KiB;

pub const PAGE_COUNT: usize = 512;

pub fn remap_kernel(kmm: &mut KernelMM, boot_info: &BootInformation) {
    let mut temporary_page = TemporaryPage::new(Page { number: 0xdeadbeef });

    let mut active_table = unsafe { ActivePageTable::new() };
    let new_table = {
        let frame = kmm.frame_allocator.alloc(1).expect("OOM: sucks dude");
        InactivePageTable::new(
            &mut temporary_page,
            frame,
            &mut active_table,
            &mut kmm.frame_allocator,
        )
    };

    active_table.with(
        &new_table,
        &mut temporary_page,
        &mut kmm.frame_allocator,
        |mapper, frame_allocator| {
            let elf_sections = boot_info.elf_sections().unwrap();

            serial_println!("Mapping Kernel ELF Sections");
            for section in elf_sections {
                if !section.is_allocated() {
                    continue;
                }

                assert_eq!(
                    section.start_address() as usize % PAGE_SIZE,
                    0,
                    "unaligned section start address"
                );

                serial_println!(
                    "==> Mapping section \"{}\" at {:p}, size: {}",
                    section.name().unwrap_or("<section>"),
                    section.start_address() as *const u8,
                    section.size()
                );
                for frame in FrameIter::new(
                    section.start_address() as usize,
                    section.end_address() as usize - 1,
                ) {
                    mapper.identity_map(frame, PageTableEntryFlags::RW, frame_allocator);
                }
            }

            if let Some(Ok(framebuffer)) = boot_info.framebuffer_tag() {
                serial_println!("Mapping VGA Framebuffer");
                for frame in FrameIter::new(
                    framebuffer.address() as usize,
                    framebuffer.address() as usize + framebuffer.size() - 1,
                ) {
                    mapper.identity_map(frame, PageTableEntryFlags::RW, frame_allocator);
                }
            }

            serial_println!("Mapping Multiboot information structure");
            for frame in FrameIter::new(boot_info.start_address(), boot_info.end_address() - 1) {
                mapper.identity_map(frame, PageTableEntryFlags::RW, frame_allocator);
            }

            serial_println!("Mapping Kernel heap");
            for page in PageIter::new(
                KERNEL_HEAP_START as usize,
                KERNEL_HEAP_START as usize + KERNEL_HEAP_SIZE,
            ) {
                mapper.map(page, PageTableEntryFlags::RW, frame_allocator);
            }
        },
    );

    let old_table = active_table.switch(new_table);
    let old_pml4_page =
        Page::containing_address(VirtualAddress(old_table.pml4_frame.start_address().0));
    unsafe {
        active_table.unmap(old_pml4_page, &mut kmm.frame_allocator);
    }
}

pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(PageTableEntryFlags::PRESENT) {
            Some(Frame::containing_address(
                (self.0 & 0x000fffff_fffff000).into(),
            ))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: PageTableEntryFlags) {
        assert_eq!(frame.start_address().0 & !0x000fffff_fffff000, 0);
        self.0 = frame.start_address().0 | flags.bits();
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("address", &self.pointed_frame())
            .field("flags", &self.flags())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub(crate) number: usize,
}

impl Page {
    pub fn containing_address(addr: VirtualAddress) -> Self {
        assert!(
            addr.0 < 0x0000_8000_0000_0000 || addr.0 >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}",
            addr.0
        );
        Self {
            number: addr.0 / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> VirtualAddress {
        VirtualAddress(self.number * PAGE_SIZE)
    }

    pub fn pml4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }

    pub fn pdpt_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }

    pub fn pdt_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }

    pub fn pt_index(&self) -> usize {
        self.number & 0o777
    }
}

pub struct PageIter {
    current: Page,
    end: Page,
}

impl PageIter {
    pub fn new(start_address: usize, end_address: usize) -> Self {
        Self {
            current: Page::containing_address(VirtualAddress(start_address)),
            end: Page::containing_address(VirtualAddress(end_address - 1)),
        }
    }
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.number <= self.end.number {
            let page = self.current;
            self.current.number += 1;
            Some(page)
        } else {
            None
        }
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct PageTableEntryFlags: usize {
        const PRESENT =   1 << 0;
        const RW =        1 << 1;
        const US =        1 << 2;
        const PWT =       1 << 3;
        const PCD =       1 << 4;
        const ACCESSED =  1 << 5;
        const DIRTY =     1 << 6;
        const PS =        1 << 7;
        const GLOBAL =    1 << 8;
        const NXE =       1 << 63;
    }
}

pub struct TemporaryPage {
    pub(crate) page: Page,
}

impl TemporaryPage {
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    pub fn map(
        &mut self,
        frame: Frame,
        active_table: &mut ActivePageTable,
        frame_allocator: &mut FrameAllocator,
    ) -> VirtualAddress {
        assert!(
            active_table.translate(self.page.start_address()).is_none(),
            "temporary page is already mapped"
        );

        active_table.map_to(self.page, frame, PageTableEntryFlags::RW, frame_allocator);
        self.page.start_address()
    }

    /// ## SAFETY:
    ///
    /// The caller must ensure that the page is no longer used.
    pub unsafe fn unmap(&mut self, active_table: &mut ActivePageTable) {
        active_table.unmap_without_freeing_frame(self.page);
    }

    pub fn map_table_frame(
        &mut self,
        frame: Frame,
        active_table: &mut ActivePageTable,
        frame_allocator: &mut FrameAllocator,
    ) -> &mut PageTable<PT> {
        unsafe { &mut *(self.map(frame, active_table, frame_allocator).0 as *mut PageTable<PT>) }
    }
}
