use core::fmt::Write;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// The standard color palette in VGA text mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii: u8,
    color: ColorCode,
}

struct Buffer {
    buffer: &'static mut [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl Buffer {
    /// # SAFETY
    /// See [`TextModeWriter::new`]
    pub unsafe fn new() -> Self {
        Self {
            buffer: &mut *(0xb8000 as *mut _),
        }
    }
}

pub struct TextModeWriter {
    buffer: Buffer,
    col: usize,
    row: usize,
}

impl TextModeWriter {
    /// # SAFETY
    /// Caller must ensure that VGA text mode is available
    pub unsafe fn init() -> Self {
        Self {
            buffer: Buffer::new(),
            col: 0,
            row: BUFFER_HEIGHT - 1,
        }
    }

    fn write_string(&mut self, s: &str) {
        for ch in s.chars() {
            let ch = ch as u8;

            match ch {
                b'\n' => self.new_line(),
                0x20..=0x7e => self.write_byte(ch),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn write_byte(&mut self, ch: u8) {
        self.buffer.buffer[self.row][self.col] = ScreenChar {
            ascii: ch,
            color: ColorCode::new(Color::White, Color::Black),
        };

        self.col += 1;
        if self.col >= BUFFER_WIDTH {
            self.new_line();
        }
    }

    fn clear_row(&mut self, row: usize) {
        for col in 0..BUFFER_WIDTH {
            self.buffer.buffer[row][col] = ScreenChar {
                ascii: b' ',
                color: ColorCode::new(Color::White, Color::Black),
            };
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.buffer[row][col];
                self.buffer.buffer[row - 1][col] = character;
            }
        }

        self.col = 0;
        self.clear_row(BUFFER_HEIGHT - 1);
    }
}

impl Write for TextModeWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);

        Ok(())
    }
}
