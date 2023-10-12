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

use crate::units::{GiB, KiB};
use core::mem::align_of;
use core::ops::Add;

/// A very naive frame allocator that basically does a lookup on the bitmaps until it finds
/// frames that can hold given amount of data. This is a thin wrapper around [`FrameAllocBitmap`]
/// which does most of the heavy lifting for allocating frames.
pub struct FrameAllocator {
    frame_alloc_bitmaps: &'static mut [FrameAllocBitmap; BITMAP_COUNT_FOR_128G as usize],
}

impl FrameAllocator {
    /// Initializes the frame allocator
    ///
    /// ## Safety
    ///
    /// Kernel bottom address must point to the end address of the kernel and the next 2MB(after proper alignment)
    /// *must* not be used by anything else.
    pub unsafe fn new(kernel_bottom_addr: *const u8) -> Self {
        let next_addr_to_kernel_bottom = kernel_bottom_addr.offset(1);
        let frame_alloc_bitmaps_addr = next_addr_to_kernel_bottom
            .offset(next_addr_to_kernel_bottom.align_offset(align_of::<u64>()) as isize);

        Self {
            frame_alloc_bitmaps: &mut *(frame_alloc_bitmaps_addr as *mut _),
        }
    }

    pub fn alloc(&mut self, frame_count: u64) -> Option<Frame> {
        for (idx, bitmap) in self.frame_alloc_bitmaps.iter_mut().enumerate() {
            if let Some(frame_idx_bitmap) = bitmap.alloc(frame_count) {
                return Some(Frame(idx as u64 * 64 + frame_idx_bitmap))
            }
        }

        None
    }

    /// Deallocates the given frame
    ///
    /// ## Safety
    ///
    /// Caller must ensure that the given frame is not used by anything
    /// when the function is called.
    pub unsafe fn dealloc(&mut self, frame: Frame) {
        let bitmap_idx = frame.0 % 64;
        self.frame_alloc_bitmaps[bitmap_idx as usize].dealloc(frame.0 - bitmap_idx);
    }

    /// Deallocates consecutive frames
    ///
    /// ## Safety
    ///
    /// Caller must ensure that the given frame is not used by anything
    /// when the function is called.
    pub unsafe fn dealloc_consecutive(&mut self, frame: Frame, frame_count: u64) {
        for n in 0..frame_count {
            self.dealloc(frame + n);
        }
    }

    /// Marks the given frame as allocated.
    ///
    /// Useful for excluding unusable memory areas from allocation.
    pub fn mark_as_allocated(&mut self, frame: Frame) {
        let bitmap_idx = frame.0 % 64;
        self.frame_alloc_bitmaps[bitmap_idx as usize].mark_as_allocated(frame.0 - bitmap_idx);
    }
}

/// Right now the kernel will support only 128GiB of physical memory. The entire physical memory
/// is divided into 4KiB sized frames. Each frame's status is represented by a bit in a 64bit
/// bitmap. So, the amount of bitmaps required to keep track of an entire 128GiB physical memory will be:
///
/// `(128GiB / 4KiB) / 64`
pub const BITMAP_COUNT_FOR_128G: u64 = ((128 * GiB) / (4 * KiB)) as u64 / 64;

/// Stores frame allocation info in a 64bit bitmap.
///
/// The index of each frame is calculated using the following formula:
///
/// `idx_of_bitmap + idx_of_bit`
///
/// Each bit of the bitmap represents whether a frame referenced by that bit
/// is free or not. If free, the bit is set to 0 otherwise it's set to 1.
///
#[derive(Debug)]
pub struct FrameAllocBitmap(u64);

impl FrameAllocBitmap {
    /// Searches the bitmap `frame_count` consecutive free frames.
    /// If it finds them, it returns the index of the bit.
    pub fn alloc(&mut self, frame_count: u64) -> Option<u64> {
        let mut consec = 0;
        let mut resulting_frame = None;

        for idx in 0..64 {
            resulting_frame = Some(idx);

            let bit = self.0 >> idx & 1;
            if bit == 0 {
                consec += 1;
            } else {
                consec = 0;
                resulting_frame = None;
            }

            if consec == frame_count {
                let resulting_frame = resulting_frame.unwrap();
                for bit_idx in resulting_frame..=resulting_frame + frame_count {
                    self.0 = self.0 | (1 << bit_idx);
                }

                break;
            }
        }


        resulting_frame
    }

    /// Deallocs the given frame by setting the corresponding bit in the bitmap
    /// to 0
    ///
    /// ## Safety
    ///
    /// Caller *must* ensure that the given frame is not used by anything else
    /// when this function is called.
    pub unsafe fn dealloc(&mut self, frame_idx: u64) {
        self.0 = self.0 & (0 << frame_idx);
    }

    /// Marks the given frame as allocated.
    ///
    /// Useful for excluding unusable memory areas from allocation.
    pub fn mark_as_allocated(&mut self, frame_idx: u64) {
        self.0 = self.0 & (1 << frame_idx);
    }
}

pub const FRAME_SIZE: u64 = 4 * KiB as u64;

#[derive(Debug, Copy, Clone)]
pub struct Frame(u64);

impl Frame {
    pub fn containing_address(addr: u64) -> Self {
        Frame(addr / FRAME_SIZE)
    }

    pub fn addr(&self) -> u64 {
        self.0 * FRAME_SIZE
    }
}

impl Add<u64> for Frame {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}
