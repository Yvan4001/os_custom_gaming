extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

// For fallback VGA support
use super::vga;

//For HDMI support
use super::hdmi;

use super::displayport;

/// Display modes supported by the driver
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Text80x25,        // Standard VGA text mode
    Vesa640x480,      // VESA VGA 640x480
    Vesa800x600,      // VESA VGA 800x600
    Vesa1024x768,     // VESA VGA 1024x768
    Vesa1280x1024,    // VESA VGA 1280x1024
    Vesa1920x1080,    // VESA VGA 1920x1080
    Vesa2560x1440,    // VESA VGA 2560x1440
    Vesa3840x2160,    // VESA VGA 3840x2160
    Vesa5120x2880,    // VESA VGA 5120x2880
    Custom(u32, u32), // Custom resolution
}

/// Color depths supported by display modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorDepth {
    Bpp8,  // 8 bits per pixel (256 colors)
    Bpp16, // 16 bits per pixel (65K colors)
    Bpp24, // 24 bits per pixel (16M colors)
    Bpp32, // 32 bits per pixel (16M colors + alpha)
}

/// Represents a pixel color in RGB format
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Display device information
pub struct DisplayInfo {
    current_mode: DisplayMode,
    current_depth: ColorDepth,
    width: u32,
    height: u32,
    pitch: u32,       // Bytes per row
    framebuffer: u64, // Physical address of framebuffer
    initialized: AtomicBool,
}

impl Color {
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const RED: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Color = Color {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    /// Create a new color from RGB values
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a new color with alpha
    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to packed 32-bit RGBA
    pub fn to_rgba32(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Convert to packed 24-bit RGB
    pub fn to_rgb24(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to packed 16-bit RGB (5:6:5)
    pub fn to_rgb16(&self) -> u16 {
        (((self.r as u16) >> 3) << 11) | (((self.g as u16) >> 2) << 5) | ((self.b as u16) >> 3)
    }
}

// Global display context
lazy_static! {
    static ref DISPLAY: Mutex<DisplayInfo> = Mutex::new(DisplayInfo {
        current_mode: DisplayMode::Text80x25,
        current_depth: ColorDepth::Bpp8,
        width: 80,
        height: 25,
        pitch: 160,  // 80 characters * 2 bytes per character
        framebuffer: 0xB8000, // VGA text mode buffer
        initialized: AtomicBool::new(false),
    });
}

/// Initialize the display subsystem
pub fn init() -> Result<(), &'static str> {
    let mut display = DISPLAY.lock();

    if display.initialized.load(Ordering::SeqCst) {
        return Ok(());
    }

    // First initialize VGA text mode for basic output
    vga::init();

    // Second initialize HDMI or other display interfaces
    hdmi::init();
    displayport::init();

    // Then try to set up a higher resolution mode if possible
    // In a real driver, we would use VESA BIOS extensions or a GPU driver

    #[cfg(feature = "std")]
    {
        // In std mode, we pretend to support higher resolutions
        // but actually use the terminal/window for output
        display.current_mode = DisplayMode::Vesa800x600;
        display.current_depth = ColorDepth::Bpp32;
        display.width = 800;
        display.height = 600;
        display.pitch = 800 * 4; // 4 bytes per pixel (32bpp)
                                 // Framebuffer would be allocated in real driver

        log::info!(
            "Display initialized: {}x{} {}bpp",
            display.width,
            display.height,
            match display.current_depth {
                ColorDepth::Bpp8 => 8,
                ColorDepth::Bpp16 => 16,
                ColorDepth::Bpp24 => 24,
                ColorDepth::Bpp32 => 32,
            }
        );
    }

    display.initialized.store(true, Ordering::SeqCst);
    Ok(())
}

/// Set the display mode
pub fn set_mode(mode: DisplayMode, depth: ColorDepth) -> Result<(), &'static str> {
    let mut display = DISPLAY.lock();

    if !display.initialized.load(Ordering::SeqCst) {
        return Err("Display not initialized");
    }

    // In a real driver, this would call into the GPU/display hardware
    // to change the video mode

    match mode {
        DisplayMode::Text80x25 => {
            // Switch back to text mode
            vga::switch_to_text_mode();
            display.current_mode = mode;
            display.current_depth = ColorDepth::Bpp8;
            display.width = 80;
            display.height = 25;
            display.pitch = 160;
            display.framebuffer = 0xB8000;
        }
        DisplayMode::Vesa640x480 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 640;
            display.height = 480;
            display.pitch = 640
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa800x600 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 800;
            display.height = 600;
            display.pitch = 800
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa1024x768 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 1024;
            display.height = 768;
            display.pitch = 1024
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa1280x1024 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 1280;
            display.height = 1024;
            display.pitch = 1280
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                })
        }
        DisplayMode::Vesa1920x1080 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 1920;
            display.height = 1080;
            display.pitch = 1920
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa2560x1440 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 2560;
            display.height = 1440;
            display.pitch = 2560
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa3840x2160 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 3840;
            display.height = 2160;
            display.pitch = 3840
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Vesa5120x2880 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 4096;
            display.height = 2160;
            display.pitch = 4096
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        DisplayMode::Custom(width, height) => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = width;
            display.height = height;
            display.pitch = width
                * (match depth {
                    ColorDepth::Bpp8 => 1,
                    ColorDepth::Bpp16 => 2,
                    ColorDepth::Bpp24 => 3,
                    ColorDepth::Bpp32 => 4,
                });
        }
        // All variants of DisplayMode have been handled
    }

    #[cfg(feature = "std")]
    log::info!("Display mode set to: {:?} with {:?}", mode, depth);

    Ok(())
}

/// Draw a pixel at the specified location using the most appropriate display driver
pub fn draw_pixel(x: u32, y: u32, color: Color) -> Result<(), &'static str> {
    let display = DISPLAY.lock();

    if !display.initialized.load(Ordering::SeqCst) {
        return Err("Display not initialized");
    }

    if x >= display.width || y >= display.height {
        return Err("Coordinates out of bounds");
    }

    // In text mode, we can't directly draw pixels
    if display.current_mode == DisplayMode::Text80x25 {
        return Err("Cannot draw pixels in text mode");
    }

    // Drop the mutex lock to prevent deadlocks when calling other modules
    drop(display);

    // Determine which display driver to use based on resolution and capabilities
    let result = match DISPLAY.lock().current_mode {
        DisplayMode::Text80x25 => {
            // Already handled above with error
            unreachable!()
        },
        // Lower resolution modes - try VGA first
        DisplayMode::Vesa640x480 | DisplayMode::Vesa800x600 => {
            // Try using VGA for these lower resolutions
            #[cfg(not(feature = "std"))]
            {
                // In real OS mode, try to use VGA hardware
                vga::set_pixel(x as usize, y as usize, vga::Color::new_rgb(color.r, color.g, color.b));
                try_modern_displays(x, y, color)
            }

            #[cfg(feature = "std")]
            try_modern_displays(x, y, color)
        },
        // Higher resolution modes - use HDMI or DisplayPort
        _ => try_modern_displays(x, y, color),
    };

    result
}

/// Helper function to try using HDMI or DisplayPort
fn try_modern_displays(x: u32, y: u32, color: Color) -> Result<(), &'static str> {
    // Try HDMI first as it's more common
    let hdmi_result = hdmi::set_pixel(x, y, color.r, color.g, color.b, color.a);

    if hdmi_result.is_ok() {
        return Ok(());
    }

    // Fall back to DisplayPort if HDMI fails
    let dp_result = displayport::set_pixel(x, y, color.r, color.g, color.b, color.a);

    if dp_result.is_ok() {
        return Ok(());
    }

    // If we get here, both modern display methods failed

    #[cfg(feature = "std")]
    {
        // In std mode, we can simulate drawing pixels
        log::trace!(
            "Drawing pixel at ({}, {}) with color RGB({}, {}, {})",
            x,
            y,
            color.r,
            color.g,
            color.b
        );
        return Ok(());
    }

    // In OS mode, if all display methods failed, return the error from HDMI
    return hdmi_result
}

/// Define an additional helper function for VGA to set pixel with RGB values
#[cfg(not(feature = "std"))]
impl vga::Color {
    pub fn new_rgb(r: u8, g: u8, b: u8) -> Self {
        // Convert RGB value to closest VGA color palette index
        // This is a simplified conversion for demonstration purposes
        let r_level = r / 85; // 0-2
        let g_level = g / 85; // 0-2
        let b_level = b / 85; // 0-2

        match (r_level, g_level, b_level) {
            (0, 0, 0) => vga::Color::Black,
            (2, 0, 0) => vga::Color::Red,
            (0, 2, 0) => vga::Color::Green,
            (2, 2, 0) => vga::Color::Brown,
            (0, 0, 2) => vga::Color::Blue,
            (2, 0, 2) => vga::Color::Magenta,
            (0, 2, 2) => vga::Color::Cyan,
            (1, 1, 1) => vga::Color::DarkGray,
            (2, 2, 2) => vga::Color::White,
            _ => vga::Color::Gray,
        }
    }
}

/// Clear the screen with a specified color
pub fn clear_screen(color: Color) -> Result<(), &'static str> {
    let display = DISPLAY.lock();

    if !display.initialized.load(Ordering::SeqCst) {
        return Err("Display not initialized");
    }

    if display.current_mode == DisplayMode::Text80x25 {
        // In text mode, use VGA text functions
        vga::clear_screen();
        return Ok(());
    }

    // In a real driver, this would fill the framebuffer
    // For now, we'll just simulate it

    #[cfg(feature = "std")]
    {
        // In std mode, we'd use the window system to clear
        // This is just a placeholder
    }

    Ok(())
}

/// Print text at the specified location
pub fn print_text(x: u32, y: u32, text: &str, color: Color) -> Result<(), &'static str> {
    let display = DISPLAY.lock();

    if !display.initialized.load(Ordering::SeqCst) {
        return Err("Display not initialized");
    }

    if display.current_mode == DisplayMode::Text80x25 {
        // In text mode, use VGA text functions
        let attribute = vga::convert_to_attribute(vga::Color::White, vga::Color::Black);
        vga::print_at(x as usize, y as usize, text, attribute)?;
        return Ok(());
    }

    // In a real driver, this would render text to the framebuffer
    // For now, we'll just simulate it

    #[cfg(feature = "std")]
    {
        // In std mode, we'd use the window system to print text
        // This is just a placeholder
    }

    Ok(())
}

/// Get the current display info
pub fn get_display_info() -> DisplayInfo {
    DISPLAY.lock().clone()
}

// Add Clone implementation for DisplayInfo
impl Clone for DisplayInfo {
    fn clone(&self) -> Self {
        Self {
            current_mode: self.current_mode,
            current_depth: self.current_depth,
            width: self.width,
            height: self.height,
            pitch: self.pitch,
            framebuffer: self.framebuffer,
            initialized: AtomicBool::new(self.initialized.load(Ordering::SeqCst)),
        }
    }
}