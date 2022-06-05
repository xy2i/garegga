//! VGA driver

use ::volatile::Volatile;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(fg: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct VgaChar {
    char: u8,
    color_code: ColorCode,
}

#[repr(C)]
pub struct Writer {
    buf: &'static mut [[Volatile<VgaChar>; Writer::WIDTH]; Writer::HEIGHT],
    col: usize,
    color_code: ColorCode,
}

impl Writer {
    const WIDTH: usize = 80;
    const HEIGHT: usize = 25;

    fn new_line(&mut self) {
        for row in 1..Writer::HEIGHT {
            for col in 0..Writer::WIDTH {
                let character = self.buf[row][col].read();
                self.buf[row - 1][col].write(character);
            }
        }

        // clear out last row
        let blank = VgaChar {
            char: b' ',
            color_code: self.color_code,
        };
        for col in 0..Writer::WIDTH {
            self.buf[Writer::HEIGHT - 1][col].write(blank);
        }

        self.col = 0;
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= Writer::WIDTH {
                    self.new_line();
                }

                // always write on bottom
                let row = Writer::HEIGHT - 1;
                let col = self.col;

                self.buf[row][col].write(VgaChar {
                    char: byte,
                    color_code: self.color_code,
                });

                self.col += 1;
            }
        }
    }

    pub fn write(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

pub fn print() {
    let mut w = Writer {
        buf: unsafe {
            &mut *(0xb8000 as *mut [[Volatile<VgaChar>; Writer::WIDTH]; Writer::HEIGHT])
        },
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        col: 0,
    };
    w.write("abcdefghijklmnopqrstuvwxyzfffff");
    w.write("abcdefghijklmnopqrstuvwxyzfffff");
    w.write("abcdefghijklmnopqrstuvwxyzfffff");
}
