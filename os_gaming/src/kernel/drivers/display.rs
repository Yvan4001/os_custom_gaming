extern crate alloc;
use core::sync::atomic::{AtomicBool, Ordering};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

// For fallback VGA support
use super::vga;

/// Display modes supported by the driver
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Text80x25,       // Standard VGA text mode
    Vesa640x480,     // VESA VGA 640x480
    Vesa800x600,     // VESA VGA 800x600
    Vesa1024x768,    // VESA VGA 1024x768
    Vesa1280x1024,   // VESA VGA 1280x1024
    Vesa1920x1080,   // VESA VGA 1920x1080
    Custom(u32, u32), // Custom resolution
}

/// Color depths supported by display modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorDepth {
    Bpp8,   // 8 bits per pixel (256 colors)
    Bpp16,  // 16 bits per pixel (65K colors)
    Bpp24,  // 24 bits per pixel (16M colors)
    Bpp32,  // 32 bits per pixel (16M colors + alpha)
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
    pitch: u32,        // Bytes per row
    framebuffer: u64,  // Physical address of framebuffer
    initialized: AtomicBool,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    
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
        
        log::info!("Display initialized: {}x{} {}bpp", 
            display.width, display.height, 
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
        },
        DisplayMode::Vesa640x480 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 640;
            display.height = 480;
            display.pitch = 640 * (match depth {
                ColorDepth::Bpp8 => 1,
                ColorDepth::Bpp16 => 2,
                ColorDepth::Bpp24 => 3,
                ColorDepth::Bpp32 => 4,
            });
        },
        DisplayMode::Vesa800x600 => {
            display.current_mode = mode;
            display.current_depth = depth;
            display.width = 800;
            display.height = 600;
            display.pitch = 800 * (match depth {
                ColorDepth::Bpp8 => 1,
                ColorDepth::Bpp16 => 2,
                ColorDepth::Bpp24 => 3,
                ColorDepth::Bpp32 => 4,
            });
        },
        // Handle other resolutions similarly
        _ => return Err("Unsupported display mode"),
    }
    
    #[cfg(feature = "std")]
    log::info!("Display mode set to: {:?} with {:?}", mode, depth);
    
    Ok(())
}

/// Draw a pixel at the specified location
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
    
    // In a real driver, this would write to the framebuffer
    // For now, we'll just simulate it
    
    #[cfg(feature = "std")]
    {
        // In std mode, we'd use the window system to draw
        // This is just a placeholder
    }
    
    Ok(())
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