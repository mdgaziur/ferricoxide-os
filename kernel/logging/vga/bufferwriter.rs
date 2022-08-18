use crate::vga::{Pixel, VGA_DRAWER};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write;
use font8x8::{UnicodeFonts, BASIC_FONTS};

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = FONT_WIDTH;
const PIXEL: Pixel = Pixel {
    r: 255,
    g: 255,
    b: 255,
};
const SPACE_CHAR: [u8; 8] = [0u8; FONT_HEIGHT];

pub struct Buffer {
    buffer: Vec<Vec<[u8; 8]>>,
    cols: usize,
    rows: usize,
    cur_row: usize,
    cur_col: usize,
}

impl Buffer {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            buffer: vec![vec![SPACE_CHAR; cols]; rows],
            cols,
            rows,
            cur_row: 0,
            cur_col: 0,
        }
    }

    fn write_string(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\n' => {
                    self.new_line();
                }
                _ => self.write_byte(ch),
            }
        }
    }

    fn get_bytearray_for_char(ch: char) -> [u8; 8] {
        if let Some(byte_array) = BASIC_FONTS.get(ch) {
            byte_array
        } else {
            SPACE_CHAR
        }
    }

    fn write_byte(&mut self, ch: char) {
        self.buffer[self.cur_row][self.cur_col] = Self::get_bytearray_for_char(ch);

        self.cur_col += 1;
        if self.cur_col >= self.cols {
            self.new_line();
        }
    }

    fn draw_bitmap(&self, bitmap: &[u8; 8], col: usize, row: usize) {
        let mut drawer_binding = VGA_DRAWER.lock();
        let drawer = &mut drawer_binding.unwrap_ref_mut().buffer;

        let mut y_pos = row * 8;
        for scanline in bitmap {
            for bit_idx in 0..FONT_WIDTH {
                let bit = scanline >> bit_idx & 1;

                if bit == 1 {
                    drawer.write_pixel(PIXEL, col * 8 + bit_idx + 1, y_pos);
                }
            }
            y_pos += 1;
        }
    }

    fn clear_row(&mut self, row: usize) {
        for col in 0..self.cols {
            self.buffer[row][col] = SPACE_CHAR;
        }
    }

    fn new_line(&mut self) {
        if self.cur_row + 1 >= self.rows {
            for row in 1..self.rows {
                for col in 0..self.cols {
                    self.buffer[row - 1][col] = self.buffer[row][col];
                }
            }

            self.cur_row = self.rows - 1;
            self.clear_row(self.cur_row);
            VGA_DRAWER
                .lock()
                .unwrap_ref_mut()
                .buffer
                .move_up(FONT_HEIGHT);
        } else {
            self.cur_row += 1;
        }
        self.cur_col = 0;
    }

    fn commit(&self) {
        for (row_idx, row) in self.buffer.iter().enumerate() {
            for (col_idx, col) in row.iter().enumerate() {
                self.draw_bitmap(col, col_idx, row_idx);
            }
        }
    }
}

pub struct BufferWriter {
    buffer: Buffer,
}

impl BufferWriter {
    pub fn new(height: usize, width: usize) -> Self {
        let cols: usize = width / FONT_WIDTH;
        let rows: usize = height / FONT_HEIGHT;

        Self {
            buffer: Buffer::new(rows, cols),
        }
    }
}

impl Write for BufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.buffer.write_string(s);
        self.buffer.commit();
        Ok(())
    }
}
