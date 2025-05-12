use core::{arch::asm, fmt};
use spin::Mutex;
use lazy_static::lazy_static;
use core::fmt::Write;
use x86_64::instructions::interrupts;


// VGA text buffer constants
const VGA_BUFFER_HEIGHT: usize = 25;
const VGA_BUFFER_WIDTH: usize = 80;
const VGA_BUFFER_ADDRESS: usize = 0xb8000;

// VGA color codes
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
    Gray = 16,
}

// Color code structure - combines foreground and background colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

// Structure for a character in the VGA buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

// The VGA text buffer as a 2D array
#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; VGA_BUFFER_WIDTH]; VGA_BUFFER_HEIGHT],
}

// Writer structure to manage VGA output
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn new() -> Writer {
        Writer {
            column_position: 0,
            color_code: ColorCode::new(Color::LightGray, Color::Black),
            buffer: unsafe { &mut *(VGA_BUFFER_ADDRESS as *mut Buffer) },
        }
    }
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= VGA_BUFFER_WIDTH {
                    self.new_line();
                }

                let row = VGA_BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.buffer.chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };

                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // Not part of printable ASCII range
                _ => self.write_byte(0xfe), // ■ character
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..VGA_BUFFER_HEIGHT {
            for col in 0..VGA_BUFFER_WIDTH {
                let character = self.buffer.chars[row][col];
                self.buffer.chars[row - 1][col] = character;
            }
        }
        self.clear_row(VGA_BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..VGA_BUFFER_WIDTH {
            self.buffer.chars[row][col] = blank;
        }
    }
    
    pub fn clear_screen(&mut self) {
        for row in 0..VGA_BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// Global writer instance
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightGray, Color::Black),
        buffer: unsafe { &mut *(VGA_BUFFER_ADDRESS as *mut Buffer) },
    });
}

// Public interface for printing
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::kernel::drivers::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    
    // Disable interrupts while locking the WRITER to prevent deadlocks
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

// Public functions for changing colors and clearing the screen
pub fn set_color(foreground: Color, background: Color) {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        writer.color_code = ColorCode::new(foreground, background);
    });
}

pub fn clear_screen() {
    interrupts::without_interrupts(|| {
        WRITER.lock().clear_screen();
    });
}

// Function to initialize the VGA driver
pub fn init() {
    // Initialize the VGA text buffer
    let mut writer = WRITER.lock();
    writer.clear_screen();
    writer.write_string("VGA Text Mode Initialized\n");
    writer.write_string("Press any key to continue...\n");
}

pub fn switch_to_text_mode() {
    // Switch to text mode
    unsafe {
        asm!("int3");
    }
    // Clear the screen
    clear_screen();
}

/// Set a pixel at the specified coordinates
pub fn set_pixel(
    x: usize,
    y: usize,
    color: Color,
) {
    // Set a pixel at the specified coordinates
    unsafe {
        asm!("int3");
    }
}

//convert_to_attribute(vga::Color::White, vga::Color::Black);
pub fn convert_to_attribute(foreground: Color, background: Color) -> u8 {
    (background as u8) << 4 | (foreground as u8)
}

//print_at(x as usize, y as usize, text, attribute)?;
pub fn print_at(x: usize, y: usize, text: &str, attribute: u8) -> Result<(), &'static str> {
    // Print text at the specified location
    unsafe {
        asm!("int3");
    }
    Ok(())
}

pub fn new_line() {
    // Move to the next line by calling the writer's new_line method
    interrupts::without_interrupts(|| {
        WRITER.lock().new_line();
    });
}

pub fn backspace() {
    // Handle backspace: move back one character and clear it
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        
        if writer.column_position > 0 {
            // Move back one column
            writer.column_position -= 1;
            
            // Clear the character by overwriting with a space
            let row = VGA_BUFFER_HEIGHT - 1;
            let col = writer.column_position;
            
            writer.buffer.chars[row][col] = ScreenChar {
                ascii_character: b' ',
                color_code: writer.color_code,
            };
        } else if VGA_BUFFER_HEIGHT > 1 {
            // Optional: If at start of line, move up to end of previous line
            // This is more complex and would require adding row tracking to Writer
            // For simplicity, we'll just do nothing when at column 0
        }
    });
}

pub fn print_char(c: char) {
    // Print a single character
    interrupts::without_interrupts(|| {
        // Convert char to byte and write it
        // ASCII characters fit in u8, but char might be Unicode
        if c.is_ascii() {
            WRITER.lock().write_byte(c as u8);
        } else {
            // Print a replacement character for non-ASCII
            WRITER.lock().write_byte(0xfe); // ■ character
        }
    });
}