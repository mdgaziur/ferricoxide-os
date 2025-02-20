/*
 * FerricOxide OS is an operating system that aims to be posix compliant and memory safe
 * Copyright (C) 2024  MD Gaziur Rahman Noor
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
use core::ptr::addr_of;
use crate::arch::x86_64::mm::frame::{Frame, FrameAllocator};
use crate::arch::x86_64::{BOOT_INFO, KERNEL_CONTENT_INFO, STACKOVERFLOW_GUARD};
mod frame;
pub mod paging;

use crate::arch::x86_64::mm::frame::FRAME_ALLOCATOR;
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::paging::{
    ActivePML4, InactivePML4, Mapper, Page, TemporaryPage, PAGE_SIZE,
};

pub type PhysAddr = usize;

pub type VirtAddr = usize;

pub fn mm_init() {
    FRAME_ALLOCATOR.lock().init();
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    let mut temporary_page = TemporaryPage::new(Page { number: 69420420 }, &mut *frame_allocator);

    let mut active_pml4 = unsafe { ActivePML4::new() };
    let mut new_table = {
        let frame = frame_allocator.allocate().expect("OOM: sucks!");
        InactivePML4::new(frame, &mut active_pml4, &mut temporary_page)
    };

    fn map_range(
        virt_start: usize,
        phys_start: usize,
        size: usize,
        flags: PageTableEntryFlags,
        mapper: &mut Mapper<'_>,
        frame_allocator: &mut impl FrameAllocator,
    ) {
        assert_eq!(virt_start % PAGE_SIZE, 0);
        assert_eq!(phys_start % PAGE_SIZE, 0);

        let size = align_up(size, PAGE_SIZE);
        let mut n = 0;
        while n * PAGE_SIZE <= size {
            mapper.map_to(
                Page::containing_address(virt_start + n * PAGE_SIZE),
                Frame::containing_address(phys_start + n * PAGE_SIZE),
                flags,
                &mut *frame_allocator,
            );

            n += 1;
        }
    }

    fn identity_map_range(
        phys_start: usize,
        size: usize,
        flags: PageTableEntryFlags,
        mapper: &mut Mapper<'_>,
        frame_allocator: &mut impl FrameAllocator,
    ) {
        assert_eq!(phys_start % PAGE_SIZE, 0);

        let size = align_up(size, PAGE_SIZE);
        let mut n = 0;
        while n * PAGE_SIZE <= size {
            mapper.identity_map(
                Frame::containing_address(phys_start + n * PAGE_SIZE),
                flags,
                &mut *frame_allocator,
            );

            n += 1;
        }
    }
    let kernel_content_info = KERNEL_CONTENT_INFO.get().unwrap();
    let kernel_content_size =
        (kernel_content_info.phys_end_addr - kernel_content_info.phys_start_addr + 1) as usize;
    let kernel_start_virt_addr = kernel_content_info.virt_start_addr as usize;
    let kernel_start_phys_addr = kernel_content_info.phys_start_addr as usize;

    let boot_info = BOOT_INFO.get().unwrap();
    let boot_info_start_addr = boot_info.start_address();
    let boot_info_total_size = boot_info.total_size();

    let framebuffer = boot_info.framebuffer_tag().unwrap().unwrap();
    let framebuffer_address = framebuffer.address() as usize;
    let framebuffer_size = (framebuffer.height() * framebuffer.pitch()) as usize;

    active_pml4.with(&mut new_table, &mut temporary_page, |mapper| {
        map_range(
            kernel_start_virt_addr,
            kernel_start_phys_addr,
            kernel_content_size,
            PageTableEntryFlags::PRESENT,
            mapper,
            &mut *frame_allocator,
        );

        identity_map_range(
            boot_info_start_addr,
            boot_info_total_size,
            PageTableEntryFlags::PRESENT | PageTableEntryFlags::NO_EXECUTE,
            mapper,
            &mut *frame_allocator,
        );

        identity_map_range(
            framebuffer_address,
            framebuffer_size,
            PageTableEntryFlags::PRESENT | PageTableEntryFlags::NO_EXECUTE,
            mapper,
            &mut *frame_allocator,
        );
    });

    unsafe {
        active_pml4.switch(new_table);
        active_pml4.unmap(
            Page::containing_address(addr_of!(STACKOVERFLOW_GUARD) as usize),
            &mut *frame_allocator
        );
    }
}

fn align_up(addr: usize, alignment: usize) -> usize {
    (addr + alignment - 1) & !(alignment - 1)
}

fn align_down(addr: usize, alignment: usize) -> usize {
    addr & !(alignment - 1)
}
