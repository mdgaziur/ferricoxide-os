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

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct BitMap<const SIZE: usize> {
    inner: [usize; SIZE],
}

impl<const SIZE: usize> BitMap<SIZE> {
    pub fn new() -> Self {
        Self { inner: [0; SIZE] }
    }

    pub fn set(&mut self, index: usize) {
        let container_idx = index / 64;
        let bitmap_idx = index % 64;

        self.inner[container_idx] |= 1 << bitmap_idx;
    }

    pub fn iter(&self) -> BitSetIterator<SIZE> {
        BitSetIterator {
            bitset: self,
            current: 0,
        }
    }

    pub fn clear(&mut self, index: usize) {
        let container_idx = index / 64;
        let bitmap_idx = index % 64;

        self.inner[container_idx] &= !(1 << bitmap_idx);
    }

    pub fn nth(&self, index: usize) -> bool {
        let container_idx = index / 64;
        let bitmap_idx = index % 64;

        self.inner[container_idx] >> bitmap_idx & 1 == 1
    }

    pub fn len(&self) -> usize {
        self.inner.len() * 64
    }
}

pub struct BitSetIterator<'a, const SIZE: usize> {
    bitset: &'a BitMap<SIZE>,
    current: usize,
}

impl<'a, const SIZE: usize> Iterator for BitSetIterator<'a, SIZE> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.bitset.len() {
            let next_bit = self.bitset.nth(self.current);
            self.current += 1;

            Some(next_bit)
        } else {
            None
        }
    }
}
