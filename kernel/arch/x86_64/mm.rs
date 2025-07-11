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
use crate::arch::x86_64::mm::frame::{Frame, FrameAllocator};
use crate::arch::x86_64::{KERNEL_CONTENT_INFO, STACKOVERFLOW_GUARD};
use core::cmp::max;
use core::ptr::addr_of;
mod frame;
pub mod paging;

use crate::arch::x86_64::mm::frame::FRAME_ALLOCATOR;
use crate::arch::x86_64::mm::paging::flags::PageTableEntryFlags;
use crate::arch::x86_64::mm::paging::{
    ActivePML4, InactivePML4, Page, TemporaryPage, identity_map_range, map_range, map_virtual_range,
};
use crate::kutils::MB;
use crate::{BOOT_INFO, serial_println};
use linked_list_allocator::LockedHeap;
use spin::{Mutex, Once};

pub type PhysAddr = usize;

pub type VirtAddr = usize;

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
const KERNEL_HEAP_SIZE: usize = 16 * MB;

pub static ACTIVE_PML4: Once<Mutex<ActivePML4>> = Once::new();

pub fn init() {
    FRAME_ALLOCATOR.lock().init();

    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    let mut temporary_page = TemporaryPage::new(Page { number: 69420420 }, &mut *frame_allocator);

    let mut active_pml4 = unsafe { ActivePML4::new() };
    let mut new_table = {
        let frame = frame_allocator.allocate().expect("OOM: sucks!");
        InactivePML4::new(frame, &mut active_pml4, &mut temporary_page)
    };

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
    let mut heap_addr = 0;

    active_pml4.with(&mut new_table, &mut temporary_page, |mapper| {
        let kernel_end = map_range(
            kernel_start_virt_addr,
            kernel_start_phys_addr,
            kernel_content_size,
            PageTableEntryFlags::PRESENT,
            mapper,
            &mut *frame_allocator,
        );

        let boot_info_end = identity_map_range(
            boot_info_start_addr,
            boot_info_total_size,
            PageTableEntryFlags::PRESENT | PageTableEntryFlags::NO_EXECUTE,
            mapper,
            &mut *frame_allocator,
        );

        let framebuffer_end = identity_map_range(
            framebuffer_address,
            framebuffer_size,
            PageTableEntryFlags::PRESENT | PageTableEntryFlags::NO_EXECUTE,
            mapper,
            &mut *frame_allocator,
        );
        // just in case something ends up after the kernel content(somehow!)
        heap_addr = max(kernel_end, max(boot_info_end, framebuffer_end));
    });

    unsafe {
        active_pml4.switch(new_table);
        active_pml4.unmap(
            Page::containing_address(addr_of!(STACKOVERFLOW_GUARD) as usize),
            &mut *frame_allocator,
        );
    }

    map_virtual_range(
        heap_addr,
        KERNEL_HEAP_SIZE,
        PageTableEntryFlags::empty(),
        &mut active_pml4.mapper,
        &mut *frame_allocator,
    );

    // SAFETY:
    // The 64MB area is frame allocated and mapped into proper address
    unsafe {
        KERNEL_HEAP_ALLOCATOR
            .lock()
            .init(heap_addr as *mut u8, KERNEL_HEAP_SIZE);
    }

    ACTIVE_PML4.call_once(|| Mutex::new(active_pml4));

    serial_println!(
        "Total memory: {} MB",
        frame_allocator.total_memory() as f64 / MB as f64
    );
    serial_println!(
        "Available memory: {} MB",
        frame_allocator.available_memory() as f64 / MB as f64
    );
    serial_println!(
        "Kernel heap size: {} MB",
        KERNEL_HEAP_SIZE as f64 / MB as f64
    );
    serial_println!(
        "Free kernel heap: {} MB",
        KERNEL_HEAP_ALLOCATOR.lock().free() as f64 / MB as f64
    );
}

pub fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    let active_pml4 = ACTIVE_PML4.get().unwrap().lock();

    active_pml4.translate(addr)
}

pub fn identity_map(addr: PhysAddr, flags: PageTableEntryFlags) {
    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    let mut active_pml4 = ACTIVE_PML4.get().unwrap().lock();

    active_pml4.identity_map(
        Frame::containing_address(addr),
        flags,
        &mut *frame_allocator,
    );
}

fn align_up(addr: usize, alignment: usize) -> usize {
    (addr + alignment - 1) & !(alignment - 1)
}

fn align_down(addr: usize, alignment: usize) -> usize {
    addr & !(alignment - 1)
}
