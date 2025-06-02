//! GPU/Hardware accelerated rendering system
//!
//! This module provides hardware-accelerated rendering capabilities for the GUI system.
//! It abstracts away the details of graphics hardware and provides a clean API for
//! drawing primitives and managing textures.

extern crate alloc;
use alloc::{vec, vec::Vec, format, string::String};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use serde::{Serialize, Deserialize};

// Assuming these are correctly pathed from your project structure
use crate::kernel::drivers::gpu;
use crate::kernel::memory;
use crate::kernel::memory::memory_manager::{
    MemoryError as KernelMemoryError,
    MemoryProtectionFlags,
    CacheType,
    MemoryType,
};

use x86_64::VirtAddr;
use micromath::F32Ext; // For f32.round()

/// Color in RGBA format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self { 
        Self { r, g, b, a } 
    }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { 
        Self { r, g, b, a: 255 } 
    }
    pub fn to_rgba(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }
    pub fn to_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self   = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self  = Self::rgb(0, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const UI_BACKGROUND: Self = Self::rgb(45, 45, 48);
    pub const UI_FOREGROUND: Self = Self::rgb(200, 200, 200);
    pub const UI_ACCENT: Self = Self::rgb(0, 120, 215);
}

/// Represents a rectangular area
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self { 
        Self { x, y, width, height } 
    }

    pub fn contains(&self, x_test: i32, y_test: i32) -> bool {
        x_test >= self.x && x_test < self.x + self.width as i32 &&
        y_test >= self.y && y_test < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.x + self.width as i32 <= other.x ||
          other.x + other.width as i32 <= self.x ||
          self.y + self.height as i32 <= other.y ||
          other.y + other.height as i32 <= self.y)
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) { 
            return None; 
        }
        let x_intersect = self.x.max(other.x);
        let y_intersect = self.y.max(other.y);
        let width_intersect = (self.x + self.width as i32).min(other.x + other.width as i32) - x_intersect;
        let height_intersect = (self.y + self.height as i32).min(other.y + other.height as i32) - y_intersect;
        if width_intersect <= 0 || height_intersect <= 0 { 
            return None; 
        }
        Some(Rect::new(x_intersect, y_intersect, width_intersect as u32, height_intersect as u32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureFormat { 
    RGBA8, 
    RGB8, 
    BGRA8, 
    A8 
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode { 
    None, 
    Alpha, 
    Additive, 
    Multiply 
}

#[derive(Debug)]
pub struct Texture {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    // Optionally, if textures can be software-backed too:
    // pub data: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct RendererCapabilities {
    pub max_texture_size: (u32, u32), // (width, height)
    pub supports_blend_modes: Vec<BlendMode>,
    pub supports_render_targets: bool,
    pub supports_shaders: bool,
    pub supported_texture_formats: Vec<TextureFormat>,
}

pub struct Renderer {
    width: u32,
    height: u32,
    framebuffer_virt_addr: VirtAddr, // Virtual address of the framebuffer
    framebuffer_ptr: *mut u32,       // Raw pointer to the framebuffer (for direct pixel manipulation)
    framebuffer_size_bytes: usize,   // Total size of the framebuffer in bytes
    framebuffer_pitch_bytes: u32,    // Pitch (stride) of the framebuffer in bytes

    // True if GPU provided the framebuffer
    using_gpu_provided_framebuffer: bool,

    clip_rect: Option<Rect>,
    current_blend_mode: BlendMode,
    is_gpu_hardware_available: bool, // Indicates if any GPU hardware acceleration is available
    capabilities: RendererCapabilities,
    textures: Mutex<Vec<Texture>>,  // Manages textures, could be GPU or CPU backed
}

#[derive(Debug)]
pub enum RendererError {
    InitializationFailed(String),
    InvalidParameters(String),
    TextureCreationFailed(String),
    DrawingFailed(String),
    UnsupportedFeature(String),
    ResourceNotFound(String),
    MemoryError(KernelMemoryError),
}

impl From<KernelMemoryError> for RendererError {
    fn from(err: KernelMemoryError) -> Self {
        RendererError::MemoryError(err)
    }
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Result<Self, RendererError> {
        if width == 0 || height == 0 {
            return Err(RendererError::InvalidParameters("Width and height must be non-zero.".into()));
        }

        let is_gpu_hw_available = gpu::init().is_ok();
        if is_gpu_hw_available {
            log::info!("GPU Hardware: Initialization reported OK.");
        } else {
            log::warn!("GPU Hardware: Initialization reported failure or no GPU driver active.");
        }

        // initialize with defaults so they are always assigned
        let mut fb_virt_addr = VirtAddr::new(0);
        let mut fb_ptr: *mut u32 = core::ptr::null_mut();
        let mut fb_size_bytes: usize = 0;
        let mut fb_pitch_bytes: u32 = 0;
        let mut using_gpu_fb = false;

        // Try to get GPU framebuffer first
        if is_gpu_hw_available {
            match gpu::get_framebuffer(width, height) {
                Ok(gpu_raw_ptr) if gpu_raw_ptr != 0 => {
                    fb_ptr = gpu_raw_ptr as *mut u32;
                    fb_virt_addr = VirtAddr::new(gpu_raw_ptr as u64);
                    fb_size_bytes = (width * height * 4) as usize;
                    fb_pitch_bytes = gpu::get_framebuffer_pitch().unwrap_or(width * 4);
                    using_gpu_fb = true;
                    log::info!(
                        "Renderer: Using GPU-provided framebuffer at VAddr: {:?}, Pitch: {} bytes",
                        fb_virt_addr, fb_pitch_bytes
                    );
                }
                Ok(_) => {
                    log::warn!("Renderer: gpu::get_framebuffer returned a NULL pointer. Falling back to software framebuffer.");
                }
                Err(e) => {
                    log::warn!("Renderer: gpu::get_framebuffer failed: {:?}. Falling back to software framebuffer.", e);
                }
            }
        }

        // Fallback to software framebuffer if GPU didn't provide one
        if !using_gpu_fb {
            log::info!("Renderer: Allocating software framebuffer ({}x{}, 32bpp).", width, height);
            fb_size_bytes = (width * height * 4) as usize;
            fb_pitch_bytes = width * 4;

            let protection = MemoryProtectionFlags {
                read: true,
                write: true,
                execute: false,
                user: false,
                cache: CacheType::WriteCombining,
                memory_type: MemoryType::Video,
            };

            let allocated_mem_result = memory::alloc_virtual_backed_memory(
                fb_size_bytes,
                protection,
                MemoryType::Video,
            );

            match allocated_mem_result {
                Ok(allocated_mem_non_null) => {
                    fb_ptr = allocated_mem_non_null.as_ptr() as *mut u32;
                    fb_virt_addr = VirtAddr::from_ptr(fb_ptr);
                    // Zero out the framebuffer
                    unsafe { core::ptr::write_bytes(fb_ptr as *mut u8, 0, fb_size_bytes); }
                    log::info!("Renderer: Software framebuffer allocated at VAddr: {:?}", fb_virt_addr);
                }
                Err(e) => {
                    return Err(RendererError::InitializationFailed(format!(
                        "Software framebuffer allocation failed: {:?}",
                        e
                    )));
                }
            }
        }

        // Determine capabilities
        let capabilities = if is_gpu_hw_available {
            RendererCapabilities {
                max_texture_size: {
                    let size = gpu::get_max_texture_size().unwrap_or(2048);
                    (size, size)
                },
                supports_blend_modes: vec![BlendMode::None, BlendMode::Alpha],
                supports_render_targets: gpu::supports_feature(gpu::Feature::RenderTargets).unwrap_or(false),
                supports_shaders: gpu::supports_feature(gpu::Feature::Shaders).unwrap_or(false),
                supported_texture_formats: vec![TextureFormat::RGBA8, TextureFormat::BGRA8],
            }
        } else {
            RendererCapabilities {
                max_texture_size: (2048, 2048),
                supports_blend_modes: vec![BlendMode::None, BlendMode::Alpha],
                supports_render_targets: false,
                supports_shaders: false,
                supported_texture_formats: vec![TextureFormat::RGBA8, TextureFormat::BGRA8],
            }
        };

        Ok(Self {
            width,
            height,
            framebuffer_virt_addr: fb_virt_addr,
            framebuffer_ptr: fb_ptr,
            framebuffer_size_bytes: fb_size_bytes,
            framebuffer_pitch_bytes: fb_pitch_bytes,
            using_gpu_provided_framebuffer: using_gpu_fb,
            clip_rect: None,
            current_blend_mode: BlendMode::Alpha,
            is_gpu_hardware_available: is_gpu_hw_available,
            capabilities,
            textures: Mutex::new(Vec::new()),
        })
    }

    #[inline(always)]
    fn get_pixel_offset(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width as usize && y < self.height as usize {
            Some(y * ((self.framebuffer_pitch_bytes / 4) as usize) + x)
        } else {
            None
        }
    }

    fn clear_software(&mut self, color: Color) {
        let color_value = color.to_argb();
        let pixel_height = self.height;
        let pixel_width = self.width;

        unsafe {
            for y_idx in 0..pixel_height {
                for x_idx in 0..pixel_width {
                    if let Some(offset) = self.get_pixel_offset(x_idx as usize, y_idx as usize) {
                        if offset < (self.framebuffer_size_bytes / 4) {
                            *self.framebuffer_ptr.add(offset) = color_value;
                        }
                    }
                }
            }
        }
    }

    pub fn clear(&mut self, color: Color) {
        if self.using_gpu_provided_framebuffer && self.is_gpu_hardware_available {
            if gpu::clear(color.to_argb()).is_err() {
                log::warn!("GPU clear failed, falling back to software clear.");
                self.clear_software(color);
            }
        } else {
            self.clear_software(color);
        }
    }

    fn fill_rect_software(&mut self, rect: Rect, color: Color) {
        let color_value = color.to_argb();
        let start_x = rect.x.max(0) as u32;
        let start_y = rect.y.max(0) as u32;
        let end_x = (rect.x + rect.width as i32).min(self.width as i32) as u32;
        let end_y = (rect.y + rect.height as i32).min(self.height as i32) as u32;

        unsafe {
            for y_idx in start_y..end_y {
                for x_idx in start_x..end_x {
                    if let Some(offset) = self.get_pixel_offset(x_idx as usize, y_idx as usize) {
                        if offset < (self.framebuffer_size_bytes / 4) {
                            let dst_pixel_ptr = self.framebuffer_ptr.add(offset);
                            if color.a == 255 || self.current_blend_mode == BlendMode::None {
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
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if let Some(draw_rect) = self.get_draw_rect(rect) {
            if draw_rect.width == 0 || draw_rect.height == 0 {
                return;
            }
            if self.using_gpu_provided_framebuffer && self.is_gpu_hardware_available {
                if gpu::fill_rect(draw_rect.x, draw_rect.y, draw_rect.width, draw_rect.height, color.to_argb())
                    .is_err()
                {
                    log::warn!("GPU fill_rect failed, falling back to software.");
                    self.fill_rect_software(draw_rect, color);
                }
            } else {
                self.fill_rect_software(draw_rect, color);
            }
        }
    }

    fn get_draw_rect(&self, rect: Rect) -> Option<Rect> {
        let mut final_rect = rect;
        final_rect.x = final_rect.x.max(0);
        final_rect.y = final_rect.y.max(0);
        if final_rect.x >= self.width as i32 || final_rect.y >= self.height as i32 {
            return None;
        }
        let end_x = (final_rect.x + final_rect.width as i32).min(self.width as i32);
        let end_y = (final_rect.y + final_rect.height as i32).min(self.height as i32);
        final_rect.width = (end_x - final_rect.x).max(0) as u32;
        final_rect.height = (end_y - final_rect.y).max(0) as u32;
        if final_rect.width == 0 || final_rect.height == 0 {
            return None;
        }
        if let Some(clip) = self.clip_rect {
            clip.intersection(&final_rect)
        } else {
            Some(final_rect)
        }
    }

    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(clip) = self.clip_rect {
            if !clip.contains(x, y) {
                return;
            }
        }
        if let Some(offset) = self.get_pixel_offset(x as usize, y as usize) {
            if offset < (self.framebuffer_size_bytes / 4) {
                unsafe {
                    let pixel_ptr = self.framebuffer_ptr.add(offset);
                    if color.a == 255 || self.current_blend_mode == BlendMode::None {
                        *pixel_ptr = color.to_argb();
                    } else if color.a > 0 {
                        let dst_c = self.unpack_color(*pixel_ptr);
                        *pixel_ptr = self.blend_colors(color, dst_c).to_argb();
                    }
                }
            }
        }
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        if self.using_gpu_provided_framebuffer && self.is_gpu_hardware_available {
            if gpu::draw_line(x1, y1, x2, y2, color.to_argb()).is_err() {
                self.draw_line_software(x1, y1, x2, y2, color);
            }
        } else {
            self.draw_line_software(x1, y1, x2, y2, color);
        }
    }

    fn draw_line_software(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        let mut x_curr = x1;
        let mut y_curr = y1;
        let dx_abs = (x2 - x1).abs();
        let dy_abs = (y2 - y1).abs();
        let sx_dir = if x1 < x2 { 1 } else { -1 };
        let sy_dir = if y1 < y2 { 1 } else { -1 };
        let mut err_term = if dx_abs > dy_abs { dx_abs } else { -dy_abs } / 2;
        let mut e2_term;
        loop {
            self.draw_pixel(x_curr, y_curr, color);
            if x_curr == x2 && y_curr == y2 {
                break;
            }
            e2_term = err_term;
            if e2_term > -dx_abs {
                err_term -= dy_abs;
                x_curr += sx_dir;
            }
            if e2_term < dy_abs {
                err_term += dx_abs;
                y_curr += sy_dir;
            }
        }
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        let x_end = rect.x + rect.width as i32 - 1;
        let y_end = rect.y + rect.height as i32 - 1;
        self.draw_line(rect.x, rect.y, x_end, rect.y, color);
        self.draw_line(x_end, rect.y, x_end, y_end, color);
        self.draw_line(rect.x, y_end, x_end, y_end, color);
        self.draw_line(rect.x, rect.y, rect.x, y_end, color);
    }

    pub fn draw_texture(&mut self, texture_id: u32, dst_rect: Rect) -> Result<(), RendererError> {
        let final_dst_rect = match self.get_draw_rect(dst_rect) {
            Some(r) => r,
            None => return Ok(()),
        };
        if final_dst_rect.width == 0 || final_dst_rect.height == 0 {
            return Ok(());
        }
        if self.is_gpu_hardware_available {
            if gpu::draw_texture(
                texture_id,
                final_dst_rect.x,
                final_dst_rect.y,
                final_dst_rect.width,
                final_dst_rect.height,
            ).is_ok() {
                return Ok(());
            } else {
                log::warn!("GPU draw_texture failed, attempting software fallback.");
            }
        }
        let texture_info = {
            let textures_guard = self.textures.lock();
            textures_guard
                .iter()
                .find(|t| t.id == texture_id)
                .map(|t| (t.width, t.height, t.format))
                .ok_or_else(|| RendererError::ResourceNotFound(format!("Texture ID {} not found", texture_id)))?
        };
        let (tex_w, tex_h, tex_fmt) = texture_info;
        if tex_w == 0 || tex_h == 0 {
            return Err(RendererError::InvalidParameters("Texture has zero dimension".into()));
        }
        if !self.is_gpu_hardware_available && !self.using_gpu_provided_framebuffer {
            log::error!("Software draw_texture: No GPU context to get texture data from for texture ID {}", texture_id);
            return Err(RendererError::DrawingFailed("Cannot get texture data for software rendering without GPU context".into()));
        }
        let tex_data_vec = gpu::get_texture_data(texture_id)
            .map_err(|e| RendererError::DrawingFailed(format!("Failed to get texture data for SW fallback: {:?}", e)))?;
        for y_dst_rel in 0..final_dst_rect.height {
            let y_dst_abs = final_dst_rect.y + y_dst_rel as i32;
            let y_src_f = y_dst_rel as f32 / final_dst_rect.height as f32;
            let y_src = (y_src_f * tex_h as f32).min((tex_h - 1) as f32) as u32;
            for x_dst_rel in 0..final_dst_rect.width {
                let x_dst_abs = final_dst_rect.x + x_dst_rel as i32;
                let x_src_f = x_dst_rel as f32 / final_dst_rect.width as f32;
                let x_src = (x_src_f * tex_w as f32).min((tex_w - 1) as f32) as u32;
                let pixel_index_src = (y_src * tex_w + x_src) as usize;
                let src_color = match tex_fmt {
                    TextureFormat::RGBA8 => {
                        if (pixel_index_src + 1) * 4 > tex_data_vec.len() { continue; }
                        Color::new(
                            tex_data_vec[pixel_index_src * 4],
                            tex_data_vec[pixel_index_src * 4 + 1],
                            tex_data_vec[pixel_index_src * 4 + 2],
                            tex_data_vec[pixel_index_src * 4 + 3],
                        )
                    }
                    TextureFormat::BGRA8 => {
                        if (pixel_index_src + 1) * 4 > tex_data_vec.len() { continue; }
                        Color::new(
                            tex_data_vec[pixel_index_src * 4 + 2],
                            tex_data_vec[pixel_index_src * 4 + 1],
                            tex_data_vec[pixel_index_src * 4 + 0],
                            tex_data_vec[pixel_index_src * 4 + 3],
                        )
                    }
                    _ => Color::TRANSPARENT,
                };
                self.draw_pixel(x_dst_abs, y_dst_abs, src_color);
            }
        }
        Ok(())
    }

    pub fn create_texture(&mut self, width: u32, height: u32, format: TextureFormat, data: &[u8]) -> Result<u32, RendererError> {
        if width == 0 || height == 0 || width > self.capabilities.max_texture_size.0 || height > self.capabilities.max_texture_size.1 {
            return Err(RendererError::InvalidParameters("Invalid texture dimensions".into()));
        }
        let expected_size = match format {
            TextureFormat::RGBA8 | TextureFormat::BGRA8 => width * height * 4,
            TextureFormat::RGB8 => width * height * 3,
            TextureFormat::A8 => width * height * 1,
        };
        if data.len() != expected_size as usize {
            return Err(RendererError::InvalidParameters(format!(
                "Data size mismatch for texture format. Expected {}, got {}",
                expected_size,
                data.len()
            )));
        }
        if self.is_gpu_hardware_available {
            let texture_id = gpu::create_texture(width, height, format as u32, data)
                .map_err(|e| RendererError::TextureCreationFailed(format!("GPU texture creation failed: {:?}", e)))?;
            self.textures.lock().push(Texture { id: texture_id, width, height, format });
            Ok(texture_id)
        } else {
            log::warn!("Renderer: GPU not available, create_texture only supports GPU textures currently.");
            Err(RendererError::UnsupportedFeature("Software texture creation not implemented.".into()))
        }
    }

    pub fn destroy_texture(&mut self, texture_id: u32) -> Result<(), RendererError> {
        let mut textures_guard = self.textures.lock();
        if let Some(index) = textures_guard.iter().position(|t| t.id == texture_id) {
            textures_guard.remove(index);
            if self.is_gpu_hardware_available {
                gpu::destroy_texture(texture_id)
                    .map_err(|e| RendererError::TextureCreationFailed(format!("GPU texture destruction failed: {:?}", e)))?;
            }
            Ok(())
        } else {
            Err(RendererError::ResourceNotFound(format!("Texture ID {} not found for destruction.", texture_id)))
        }
    }

    pub fn set_clip_rect(&mut self, rect_opt: Option<Rect>) {
        self.clip_rect = rect_opt.and_then(|r| {
            let screen_rect = Rect::new(0, 0, self.width, self.height);
            screen_rect.intersection(&r)
        });
        if self.is_gpu_hardware_available {
            if let Some(r) = self.clip_rect {
                let _ = gpu::set_clip_rect(r.x, r.y, r.width, r.height);
            } else {
                let _ = gpu::clear_clip_rect();
            }
        }
    }

    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.current_blend_mode = mode;
        if self.is_gpu_hardware_available && self.capabilities.supports_blend_modes.contains(&mode) {
            if gpu::set_blend_mode(mode as u32).is_err() {
                log::warn!("Failed to set GPU blend mode to {:?}, software blending will be used if applicable.", mode);
            }
        }
    }

    pub fn present(&mut self) -> Result<(), RendererError> {
        if self.using_gpu_provided_framebuffer && self.is_gpu_hardware_available {
            gpu::present().map_err(|e| RendererError::DrawingFailed(format!("GPU present failed: {:?}", e)))?;
        } else {
            log::trace!("Software present called (no-op for direct drawing).");
        }
        Ok(())
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn is_accelerated(&self) -> bool {
        self.using_gpu_provided_framebuffer && self.is_gpu_hardware_available
    }

    pub fn capabilities(&self) -> &RendererCapabilities {
        &self.capabilities
    }

    fn unpack_color(&self, argb_val: u32) -> Color {
        Color {
            a: (argb_val >> 24) as u8,
            r: (argb_val >> 16) as u8,
            g: (argb_val >> 8) as u8,
            b: argb_val as u8,
        }
    }

    fn blend_colors(&self, src: Color, dst: Color) -> Color {
        match self.current_blend_mode {
            BlendMode::None => src,
            BlendMode::Alpha => {
                if src.a == 0 {
                    return dst;
                }
                if src.a == 255 {
                    return src;
                }
                let sa_norm = src.a as f32 / 255.0;
                let da_norm = dst.a as f32 / 255.0;
                let out_a_norm = sa_norm + da_norm * (1.0 - sa_norm);
                if out_a_norm < 0.001 {
                    return Color::TRANSPARENT;
                }
                let r_blend = ((src.r as f32 * sa_norm + dst.r as f32 * da_norm * (1.0 - sa_norm)) / out_a_norm).round() as u8;
                let g_blend = ((src.g as f32 * sa_norm + dst.g as f32 * da_norm * (1.0 - sa_norm)) / out_a_norm).round() as u8;
                let b_blend = ((src.b as f32 * sa_norm + dst.b as f32 * da_norm * (1.0 - sa_norm)) / out_a_norm).round() as u8;
                Color::new(r_blend, g_blend, b_blend, (out_a_norm * 255.0).round() as u8)
            },
            BlendMode::Additive => Color::new(
                src.r.saturating_add(dst.r),
                src.g.saturating_add(dst.g),
                src.b.saturating_add(dst.b),
                255.max(src.a.saturating_add(dst.a))
            ),
            BlendMode::Multiply => Color::new(
                ((src.r as u16 * dst.r as u16) / 255) as u8,
                ((src.g as u16 * dst.g as u16) / 255) as u8,
                ((src.b as u16 * dst.b as u16) / 255) as u8,
                ((src.a as u16 * dst.a as u16) / 255) as u8
            ),
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Renderer: Dropping resources.");
        // Destroy textures
        let texture_ids: Vec<u32> = self.textures.lock().iter().map(|t| t.id).collect();
        for id in texture_ids {
            if self.is_gpu_hardware_available {
                if let Err(e) = gpu::destroy_texture(id) {
                    log::warn!("Renderer: Failed to destroy GPU texture {}: {:?}", id, e);
                }
            }
        }
        self.textures.lock().clear();
        // Deallocate software framebuffer if it was used
        if !self.using_gpu_provided_framebuffer && !self.framebuffer_ptr.is_null() {
            log::info!("Renderer: Deallocating software framebuffer at VAddr: {:?}", self.framebuffer_virt_addr);
            if let Some(non_null_ptr) = core::ptr::NonNull::new(self.framebuffer_ptr as *mut u8) {
                match memory::free_virtual_backed_memory(non_null_ptr, self.framebuffer_size_bytes) {
                    Ok(_) => log::info!("Renderer: Software framebuffer deallocated successfully."),
                    Err(e) => log::error!("Renderer: Failed to deallocate software framebuffer: {:?}", e),
                }
            } else {
                log::warn!("Renderer: Software framebuffer pointer was null, skipping deallocation.");
            }
        }
        // GPU driver might have its own cleanup if any.
        if self.is_gpu_hardware_available {
            gpu::shutdown();
        }
        log::info!("Renderer: Drop complete.");
    }
}