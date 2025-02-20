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
pub struct StaticBitMap<const S: usize> {
    bit_map: [u64; S],
}

impl<const S: usize> StaticBitMap<S> {
    pub const fn new() -> Self {
        Self { bit_map: [0; S] }
    }

    pub fn get(&self, index: usize) -> bool {
        if index >= self.len() {
            panic!(
                "attempt to access bit at index `{index}` where len is `{}`",
                self.len()
            );
        }

        let bitmap_idx = index / 64;
        let bit_idx = index % 64;

        ((self.bit_map[bitmap_idx] >> bit_idx) & 0b11) == 1
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

    pub fn len(&self) -> usize {
        self.bit_map.len() * 64
    }
}

pub const fn static_bitmap_size(size: usize) -> usize {
    size.div_ceil(64)
}
