//! GPU/Hardware accelerated rendering system
//!
//! This module provides hardware-accelerated rendering capabilities for the GUI system.
//! It abstracts away the details of graphics hardware and provides a clean API for
//! drawing primitives and managing textures.
extern crate alloc;
use alloc::{vec, vec::Vec, format, string::String}; // Added format and String
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use serde::{Serialize, Deserialize};
use crate::kernel::memory;

use crate::kernel::drivers::gpu;
use crate::kernel::memory::memory_manager::MemoryError as KernelMemoryError;
use crate::kernel::memory::memory_manager::MemoryProtectionFlags;
use crate::kernel::memory::memory_manager::CacheType;
use crate::kernel::memory::memory_manager::MemoryType;
use crate::kernel::memory::memory_manager::MemoryInfo;
use crate::kernel::memory::memory_manager::MemoryInitError;


use x86_64::VirtAddr;
use micromath::F32Ext; // For f32.round() if used in blend_colors

/// Color in RGBA format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color { /* ... as before ... */
    pub r: u8, pub g: u8, pub b: u8, pub a: u8,
}
impl Color { /* ... as before ... */
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self { Self { r, g, b, a } }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b, a: 255 } }
    pub fn to_rgba(&self) -> u32 { ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32) }
    pub fn to_argb(&self) -> u32 { ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) }
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0,0,0,0);
    pub const UI_BACKGROUND: Self = Self::rgb(45, 45, 48);
    pub const UI_FOREGROUND: Self = Self::rgb(200, 200, 200);
    pub const UI_ACCENT: Self = Self::rgb(0, 120, 215);
}

/// Represents a rectangular area
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect { /* ... as before ... */
    pub x: i32, pub y: i32, pub width: u32, pub height: u32,
}
impl Rect { /* ... as before ... */
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self { Self { x, y, width, height } }
    pub fn contains(&self, x: i32, y: i32) -> bool { x >= self.x && x < self.x + self.width as i32 && y >= self.y && y < self.y + self.height as i32 }
    pub fn intersects(&self, other: &Rect) -> bool { !(self.x + self.width as i32 <= other.x || other.x + other.width as i32 <= self.x || self.y + self.height as i32 <= other.y || other.y + other.height as i32 <= self.y) }
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) { return None; }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let width = (self.x + self.width as i32).min(other.x + other.width as i32) - x;
        let height = (self.y + self.height as i32).min(other.y + other.height as i32) - y;
        if width <= 0 || height <= 0 { return None; }
        Some(Rect::new(x, y, width as u32, height as u32))
    }
}

#[derive(Debug)]
pub struct Texture { /* ... as before ... */
    pub id: u32, pub width: u32, pub height: u32, pub format: TextureFormat,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat { RGBA8, RGB8, BGRA8, A8 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode { None, Alpha, Additive, Multiply }
#[derive(Debug)]
pub struct RendererCapabilities { /* ... as before ... */
    pub max_texture_size: u32, pub supports_blend_modes: bool,
    pub supports_render_targets: bool, pub supports_shaders: bool,
}

pub struct Renderer {
    width: u32,
    height: u32,
    framebuffer_virt_addr: VirtAddr,
    framebuffer_ptr: *mut u32,
    framebuffer_size: usize,
    framebuffer_is_gpu_provided: bool,
    framebuffer_pitch_pixels: u32,
    clip_rect: Option<Rect>,
    blend_mode: BlendMode,
    gpu_accelerated: AtomicBool,
    capabilities: RendererCapabilities,
    textures: Mutex<Vec<Texture>>,
}

#[derive(Debug)]
pub enum RendererError { /* ... as before ... */
    InitializationFailed(String), InvalidParameters, TextureCreationFailed, DrawingFailed,
    MemoryError(KernelMemoryError),
}
impl From<KernelMemoryError> for RendererError { fn from(err: KernelMemoryError) -> Self { RendererError::MemoryError(err) } }

impl Renderer {
    pub fn new(width: u32, height: u32) -> Result<Self, RendererError> {
        if width == 0 || height == 0 { return Err(RendererError::InvalidParameters); }

        let mut gpu_hw_initialized = gpu::init().is_ok(); // Check if GPU hardware itself is okay
        if gpu_hw_initialized { log::info!("GPU hardware responded."); }
        else { log::warn!("GPU hardware did not respond or failed to init."); }

        let mut framebuffer_virt_addr_opt: Option<VirtAddr> = None;
        let mut framebuffer_ptr_opt: Option<*mut u32> = None;
        let mut framebuffer_size_val: usize = 0;
        let mut framebuffer_is_gpu_provided_val = false;
        let mut actual_pitch_bytes_val = width * 4; // Default to standard pitch

        if gpu_hw_initialized {
            match gpu::get_framebuffer(width, height) {
                Ok(gpu_fb_raw_ptr) => {
                    // MODIFIED: Check is_null on the raw pointer
                    if gpu_fb_raw_ptr != 0 {
                        framebuffer_ptr_opt = Some(gpu_fb_raw_ptr as *mut u32);
                        framebuffer_virt_addr_opt = Some(VirtAddr::new(gpu_fb_raw_ptr as u64));
                        framebuffer_size_val = (width * height * 4) as usize;
                        framebuffer_is_gpu_provided_val = true;
                        actual_pitch_bytes_val = gpu::get_framebuffer_pitch().unwrap_or(width * 4);
                        log::info!("Using GPU provided framebuffer at VAddr: {:?}", framebuffer_virt_addr_opt.unwrap());
                    } else {
                        log::warn!("GPU get_framebuffer returned null. Falling back to software framebuffer.");
                        // gpu_hw_initialized remains true, but we'll allocate FB
                        framebuffer_is_gpu_provided_val = false; // Ensure this is set for fallback
                    }
                }
                Err(e) => {
                    log::warn!("GPU get_framebuffer failed: {:?}. Falling back to software framebuffer.", e);
                    // gpu_hw_initialized remains true, but we'll allocate FB
                    framebuffer_is_gpu_provided_val = false; // Ensure this is set for fallback
                }
            }
        }

        // Fallback to software framebuffer if GPU didn't provide one
        if !framebuffer_is_gpu_provided_val {
            log::info!("Allocating software framebuffer ({}x{}).", width, height);
            framebuffer_size_val = (width * height * 4) as usize;

            // MODIFIED: Use MemoryProtection struct
            let protection = MemoryProtectionFlags {
                read: true,
                write: true,
                execute: false,
                user: false,
                cache: CacheType::WriteCombining, // Good for framebuffers
                memory_type: MemoryType::Video, // Or Normal
            };
            let allocated_mem = memory::alloc_virtual_backed_memory(
                framebuffer_size_val,
                protection,
                MemoryType::Video, // Or Normal
            ).map_err(|e| RendererError::InitializationFailed(format!("SW Framebuffer allocation failed: {:?}", e)))?;
            
            framebuffer_virt_addr_opt = Some(VirtAddr::from_ptr(allocated_mem.as_ptr()));
            framebuffer_ptr_opt = Some(allocated_mem.as_ptr() as *mut u32);
            // framebuffer_is_gpu_provided_val is already false
            actual_pitch_bytes_val = width * 4; // Standard pitch

            unsafe { core::ptr::write_bytes(framebuffer_ptr_opt.unwrap() as *mut u8, 0, framebuffer_size_val); }
            log::info!("Software framebuffer allocated at VAddr: {:?}", framebuffer_virt_addr_opt.unwrap());
        }

        let capabilities = if gpu_hw_initialized { // Capabilities based on GPU hardware init status
            RendererCapabilities { /* ... as before ... */
                max_texture_size: gpu::get_max_texture_size().unwrap_or(2048),
                supports_blend_modes: gpu::supports_feature(gpu::Feature::Blending).unwrap_or(false),
                supports_render_targets: gpu::supports_feature(gpu::Feature::RenderTargets).unwrap_or(false),
                supports_shaders: gpu::supports_feature(gpu::Feature::Shaders).unwrap_or(false),
            }
        } else {
            RendererCapabilities {
                 max_texture_size: 2048, supports_blend_modes: true, // SW can do alpha blending
                 supports_render_targets: false, supports_shaders: false,
            }
        };

        Ok(Self {
            width,
            height,
            framebuffer_virt_addr: framebuffer_virt_addr_opt.ok_or(RendererError::InitializationFailed("Framebuffer VA not set".into()))?,
            framebuffer_ptr: framebuffer_ptr_opt.ok_or(RendererError::InitializationFailed("Framebuffer PTR not set".into()))?,
            framebuffer_size: framebuffer_size_val,
            framebuffer_is_gpu_provided: framebuffer_is_gpu_provided_val,
            framebuffer_pitch_pixels: actual_pitch_bytes_val / 4,
            clip_rect: None,
            blend_mode: BlendMode::Alpha,
            gpu_accelerated: AtomicBool::new(gpu_hw_initialized && framebuffer_is_gpu_provided_val), // True acceleration if GPU provides FB
            capabilities,
            textures: Mutex::new(Vec::new()),
        })
    }

    pub fn clear(&mut self, color: Color) { /* ... as in previous corrected version ... */
        if self.gpu_accelerated.load(Ordering::Relaxed) { // Check gpu_accelerated which considers if GPU FB is used
            if gpu::clear(color.to_argb()).is_err() {
                self.clear_software(color);
            }
        } else {
            self.clear_software(color);
        }
    }

    fn clear_software(&self, color: Color) { /* ... as in previous corrected version ... */
        let color_value = color.to_argb();
        let pixel_count = self.width * self.height;
        unsafe {
            for i in 0..pixel_count { // This simple loop is safer than assuming row-major for generic pitch
                let y = i / self.width;
                let x = i % self.width;
                let offset = y as usize * self.framebuffer_pitch_pixels as usize + x as usize;
                if offset < (self.framebuffer_size / 4) { // Bounds check
                    *self.framebuffer_ptr.add(offset) = color_value;
                }
            }
        }
    }
    
    // MODIFIED: fill_rect to handle Option from get_draw_rect and return ()
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if let Some(draw_rect) = self.get_draw_rect(rect) { // Handle Option
            if self.gpu_accelerated.load(Ordering::Relaxed) {
                 if gpu::fill_rect(draw_rect.x, draw_rect.y, draw_rect.width, draw_rect.height, color.to_argb()).is_err() {
                    self.fill_rect_software(draw_rect, color);
                }
            } else {
                self.fill_rect_software(draw_rect, color);
            }
        }
        // Implicitly returns ()
    }

    fn get_draw_rect(&self, rect: Rect) -> Option<Rect> { /* ... as in previous corrected version ... */
        let mut final_rect = rect;
        final_rect.x = final_rect.x.max(0);
        final_rect.y = final_rect.y.max(0);
        if final_rect.x >= self.width as i32 || final_rect.y >= self.height as i32 { return None; }
        
        let end_x = (final_rect.x + final_rect.width as i32).min(self.width as i32);
        let end_y = (final_rect.y + final_rect.height as i32).min(self.height as i32);

        final_rect.width = (end_x - final_rect.x).max(0) as u32;
        final_rect.height = (end_y - final_rect.y).max(0) as u32;

        if final_rect.width == 0 || final_rect.height == 0 { return None; }

        if let Some(clip) = self.clip_rect {
            clip.intersection(&final_rect)
        } else {
            Some(final_rect)
        }
    }

    fn fill_rect_software(&self, rect: Rect, color: Color) { /* ... as in previous corrected version, ensure use of self.framebuffer_pitch_pixels ... */
        let color_value = color.to_argb();
        let start_x = rect.x as usize;
        let start_y = rect.y as usize;
        let end_x = (rect.x + rect.width as i32) as usize;
        let end_y = (rect.y + rect.height as i32) as usize;

        unsafe {
            for y_idx in start_y..end_y {
                for x_idx in start_x..end_x {
                    let offset = y_idx * self.framebuffer_pitch_pixels as usize + x_idx;
                    if offset < (self.framebuffer_size / 4) { // Bounds check
                        let dst_pixel_ptr = self.framebuffer_ptr.add(offset);
                        if color.a == 255 {
                            *dst_pixel_ptr = color_value;
                        } else if color.a > 0 {
                            let dst_color_val = *dst_pixel_ptr;
                            let dst_c = self.unpack_color(dst_color_val);
                            *dst_pixel_ptr = self.blend_colors(color, dst_c).to_argb();
                        }
                    }
                }
            }
        }
    }
    
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) { /* ... as in previous corrected version ... */
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        if let Some(clip) = self.clip_rect { if !clip.contains(x,y) { return; } }
        unsafe {
            let offset = y as usize * self.framebuffer_pitch_pixels as usize + x as usize;
            if offset < (self.framebuffer_size / 4) { // Bounds check
                let pixel_ptr = self.framebuffer_ptr.add(offset);
                if color.a == 255 { *pixel_ptr = color.to_argb(); }
                else if color.a > 0 {
                    let dst_c = self.unpack_color(*pixel_ptr);
                    *pixel_ptr = self.blend_colors(color, dst_c).to_argb();
                }
            }
        }
    }
    
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) { /* ... as in previous corrected version ... */
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            if gpu::draw_line(x1, y1, x2, y2, color.to_argb()).is_err() {
                self.draw_line_software(x1, y1, x2, y2, color);
            }
        } else {
            self.draw_line_software(x1, y1, x2, y2, color);
        }
    }
    
    fn draw_line_software(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) { /* ... as in previous corrected version ... */
        let mut x = x1; let mut y = y1;
        let dx = (x2 - x1).abs(); let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 }; let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2; let mut err2;
        loop {
            self.draw_pixel(x, y, color);
            if x == x2 && y == y2 { break; }
            err2 = err;
            if err2 > -dx { err -= dy; x += sx; }
            if err2 < dy { err += dx; y += sy; }
        }
    }
    
    // This method was missing from the impl block in the previous version based on errors
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.draw_line(rect.x, rect.y, rect.x + rect.width as i32 - 1, rect.y, color);
        self.draw_line(rect.x + rect.width as i32 - 1, rect.y, rect.x + rect.width as i32 - 1, rect.y + rect.height as i32 - 1, color);
        self.draw_line(rect.x, rect.y + rect.height as i32 - 1, rect.x + rect.width as i32 - 1, rect.y + rect.height as i32 - 1, color);
        self.draw_line(rect.x, rect.y, rect.x, rect.y + rect.height as i32 - 1, color);
    }
    
    pub fn draw_texture(&mut self, texture_id: u32, dst_rect: Rect) -> Result<(), RendererError> { /* ... as in previous corrected version ... */
        // Ensure to use self.get_draw_rect for dst_rect clipping
        let final_dst_rect = match self.get_draw_rect(dst_rect) {
            Some(r) => r,
            None => return Ok(()), // Clipped out
        };

        if self.gpu_accelerated.load(Ordering::Relaxed) {
            if gpu::draw_texture(texture_id, final_dst_rect.x, final_dst_rect.y, final_dst_rect.width, final_dst_rect.height).is_ok() {
                return Ok(());
            }
        }
        // Software fallback for draw_texture needs careful implementation
        // to get texture data (which might be on GPU) and draw it pixel by pixel.
        // This is complex if data isn't easily readable.
        log::warn!("Software draw_texture fallback is complex and may be slow/incomplete.");
        let texture_info = {
            let textures = self.textures.lock();
            textures.iter().find(|t| t.id == texture_id)
                .map(|t| (t.width, t.height, t.format, t.id /* pass id for get_texture_data */))
                .ok_or(RendererError::InvalidParameters)?
        };
        let (tex_w, tex_h, tex_fmt, tex_id_for_data) = texture_info;
        if tex_w == 0 || tex_h == 0 { return Err(RendererError::InvalidParameters); }

        // This assumes gpu::get_texture_data can retrieve data even if other ops failed.
        let tex_data = gpu::get_texture_data(tex_id_for_data).map_err(|_| RendererError::DrawingFailed)?;

        for y_dst_rel in 0..final_dst_rect.height {
            let y_dst_abs = final_dst_rect.y + y_dst_rel as i32;
            let y_src = (y_dst_rel as f32 / final_dst_rect.height as f32 * tex_h as f32) as u32;
            if y_src >= tex_h { continue; }

            for x_dst_rel in 0..final_dst_rect.width {
                let x_dst_abs = final_dst_rect.x + x_dst_rel as i32;
                let x_src = (x_dst_rel as f32 / final_dst_rect.width as f32 * tex_w as f32) as u32;
                if x_src >= tex_w { continue; }

                let pixel_index = (y_src * tex_w + x_src) as usize;
                let src_color = match tex_fmt { /* ... unpack pixel from tex_data ... */
                    TextureFormat::RGBA8 => Color::new(tex_data[pixel_index*4], tex_data[pixel_index*4+1], tex_data[pixel_index*4+2], tex_data[pixel_index*4+3]),
                    // Add other formats
                    _ => Color::TRANSPARENT,
                };
                self.draw_pixel(x_dst_abs, y_dst_abs, src_color);
            }
        }
        Ok(())
    }
    
    pub fn create_texture(&self, width: u32, height: u32, format: TextureFormat, data: &[u8]) -> Result<u32, RendererError> { /* ... as in previous corrected version ... */
        if width == 0 || height == 0 || width > self.capabilities.max_texture_size || height > self.capabilities.max_texture_size {
            return Err(RendererError::InvalidParameters);
        }
        // ... (data size check)
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            let texture_id = gpu::create_texture(width, height, format as u32, data)
                .map_err(|_| RendererError::TextureCreationFailed)?;
            self.textures.lock().push(Texture { id: texture_id, width, height, format });
            Ok(texture_id)
        } else {
            Err(RendererError::TextureCreationFailed)
        }
    }
    
    pub fn destroy_texture(&self, texture_id: u32) -> Result<(), RendererError> { /* ... as in previous corrected version ... */
        let mut textures = self.textures.lock();
        let index = textures.iter().position(|t| t.id == texture_id).ok_or(RendererError::InvalidParameters)?;
        textures.remove(index);
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            gpu::destroy_texture(texture_id).map_err(|_| RendererError::InvalidParameters)?;
        }
        Ok(())
    }
    
    // This method was missing from the impl block in the previous version based on errors
    pub fn set_clip_rect(&mut self, rect: Option<Rect>) {
        self.clip_rect = rect.and_then(|r| { // Use and_then for cleaner chaining
            let x = r.x.max(0).min(self.width as i32);
            let y = r.y.max(0).min(self.height as i32);
            let end_x = (r.x + r.width as i32).min(self.width as i32);
            let end_y = (r.y + r.height as i32).min(self.height as i32);
            let width = (end_x - x).max(0) as u32;
            let height = (end_y - y).max(0) as u32;
            if width == 0 || height == 0 { None }
            else { Some(Rect::new(x, y, width, height)) }
        });

        if self.gpu_accelerated.load(Ordering::Relaxed) {
            if let Some(r) = self.clip_rect {
                let _ = gpu::set_clip_rect(r.x, r.y, r.width, r.height);
            } else {
                let _ = gpu::clear_clip_rect();
            }
        }
    }
    
    pub fn set_blend_mode(&mut self, mode: BlendMode) { /* ... as in previous corrected version ... */
        self.blend_mode = mode;
        if self.gpu_accelerated.load(Ordering::Relaxed) && self.capabilities.supports_blend_modes {
            let _ = gpu::set_blend_mode(mode as u32);
        }
    }
    
    pub fn present(&self) -> Result<(), RendererError> { /* ... as in previous corrected version ... */
        if self.gpu_accelerated.load(Ordering::Relaxed) {
            gpu::present().map_err(|_| RendererError::DrawingFailed)?;
        } else {
            log::trace!("Software present called (typically a no-op if direct drawing to screen buffer).");
        }
        Ok(())
    }
    
    pub fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }
    pub fn is_accelerated(&self) -> bool { self.gpu_accelerated.load(Ordering::Relaxed) }
    pub fn capabilities(&self) -> &RendererCapabilities { &self.capabilities }
    
    fn unpack_color(&self, argb_val: u32) -> Color { /* ... as in previous corrected version (assuming ARGB) ... */
        Color {a: (argb_val >> 24) as u8, r: (argb_val >> 16) as u8, g: (argb_val >> 8) as u8, b: argb_val as u8 }
    }
    fn blend_colors(&self, src: Color, dst: Color) -> Color { /* ... as in previous corrected version ... */
        match self.blend_mode {
            BlendMode::None => src,
            BlendMode::Alpha => {
                if src.a == 0 { return dst; } if src.a == 255 { return src; }
                let sa = src.a as f32 / 255.0; let da = dst.a as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a < 0.001 { return Color::TRANSPARENT; }
                let r = ((src.r as f32 * sa + dst.r as f32 * da * (1.0 - sa)) / out_a).round() as u8;
                let g = ((src.g as f32 * sa + dst.g as f32 * da * (1.0 - sa)) / out_a).round() as u8;
                let b = ((src.b as f32 * sa + dst.b as f32 * da * (1.0 - sa)) / out_a).round() as u8;
                Color::new(r, g, b, (out_a * 255.0).round() as u8)
            },
            BlendMode::Additive => Color::rgb(src.r.saturating_add(dst.r), src.g.saturating_add(dst.g), src.b.saturating_add(dst.b)),
            BlendMode::Multiply => Color::rgb(((src.r as u16 * dst.r as u16) / 255) as u8, ((src.g as u16 * dst.g as u16) / 255) as u8, ((src.b as u16 * dst.b as u16) / 255) as u8),
        }
    }
}

impl Drop for Renderer { /* ... as in previous corrected version, ensure memory::free_virtual_backed_memory is used ... */
    fn drop(&mut self) {
        log::info!("Dropping Renderer resources.");
        let textures_guard = self.textures.lock();
        for texture in textures_guard.iter() {
            if self.gpu_accelerated.load(Ordering::Relaxed) {
                if let Err(e) = gpu::destroy_texture(texture.id) { log::warn!("Failed to destroy GPU texture {}: {:?}", texture.id, e); }
            }
        }
        drop(textures_guard);

        if !self.framebuffer_is_gpu_provided && !self.framebuffer_ptr.is_null() {
            log::info!("Deallocating software framebuffer at VAddr: {:?}", self.framebuffer_virt_addr);
            if let Some(non_null_ptr) = core::ptr::NonNull::new(self.framebuffer_ptr as *mut u8) {
                match memory::free_virtual_backed_memory(non_null_ptr, self.framebuffer_size) {
                    Ok(_) => log::info!("Software framebuffer deallocated."),
                    Err(e) => log::error!("Failed to deallocate software framebuffer: {:?}", e),
                }
            }
        }
    }
}
