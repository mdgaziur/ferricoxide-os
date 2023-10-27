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

use crate::arch::mm_bck::PhysicalAddress;
use crate::kutils::BitMap;
use crate::units::{GiB, KiB};

use core::cmp::{max, min};
use core::mem::{align_of, size_of};
use multiboot2::{MemoryArea, MemoryAreaType};

const BITMAP_SIZE_FOR_128G: usize = ((128 * GiB) / (4 * KiB)) / 64;

pub struct FrameAllocator {
    bitmap: &'static mut BitMap<BITMAP_SIZE_FOR_128G>,
}

impl FrameAllocator {
    pub const SIZE: usize = size_of::<BitMap<BITMAP_SIZE_FOR_128G>>();

    pub fn new(
        memory_areas: &[MemoryArea],
        multiboot_info_start: usize,
        multiboot_info_end: usize,
        kernel_start: usize,
        kernel_end: usize,
    ) -> Self {
        const REQUIRED_FREE_SPACE: usize = size_of::<BitMap<BITMAP_SIZE_FOR_128G>>();

        let mut bitmap: Option<&'static mut BitMap<BITMAP_SIZE_FOR_128G>> = None;

        for area in memory_areas {
            if area.typ() == MemoryAreaType::Available {
                let area_start = area.start_address() as usize;
                let area_end = area.end_address() as usize;
                let available_parts = [
                    (area_start, min(kernel_start, multiboot_info_start)),
                    (
                        min(kernel_end, multiboot_info_end),
                        max(kernel_start, multiboot_info_start),
                    ),
                    (max(kernel_end, multiboot_info_end), area_end),
                ];

                for (start, end) in available_parts {
                    let mut aligned_start = start as *mut BitMap<BITMAP_SIZE_FOR_128G>;
                    unsafe {
                        aligned_start =
                            aligned_start.byte_add(align_of::<BitMap<BITMAP_SIZE_FOR_128G>>() - 1);
                    }
                    if end.saturating_sub(aligned_start as _) >= REQUIRED_FREE_SPACE {
                        let bitmap_ref = unsafe { &mut *(start as *mut _) };
                        *bitmap_ref = BitMap::new();
                        bitmap = Some(bitmap_ref);
                    }
                }
            }
        }

        if let Some(bitmap) = bitmap {
            for frame in FrameIter::new(kernel_start, kernel_end) {
                bitmap.set(frame.0);
            }

            for frame in FrameIter::new(multiboot_info_start, multiboot_info_end) {
                bitmap.set(frame.0);
            }

            for area in memory_areas {
                if area.typ() != MemoryAreaType::Available {
                    for frame in FrameIter::new(
                        area.start_address() as usize / FRAME_SIZE,
                        area.end_address() as usize / FRAME_SIZE,
                    ) {
                        bitmap.set(frame.0);
                    }
                }
            }

            let bitmap_start = bitmap as *mut _ as usize;
            for frame in FrameIter::new(
                bitmap_start,
                bitmap_start + (size_of::<BitMap<BITMAP_SIZE_FOR_128G>>() - 1),
            ) {
                if frame.0 > BITMAP_SIZE_FOR_128G {
                    continue;
                }
                bitmap.set(frame.0);
            }

            return Self { bitmap };
        }

        panic!("OOM: cannot find enough space for creating a frame allocator")
    }

    pub fn alloc(&mut self, count: usize) -> Option<Frame> {
        let mut consec = 0;
        let mut end_idx = 0;

        for (idx, status) in self.bitmap.iter().enumerate() {
            if !status {
                consec += 1;
            } else {
                consec = 0;
            }

            if consec == count {
                end_idx = idx;
                break;
            }
        }
        if consec == count {
            let start_idx = end_idx - count + 1;
            for status_idx in start_idx..start_idx + count {
                assert!(!self.bitmap.nth(status_idx));
                self.bitmap.set(status_idx);
            }

            Some(Frame(start_idx))
        } else {
            None
        }
    }

    pub unsafe fn free(&mut self, first_frame: Frame, count: usize) {
        for status_idx in first_frame.0..first_frame.0 + count {
            self.bitmap.clear(status_idx);
        }
    }
}

pub const FRAME_SIZE: usize = 4 * KiB;

#[derive(Debug)]
pub struct Frame(pub usize);

impl Frame {
    pub fn containing_address(addr: PhysicalAddress) -> Self {
        Self(addr.0 / FRAME_SIZE)
    }

    pub fn start_address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 * FRAME_SIZE)
    }

    pub fn clone(&self) -> Self {
        Self(self.0)
    }
}

pub struct FrameIter {
    pub start: usize,
    pub end: usize,
}

impl FrameIter {
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        Self {
            start: start_addr / FRAME_SIZE,
            end: end_addr / FRAME_SIZE,
        }
    }
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let frame = Frame(self.start);
            self.start += 1;

            Some(frame)
        }
    }
}
