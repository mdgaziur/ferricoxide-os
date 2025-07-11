use crate::display::FRAMEBUFFER;
use crate::display::framebuffer::Pixel;
use alloc::vec;
use alloc::vec::Vec;
use noto_sans_mono_bitmap::{FontWeight, RasterHeight, get_raster, get_raster_width};

const RASTER_SIZE: RasterHeight = RasterHeight::Size24;

#[derive(Debug)]
pub struct TextRenderer {
    screen_height: usize,
    screen_width: usize,
    cursor: ScreenCursor,
    char_height: u8,
    char_width: u8,
    current_color: Pixel,
}

impl TextRenderer {
    pub fn new(screen_width: usize, screen_height: usize) -> TextRenderer {
        let char_width = get_raster_width(FontWeight::Regular, RASTER_SIZE) as u8;
        let char_height = RASTER_SIZE.val() as u8;

        let cols = screen_width / char_width as usize;
        let rows = screen_height / char_height as usize;

        Self {
            screen_height,
            screen_width,
            cursor: ScreenCursor::new(rows, cols),
            char_height,
            char_width,
            current_color: Pixel::WHITE,
        }
    }

    pub fn draw_char(&mut self, c: char) {
        let screen_x = self.cursor.x * self.char_width as usize;
        let screen_y = self.cursor.y * self.char_height as usize;
        let mut fb = FRAMEBUFFER.get().unwrap().lock();

        if c == '\n' {
            if self.cursor.new_line() {
                fb.scroll_up(RASTER_SIZE.val());
            }
            fb.flush();

            return;
        }

        let rasterized_char;
        if let Some(res) = get_raster(c, FontWeight::Regular, RASTER_SIZE) {
            rasterized_char = res;
        } else {
            rasterized_char = get_raster('�', FontWeight::Regular, RASTER_SIZE).unwrap();
        }

        let pixels = rasterized_char.raster();
        for (y, _) in pixels.iter().enumerate() {
            for (x, pixel) in pixels[y].iter().enumerate() {
                fb.put_pixel(
                    screen_x + x,
                    screen_y + y,
                    self.current_color.apply_intensity(*pixel),
                );
            }
        }

        if self.cursor.next() {
            fb.scroll_up(RASTER_SIZE.val());
        }
    }

    pub fn display_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.draw_char(ch);
        }
    }
}

/// Screen cursor for handling screen character position.
/// This cursor is dumb as it won't handle editing text and inserting
/// text differently. That means it will simply jump to the next line and
/// scroll the entire screen one row up when the current line is full. When
/// going back is necessary, it is up to the one printing the text to manually
/// set the position to proper position as this won't handle backspaces.
#[derive(Debug)]
struct ScreenCursor {
    line_sizes: Vec<usize>,
    rows: usize,
    cols: usize,
    x: usize,
    y: usize,
    target_line_size: usize,
}

impl ScreenCursor {
    pub fn new(rows: usize, cols: usize) -> Self {
        assert!(rows > 0 && cols > 0);
        Self {
            line_sizes: vec![0; rows],
            rows,
            cols,
            x: 0,
            y: 0,
            target_line_size: 0,
        }
    }

    pub fn reset_pos(&mut self) {
        self.set_pos(0, 0);
    }

    pub fn set_pos(&mut self, x: usize, y: usize) {
        assert!(x < self.cols);
        assert!(y < self.rows);

        self.x = x;
        self.y = y;
    }

    /// Shifts the cursor to insert a new character
    /// This will return true if scrolling up is necessary
    pub fn next(&mut self) -> bool {
        if self.x + 1 == self.cols {
            self.x = 0;

            return if self.y + 1 == self.rows {
                true
            } else {
                self.y += 1;
                false
            };
        }

        self.x += 1;
        false
    }

    /// Acts like a new line character. This will return true
    /// if scrolling up is necessary.
    pub fn new_line(&mut self) -> bool {
        self.x = 0;
        if self.y + 1 == self.rows {
            true
        } else {
            self.y += 1;
            false
        }
    }
}
