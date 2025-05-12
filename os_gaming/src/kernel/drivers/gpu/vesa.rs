//! VESA/VBE GPU driver
//!
//! Provides basic framebuffer access through VESA BIOS Extensions.
extern crate alloc;
use alloc::boxed::Box;
use core::ptr;
use core::slice;

use super::specific::GpuDevice;
use super::{GpuInfo, GpuError, DisplayMode, TextureFormat};

/// Initialize VESA/VBE
pub fn init() -> Result<(), GpuError> {
    // In a real implementation, you'd initialize VESA here
    // For now, we'll assume it's working
    Ok(())
}

/// Create a VESA driver
pub fn create_driver() -> Result<Box<dyn GpuDevice>, GpuError> {
    // Get current video mode
    let mode = get_current_mode()?;
    
    let driver = VesaDriver {
        info: GpuInfo {
            vendor: "VESA",
            device: "VESA VBE Framebuffer",
            vram_size: 16 * 1024 * 1024, // Assume 16MB of VRAM
            max_texture_size: 2048,
            features: 0, // No hardware acceleration
            current_mode: mode,
            available_modes: get_available_modes(),
        },
        framebuffer: get_framebuffer_address()?,
        pitch: get_framebuffer_pitch()?,
        width: mode.width,
        height: mode.height,
        bpp: mode.bpp,
        clip_rect: None,
    };
    
    Ok(Box::new(driver))
}

/// Get current video mode
fn get_current_mode() -> Result<DisplayMode, GpuError> {
    // In a real implementation, you'd query VESA
    // For now, we'll return a default mode
    Ok(DisplayMode {
        width: 1024,
        height: 768,
        bpp: 32,
        refresh_rate: 60,
    })
}

/// Get available video modes
fn get_available_modes() -> &'static [DisplayMode] {
    static MODES: [DisplayMode; 3] = [
        DisplayMode { width: 800, height: 600, bpp: 32, refresh_rate: 60 },
        DisplayMode { width: 1024, height: 768, bpp: 32, refresh_rate: 60 },
        DisplayMode { width: 1280, height: 720, bpp: 32, refresh_rate: 60 },
    ];
    
    &MODES
}

/// Get framebuffer address
fn get_framebuffer_address() -> Result<usize, GpuError> {
    // In a real implementation, you'd query VESA
    // For now, we'll return a placeholder value
    // This should be replaced with the actual framebuffer address
    Ok(0xFD000000) // Example address
}

/// Get framebuffer pitch
fn get_framebuffer_pitch() -> Result<u32, GpuError> {
    // In a real implementation, you'd query VESA
    // For now, we'll calculate based on width and bpp
    let mode = get_current_mode()?;
    Ok(mode.width * (mode.bpp as u32 / 8))
}

/// Clipping rectangle
#[derive(Clone, Copy)]
struct ClipRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// VESA driver implementation
pub struct VesaDriver {
    /// GPU information
    info: GpuInfo,
    /// Framebuffer address
    framebuffer: usize,
    /// Bytes per row
    pitch: u32,
    /// Screen width
    width: u32,
    /// Screen height
    height: u32,
    /// Bits per pixel
    bpp: u8,
    /// Current clipping rectangle
    clip_rect: Option<ClipRect>,
}

impl GpuDevice for VesaDriver {
    fn get_info(&self) -> Result<GpuInfo, GpuError> {
        Ok(self.info.clone())
    }
    
    fn get_framebuffer(&mut self, _width: u32, _height: u32) -> Result<usize, GpuError> {
        Ok(self.framebuffer)
    }
    
    fn get_framebuffer_pitch(&self) -> Result<u32, GpuError> {
        Ok(self.pitch)
    }
    
    fn clear(&mut self, color: u32) -> Result<(), GpuError> {
        // Simple implementation that just fills the entire framebuffer
        let bytes_per_pixel = self.bpp as usize / 8;
        let framebuffer_size = self.pitch as usize * self.height as usize;
        
        unsafe {
            let mut ptr = self.framebuffer as *mut u8;
            let end = ptr.add(framebuffer_size);
            
            while ptr < end {
                for i in 0..bytes_per_pixel {
                    *ptr = ((color >> (8 * i)) & 0xFF) as u8;
                    ptr = ptr.add(1);
                }
            }
        }
        
        Ok(())
    }
    
    fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) -> Result<(), GpuError> {
        // Check bounds and apply clipping
        let (x, y, width, height) = self.apply_clip(x, y, width, height);
        if width == 0 || height == 0 {
            return Ok(());
        }
        
        let bytes_per_pixel = self.bpp as usize / 8;
        let stride = self.pitch as usize;
        
        unsafe {
            for row in 0..height {
                let mut ptr = self.framebuffer as *mut u8;
                ptr = ptr.add((y as usize + row as usize) * stride + x as usize * bytes_per_pixel);
                
                for _ in 0..width {
                    for i in 0..bytes_per_pixel {
                        *ptr = ((color >> (8 * i)) & 0xFF) as u8;
                        ptr = ptr.add(1);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) -> Result<(), GpuError> {
        // Simple Bresenham's line algorithm
        let mut x = x1;
        let mut y = y1;
        
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        
        let mut err = if dx > dy { dx } else { -dy } / 2;
        
        loop {
            // Draw pixel if within bounds
            if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                if self.is_in_clip(x, y) {
                    unsafe {
                        let bytes_per_pixel = self.bpp as usize / 8;
                        let offset = (y as usize * self.pitch as usize) + (x as usize * bytes_per_pixel);
                        let ptr = (self.framebuffer + offset) as *mut u8;
                        
                        for i in 0..bytes_per_pixel {
                            *ptr.add(i) = ((color >> (8 * i)) & 0xFF) as u8;
                        }
                    }
                }
            }
            
            if x == x2 && y == y2 {
                break;
            }
            
            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
        
        Ok(())
    }
    
    fn create_texture(&mut self, _width: u32, _height: u32, _format: u32, _data: &[u8]) -> Result<u32, GpuError> {
        // VESA doesn't support hardware textures
        Err(GpuError::UnsupportedFeature)
    }
    
    fn destroy_texture(&mut self, _texture_id: u32) -> Result<(), GpuError> {
        // VESA doesn't support hardware textures
        Err(GpuError::UnsupportedFeature)
    }
    
    fn get_texture_data(&self, _texture_id: u32) -> Result<&[u8], GpuError> {
        // VESA doesn't support hardware textures
        Err(GpuError::UnsupportedFeature)
    }
    
    fn draw_texture(&mut self, _texture_id: u32, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<(), GpuError> {
        // VESA doesn't support hardware textures
        Err(GpuError::UnsupportedFeature)
    }
    
    fn set_clip_rect(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError> {
        // Ensure the rectangle is within screen bounds
        let x = x.max(0).min(self.width as i32);
        let y = y.max(0).min(self.height as i32);
        let width = width.min((self.width as i32 - x).max(0) as u32);
        let height = height.min((self.height as i32 - y).max(0) as u32);
        
        self.clip_rect = Some(ClipRect { x, y, width, height });
        Ok(())
    }
    
    fn clear_clip_rect(&mut self) -> Result<(), GpuError> {
        self.clip_rect = None;
        Ok(())
    }
    
    fn set_blend_mode(&mut self, _mode: u32) -> Result<(), GpuError> {
        // VESA doesn't support blending
        Err(GpuError::UnsupportedFeature)
    }
    
    fn present(&mut self) -> Result<(), GpuError> {
        // For VESA, we're directly drawing to the framebuffer
        // so there's nothing to do here
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), GpuError> {
        // Nothing to shut down in VESA
        Ok(())
    }
}

impl VesaDriver {
    /// Check if a point is within the clipping rectangle
    fn is_in_clip(&self, x: i32, y: i32) -> bool {
        if let Some(clip) = self.clip_rect {
            x >= clip.x && x < clip.x + clip.width as i32 && 
            y >= clip.y && y < clip.y + clip.height as i32
        } else {
            true // No clipping
        }
    }
    
    /// Apply clipping to a rectangle
    fn apply_clip(&self, mut x: i32, mut y: i32, mut width: u32, mut height: u32) -> (i32, i32, u32, u32) {
        // First apply screen bounds
        if x < 0 {
            width = width.saturating_sub((-x) as u32);
            x = 0;
        }
        
        if y < 0 {
            height = height.saturating_sub((-y) as u32);
            y = 0;
        }
        
        if x + width as i32 > self.width as i32 {
            width = (self.width as i32 - x).max(0) as u32;
        }
        
        if y + height as i32 > self.height as i32 {
            height = (self.height as i32 - y).max(0) as u32;
        }
        
        // Then apply clipping rectangle if any
        if let Some(clip) = self.clip_rect {
            let clip_right = clip.x + clip.width as i32;
            let clip_bottom = clip.y + clip.height as i32;
            
            if x < clip.x {
                width = width.saturating_sub((clip.x - x) as u32);
                x = clip.x;
            }
            
            if y < clip.y {
                height = height.saturating_sub((clip.y - y) as u32);
                y = clip.y;
            }
            
            if x + width as i32 > clip_right {
                width = (clip_right - x).max(0) as u32;
            }
            
            if y + height as i32 > clip_bottom {
                height = (clip_bottom - y).max(0) as u32;
            }
        }
        
        (x, y, width, height)
    }
}