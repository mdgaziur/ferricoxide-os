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
use alloc::vec;
use alloc::vec::Vec;

/// # Statically allocated bit-map.
///
/// Usage:
///
/// ```
/// let mut bitmap = StaticBitMap::<{ static_bitmap_size(123) }>::new();
/// bitmap.set(10);
/// assert!(bitmap.get(10));
/// bitmap.clear(10);
/// assert!(!bitmap.get(10));
/// ```
#[derive(Debug)]
pub struct StaticBitmap<const S: usize> {
    bit_map: [u64; S],
}

impl<const S: usize> StaticBitmap<S> {
    pub const fn new() -> Self {
        Self { bit_map: [0; S] }
    }

    pub fn get(&self, index: usize) -> bool {
        match self.try_get(index) {
            Some(bit) => bit,
            None => panic!(
                "attempt to access bit at index `{index}` where len is `{}`",
                self.len()
            ),
        }
    }

    pub fn try_get(&self, index: usize) -> Option<bool> {
        if index >= self.len() {
            return None;
        }

        let bitmap_idx = index / 64;
        let bit_idx = index % 64;

        Some(((self.bit_map[bitmap_idx] >> bit_idx) & 0b1) == 1)
    }

    pub fn set(&mut self, index: usize) {
        if index >= self.len() {
            panic!(
                "attempt to access bit at index `{index}` where len is `{}`",
                self.len()
            );
        }

        let bitmap_idx = index / 64;
        let bit_idx = index % 64;

        let bitmap = &mut self.bit_map[bitmap_idx];
        *bitmap |= 1 << bit_idx;
    }

    pub fn clear(&mut self, index: usize) {
        if index >= self.len() {
            panic!(
                "attempt to access bit at index `{index}` where len is `{}`",
                self.len()
            );
        }

        let bitmap_idx = index / 64;
        let bit_idx = index % 64;

        let bitmap = &mut self.bit_map[bitmap_idx];
        *bitmap &= !(1 << bit_idx) as u64;
    }

    pub fn iter(&self) -> StaticBitMapIterator<'_, S> {
        StaticBitMapIterator {
            bitmap: self,
            current_pos: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.bit_map.len() * 64
    }
}

pub struct StaticBitMapIterator<'a, const S: usize> {
    bitmap: &'a StaticBitmap<S>,
    current_pos: usize,
}

impl<const S: usize> Iterator for StaticBitMapIterator<'_, S> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.bitmap.try_get(self.current_pos);
        self.current_pos += 1;

        res
    }
}

pub const fn static_bitmap_size(size: usize) -> usize {
    size.div_ceil(64)
}

pub struct RingBuffer<T> {
    storage: Vec<T>,
    capacity: usize,
    len: usize,
    reader: usize,
    writer: usize,
}

impl<T: Default + Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> RingBuffer<T> {
        Self {
            storage: vec![T::default(); capacity],
            capacity,
            len: 0,
            reader: 0,
            writer: 0,
        }
    }

    pub fn get(&mut self) -> Option<&T> {
        if self.len == 0 {
            return None
        }

        let current_val = &self.storage[self.reader];

        self.reader += 1;
        self.reader %= self.len;

        Some(current_val)
    }

    pub fn insert(&mut self, value: T) {
        if self.len < self.capacity {
            self.len += 1;
        }

        self.storage[self.writer] = value;
        
        self.writer += 1;
        self.writer %= self.capacity;
    }

    pub fn extend(&mut self, extend_by: usize) {
        self.storage.reserve(extend_by);
        self.capacity += extend_by;
    }
    
    pub fn get_all(&self) -> &[T] {
        &self.storage[..self.len]
    }
}
