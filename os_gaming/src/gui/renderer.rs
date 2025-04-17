//! GPU/Hardware accelerated rendering system
//!
//! This module provides hardware-accelerated rendering capabilities for the GUI system.
//! It abstracts away the details of graphics hardware and provides a clean API for
//! drawing primitives and managing textures.
extern crate alloc;
use alloc::{vec, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

use crate::kernel::drivers::gpu;
use crate::kernel::memory;
use crate::kernel::memory::physical::PhysicalMemoryManager;

/// Color in RGBA format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a new color with specified RGBA values
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new color with RGB values and full opacity
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Convert color to 32-bit RGBA value
    pub fn to_rgba(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }
    
    /// Convert color to 32-bit ARGB value (common format for many hardware interfaces)
    pub fn to_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    // Define some common colors
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    
    // UI-specific colors
    pub const UI_BACKGROUND: Self = Self::rgb(45, 45, 48);
    pub const UI_FOREGROUND: Self = Self::rgb(200, 200, 200);
    pub const UI_ACCENT: Self = Self::rgb(0, 120, 215);
    pub const UI_ACCENT_DARK: Self = Self::rgb(0, 90, 158);
}

/// Represents a rectangular area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Create a new rectangle with specified position and size
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width as i32 && 
        y >= self.y && y < self.y + self.height as i32
    }

    /// Check if this rectangle intersects with another
    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.x + self.width as i32 <= other.x || 
          other.x + other.width as i32 <= self.x || 
          self.y + self.height as i32 <= other.y || 
          other.y + other.height as i32 <= self.y)
    }

    /// Get the intersection of this rectangle with another
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width as i32).min(other.x + other.width as i32) - x;
        let height = (self.y + self.height as i32).min(other.y + other.height as i32) - y;

        if width <= 0 || height <= 0 {
            return None;
        }

        Some(Rect::new(x, y, width as u32, height as u32))
    }
}

/// Represents a hardware-accelerated texture
pub struct Texture {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

/// Possible texture formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    RGBA8,
    RGB8,
    BGRA8,
    A8,
}

/// Blending mode for rendering operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    None,
    Alpha,
    Additive,
    Multiply,
}

/// Renderer capabilities determined at initialization
pub struct RendererCapabilities {
    pub max_texture_size: u32,
    pub supports_blend_modes: bool,
    pub supports_render_targets: bool,
    pub supports_shaders: bool,
}

/// Main renderer responsible for all drawing operations
pub struct Renderer {
    // Hardware-related fields
    width: u32,
    height: u32,
    framebuffer: *mut u32,
    framebuffer_pitch: u32,
    
    // State tracking
    clip_rect: Option<Rect>,
    blend_mode: BlendMode,
    
    // Hardware acceleration status
    gpu_accelerated: AtomicBool,
    capabilities: RendererCapabilities,
    
    // Resource tracking
    textures: Mutex<Vec<Texture>>,
}

// Error type for renderer operations
#[derive(Debug)]
pub enum RendererError {
    InitializationFailed,
    InvalidParameters,
    TextureCreationFailed,
    DrawingFailed,
}

/// Safe implementation of Renderer
impl Renderer {
    /// Create a new renderer using the specified width and height
    pub fn new(width: u32, height: u32) -> Result<Self, RendererError> {
        // Try to initialize GPU hardware
        let gpu_initialized = match gpu::init() {
            Ok(_) => {
                #[cfg(feature = "log")]
                log::info!("GPU hardware acceleration initialized");
                true
            },
            Err(_) => {
                #[cfg(feature = "log")]
                log::warn!("GPU hardware acceleration not available, falling back to software rendering");
                false
            }
        };
        
        // Get framebuffer address
        let framebuffer = match gpu::get_framebuffer(width, height) {
            Ok(addr) => addr as *mut u32,
            Err(_) => {
                // Allocate our own framebuffer in system memory
                let size = (width * height * 4) as usize;
                let addr = memory::allocate_virtual(size, 16)
                    .map_err(|_| RendererError::InitializationFailed)?;
                
                // Zero the framebuffer
                unsafe {
                    core::ptr::write_bytes(addr, 0, size);
                }
                
                addr as *mut u32
            }
        };
        
        // Get framebuffer pitch (bytes per row)
        let framebuffer_pitch = gpu::get_framebuffer_pitch().unwrap_or(width * 4);
        
        // Detect hardware capabilities
        let capabilities = if gpu_initialized {
            RendererCapabilities {
                max_texture_size: gpu::get_max_texture_size().unwrap_or(2048),
                supports_blend_modes: gpu::supports_feature(gpu::Feature::Blending).unwrap_or(false),
                supports_render_targets: gpu::supports_feature(gpu::Feature::RenderTargets).unwrap_or(false),
                supports_shaders: gpu::supports_feature(gpu::Feature::Shaders).unwrap_or(false),
            }
        } else {
            RendererCapabilities {
                max_texture_size: 2048,
                supports_blend_modes: false,
                supports_render_targets: false,
                supports_shaders: false,
            }
        };
        
        let renderer = Self {
            width,
            height,
            framebuffer,
            framebuffer_pitch: framebuffer_pitch / 4, // Convert to pixels
            clip_rect: None,
            blend_mode: BlendMode::Alpha,
            gpu_accelerated: AtomicBool::new(gpu_initialized),
            capabilities,
            textures: Mutex::new(Vec::new()),
        };
        
        Ok(renderer)
    }
    
    /// Clear the entire screen to a specific color
    pub fn clear(&mut self, color: Color) {
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            // Use hardware acceleration if available
            if gpu::clear(color.to_rgba()).is_err() {
                // Fall back to software implementation
                self.clear_software(color);
            }
        } else {
            // Use software implementation
            self.clear_software(color);
        }
    }
    
    /// Software implementation of screen clearing
    fn clear_software(&self, color: Color) {
        let color_value = color.to_rgba();
        let framebuffer = self.framebuffer;
        let pixel_count = self.width * self.height;
        
        unsafe {
            // Fast path: memset the entire buffer if possible
            if color.r == color.g && color.g == color.b && color.b == color.a {
                core::ptr::write_bytes(framebuffer as *mut u8, color.r, (pixel_count * 4) as usize);
            } else {
                // Set each pixel individually
                for i in 0..pixel_count {
                    *framebuffer.add(i as usize) = color_value;
                }
            }
        }
    }
    
    /// Draw a filled rectangle
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        // Apply clipping
        let draw_rect = if let Some(clip) = self.clip_rect {
            match rect.intersection(&clip) {
                Some(r) => r,
                None => return, // Outside clip area
            }
        } else {
            rect
        };
        
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            // Use hardware acceleration if available
            if gpu::fill_rect(draw_rect.x, draw_rect.y, 
                            draw_rect.width, draw_rect.height, 
                            color.to_rgba()).is_err() {
                // Fall back to software implementation
                self.fill_rect_software(draw_rect, color);
            }
        } else {
            // Use software implementation
            self.fill_rect_software(draw_rect, color);
        }
    }
    
    /// Software implementation of rectangle filling
    fn fill_rect_software(&self, rect: Rect, color: Color) {
        let color_value = color.to_rgba();
        let framebuffer = self.framebuffer;
        let pitch = self.framebuffer_pitch;
        
        let x_start = rect.x.max(0).min(self.width as i32 - 1) as usize;
        let y_start = rect.y.max(0).min(self.height as i32 - 1) as usize;
        let x_end = (rect.x + rect.width as i32).max(0).min(self.width as i32) as usize;
        let y_end = (rect.y + rect.height as i32).max(0).min(self.height as i32) as usize;
        
        if x_start >= x_end || y_start >= y_end {
            return;
        }
        
        unsafe {
            for y in y_start..y_end {
                let row = framebuffer.add(y * pitch as usize + x_start);
                
                if color.a == 255 {
                    // Fast path for opaque colors
                    for x in 0..(x_end - x_start) {
                        *row.add(x) = color_value;
                    }
                } else if color.a > 0 {
                    // Alpha blending for transparent colors
                    for x in 0..(x_end - x_start) {
                        let dst_pixel = row.add(x);
                        let src_color = color;
                        let dst_color = self.unpack_color(*dst_pixel);
                        *dst_pixel = self.blend_colors(src_color, dst_color).to_rgba();
                    }
                }
            }
        }
    }
    
    /// Draw a single pixel
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) {
        // Check bounds and clipping
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        
        if let Some(clip) = self.clip_rect {
            if !clip.contains(x, y) {
                return;
            }
        }
        
        unsafe {
            let pixel = self.framebuffer.add(y as usize * self.framebuffer_pitch as usize + x as usize);
            
            if color.a == 255 {
                // Opaque pixel, just set it
                *pixel = color.to_rgba();
            } else if color.a > 0 {
                // Blend with existing pixel
                let dst_color = self.unpack_color(*pixel);
                *pixel = self.blend_colors(color, dst_color).to_rgba();
            }
        }
    }
    
    /// Draw a line between two points
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            // Use hardware acceleration if available
            if gpu::draw_line(x1, y1, x2, y2, color.to_rgba()).is_err() {
                // Fall back to software implementation
                self.draw_line_software(x1, y1, x2, y2, color);
            }
        } else {
            // Use software implementation
            self.draw_line_software(x1, y1, x2, y2, color);
        }
    }
    
    /// Software implementation of line drawing using Bresenham's algorithm
    fn draw_line_software(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        let mut x = x1;
        let mut y = y1;
        
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        
        let mut err = if dx > dy { dx } else { -dy } / 2;
        let mut err2;
        
        loop {
            self.draw_pixel(x, y, color);
            
            if x == x2 && y == y2 {
                break;
            }
            
            err2 = err;
            
            if err2 > -dx {
                err -= dy;
                x += sx;
            }
            
            if err2 < dy {
                err += dx;
                y += sy;
            }
        }
    }
    
    /// Draw an outlined rectangle
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        // Top edge
        self.draw_line(rect.x, rect.y, 
                      rect.x + rect.width as i32 - 1, rect.y, 
                      color);
        
        // Right edge
        self.draw_line(rect.x + rect.width as i32 - 1, rect.y, 
                      rect.x + rect.width as i32 - 1, rect.y + rect.height as i32 - 1, 
                      color);
        
        // Bottom edge
        self.draw_line(rect.x, rect.y + rect.height as i32 - 1, 
                      rect.x + rect.width as i32 - 1, rect.y + rect.height as i32 - 1, 
                      color);
        
        // Left edge
        self.draw_line(rect.x, rect.y, 
                      rect.x, rect.y + rect.height as i32 - 1, 
                      color);
    }
    
    /// Draw a texture to the screen
    pub fn draw_texture(&mut self, texture_id: u32, dst_rect: Rect) -> Result<(), RendererError> {
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            // Use hardware acceleration if available
            if gpu::draw_texture(texture_id, dst_rect.x, dst_rect.y, 
                               dst_rect.width, dst_rect.height).is_ok() {
                return Ok(());
            }
            // Fall through to software implementation if hardware failed
        }
        
        // Extract the texture information in a separate scope
        let texture_info = {
            let textures = self.textures.lock();
            let texture = textures.iter()
                .find(|t| t.id == texture_id)
                .ok_or(RendererError::InvalidParameters)?;
            
            // Clone the texture info we need
            (texture.width, texture.height, texture.format)
        };
        
        // Unpack texture info
        let (texture_width, texture_height, texture_format) = texture_info;
        
        // Get texture data from GPU memory
        let data = match gpu::get_texture_data(texture_id) {
            Ok(data) => data,
            Err(_) => return Err(RendererError::DrawingFailed),
        };
        
        // Calculate scaling factors
        let scale_x = dst_rect.width as f32 / texture_width as f32;
        let scale_y = dst_rect.height as f32 / texture_height as f32;
        
        // Draw the texture using software rendering
        for y in 0..dst_rect.height {
            let src_y = (y as f32 / scale_y) as u32;
            if src_y >= texture_height {
                continue;
            }
            
            let dst_y = dst_rect.y + y as i32;
            if dst_y < 0 || dst_y >= self.height as i32 {
                continue;
            }
            
            for x in 0..dst_rect.width {
                let src_x = (x as f32 / scale_x) as u32;
                if src_x >= texture_width {
                    continue;
                }
                
                let dst_x = dst_rect.x + x as i32;
                if dst_x < 0 || dst_x >= self.width as i32 {
                    continue;
                }
                
                // Apply clipping
                if let Some(clip) = self.clip_rect {
                    if !clip.contains(dst_x, dst_y) {
                        continue;
                    }
                }
                
                // Get source pixel
                let pixel_index = (src_y * texture_width + src_x) as usize;
                let src_pixel = match texture_format {
                    TextureFormat::RGBA8 | TextureFormat::BGRA8 => {
                        let offset = pixel_index * 4;
                        if offset + 3 >= data.len() {
                            continue;
                        }
                        
                        if texture_format == TextureFormat::RGBA8 {
                            Color::new(data[offset], data[offset + 1], data[offset + 2], data[offset + 3])
                        } else {
                            Color::new(data[offset + 2], data[offset + 1], data[offset], data[offset + 3])
                        }
                    },
                    TextureFormat::RGB8 => {
                        let offset = pixel_index * 3;
                        if offset + 2 >= data.len() {
                            continue;
                        }
                        
                        Color::rgb(data[offset], data[offset + 1], data[offset + 2])
                    },
                    TextureFormat::A8 => {
                        if pixel_index >= data.len() {
                            continue;
                        }
                        
                        Color::new(255, 255, 255, data[pixel_index])
                    }
                };
                
                // Draw the pixel with proper blending
                if src_pixel.a > 0 {
                    self.draw_pixel(dst_x, dst_y, src_pixel);
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a texture from pixel data
    pub fn create_texture(&self, width: u32, height: u32, format: TextureFormat, 
                        data: &[u8]) -> Result<u32, RendererError> {
        // Check texture size limits
        if width == 0 || height == 0 || width > self.capabilities.max_texture_size || 
           height > self.capabilities.max_texture_size {
            return Err(RendererError::InvalidParameters);
        }
        
        // Calculate expected data size
        let bytes_per_pixel = match format {
            TextureFormat::RGBA8 | TextureFormat::BGRA8 => 4,
            TextureFormat::RGB8 => 3,
            TextureFormat::A8 => 1,
        };
        
        let expected_size = (width * height * bytes_per_pixel) as usize;
        if data.len() < expected_size {
            return Err(RendererError::InvalidParameters);
        }
        
        // Try hardware texture creation first
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            let texture_id = match gpu::create_texture(width, height, format as u32, data) {
                Ok(id) => id,
                Err(_) => return Err(RendererError::TextureCreationFailed),
            };
            
            // Store texture information
            let mut textures = self.textures.lock();
            textures.push(Texture {
                id: texture_id,
                width,
                height,
                format,
            });
            
            return Ok(texture_id);
        } else {
            // Fall back to software textures
            // For now, we'll just use the GPU API for storing texture data
            // In a real implementation, you might want to add software texture support
            return Err(RendererError::TextureCreationFailed);
        }
    }
    
    /// Destroy a texture and free its resources
    pub fn destroy_texture(&self, texture_id: u32) -> Result<(), RendererError> {
        // Remove from our tracking
        let mut textures = self.textures.lock();
        let index = textures.iter().position(|t| t.id == texture_id)
            .ok_or(RendererError::InvalidParameters)?;
        textures.remove(index);
        
        // Free GPU resources
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            if let Err(_) = gpu::destroy_texture(texture_id) {
                return Err(RendererError::InvalidParameters);
            }
        }
        
        Ok(())
    }
    
    /// Set the current clipping rectangle
    pub fn set_clip_rect(&mut self, rect: Option<Rect>) {
        self.clip_rect = rect.map(|r| {
            // Ensure the rectangle is within screen bounds
            let x = r.x.max(0).min(self.width as i32);
            let y = r.y.max(0).min(self.height as i32);
            let width = r.width.min((self.width as i32 - x).max(0) as u32);
            let height = r.height.min((self.height as i32 - y).max(0) as u32);
            
            Rect::new(x, y, width, height)
        });
        
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            if let Some(rect) = self.clip_rect {
                let _ = gpu::set_clip_rect(rect.x, rect.y, rect.width, rect.height);
            } else {
                let _ = gpu::clear_clip_rect();
            }
        }
    }
    
    /// Set the current blend mode
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
        
        if self.gpu_accelerated.load(Ordering::Relaxed) && 
           self.capabilities.supports_blend_modes {
            let _ = gpu::set_blend_mode(mode as u32);
        }
    }
    
    /// Present the current frame to the screen
    pub fn present(&self) -> Result<(), RendererError> {
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            // Hardware-accelerated presentation
            if let Err(_) = gpu::present() {
                // Fall back to software presentation if hardware fails
                self.present_software()?;
            }
        } else {
            // Software presentation
            self.present_software()?;
        }
        
        Ok(())
    }
    
    /// Software implementation of frame presentation
    fn present_software(&self) -> Result<(), RendererError> {
        // In a real implementation, this would copy our framebuffer to the
        // actual screen framebuffer or a memory-mapped video buffer
        
        // For simplicity, we'll assume the framebuffer is already mapped to video memory
        Ok(())
    }
    
    /// Get renderer dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    /// Check if hardware acceleration is available
    pub fn is_accelerated(&self) -> bool {
        self.gpu_accelerated.load(Ordering::Relaxed)
    }
    
    /// Get renderer capabilities
    pub fn capabilities(&self) -> &RendererCapabilities {
        &self.capabilities
    }
    
    /// Convert a 32-bit RGBA value back to a Color struct
    fn unpack_color(&self, rgba: u32) -> Color {
        Color {
            r: ((rgba >> 24) & 0xFF) as u8,
            g: ((rgba >> 16) & 0xFF) as u8,
            b: ((rgba >> 8) & 0xFF) as u8,
            a: (rgba & 0xFF) as u8,
        }
    }
    
    /// Blend two colors according to the current blend mode
    fn blend_colors(&self, src: Color, dst: Color) -> Color {
        match self.blend_mode {
            BlendMode::None => src,
            BlendMode::Alpha => {
                let src_a = src.a as f32 / 255.0;
                let dst_a = dst.a as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);
                
                if out_a <= 0.001 {
                    return Color::TRANSPARENT;
                }
                
                let out_r = ((src.r as f32 * src_a + dst.r as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                let out_g = ((src.g as f32 * src_a + dst.g as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                let out_b = ((src.b as f32 * src_a + dst.b as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                
                Color::new(out_r, out_g, out_b, (out_a * 255.0) as u8)
            },
            BlendMode::Additive => {
                let src_a = src.a as f32 / 255.0;
                
                let out_r = (src.r as f32 * src_a + dst.r as f32).min(255.0) as u8;
                let out_g = (src.g as f32 * src_a + dst.g as f32).min(255.0) as u8;
                let out_b = (src.b as f32 * src_a + dst.b as f32).min(255.0) as u8;
                
                Color::rgb(out_r, out_g, out_b)
            },
            BlendMode::Multiply => {
                let src_a = src.a as f32 / 255.0;
                
                let out_r = ((src.r as f32 / 255.0 * dst.r as f32 / 255.0) * 255.0) as u8;
                let out_g = ((src.g as f32 / 255.0 * dst.g as f32 / 255.0) * 255.0) as u8;
                let out_b = ((src.b as f32 / 255.0 * dst.b as f32 / 255.0) * 255.0) as u8;
                
                let out_r = (out_r as f32 * src_a + dst.r as f32 * (1.0 - src_a)) as u8;
                let out_g = (out_g as f32 * src_a + dst.g as f32 * (1.0 - src_a)) as u8;
                let out_b = (out_b as f32 * src_a + dst.b as f32 * (1.0 - src_a)) as u8;
                
                Color::rgb(out_r, out_g, out_b)
            },
        }
    }
}

// Clean up renderer resources on drop
impl Drop for Renderer {
    fn drop(&mut self) {
        // Destroy all textures
        let textures = self.textures.lock();
        for texture in textures.iter() {
            let _ = gpu::destroy_texture(texture.id);
        }
        
        // Shut down GPU if we initialized it
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            let _ = gpu::shutdown();
        }
        
        // Free the framebuffer if we allocated it ourselves
        if self.framebuffer as usize != gpu::get_framebuffer(0, 0).unwrap_or(0) as usize {
            unsafe {
                memory::deallocate_virtual(self.framebuffer as *mut u8);
            }
        }
    }
}