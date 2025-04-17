//! GPU device interface and vendor-specific implementations
//!
//! This module defines the common interface that all GPU implementations must implement
//! and provides vendor-specific drivers.
extern crate alloc;
use alloc::boxed::Box;
use crate::kernel::drivers::gpu::pci::PciDevice;
use crate::kernel::drivers::gpu::{GpuInfo, GpuError, DisplayMode, TextureFormat};

/// Interface for GPU device drivers
pub trait GpuDevice: Send + Sync {
    /// Get information about the GPU
    fn get_info(&self) -> Result<GpuInfo, GpuError>;
    
    /// Get the framebuffer address
    fn get_framebuffer(&mut self, width: u32, height: u32) -> Result<usize, GpuError>;
    
    /// Get the framebuffer pitch
    fn get_framebuffer_pitch(&self) -> Result<u32, GpuError>;
    
    /// Clear the screen with the specified color
    fn clear(&mut self, color: u32) -> Result<(), GpuError>;
    
    /// Draw a filled rectangle
    fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) -> Result<(), GpuError>;
    
    /// Draw a line
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) -> Result<(), GpuError>;
    
    /// Create a texture
    fn create_texture(&mut self, width: u32, height: u32, format: u32, data: &[u8]) -> Result<u32, GpuError>;
    
    /// Destroy a texture
    fn destroy_texture(&mut self, texture_id: u32) -> Result<(), GpuError>;
    
    /// Get texture data
    fn get_texture_data(&self, texture_id: u32) -> Result<&[u8], GpuError>;
    
    /// Draw a texture
    fn draw_texture(&mut self, texture_id: u32, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError>;
    
    /// Set clipping rectangle
    fn set_clip_rect(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError>;
    
    /// Clear clipping rectangle
    fn clear_clip_rect(&mut self) -> Result<(), GpuError>;
    
    /// Set blend mode
    fn set_blend_mode(&mut self, mode: u32) -> Result<(), GpuError>;
    
    /// Present the frame to the screen
    fn present(&mut self) -> Result<(), GpuError>;
    
    /// Shut down the GPU
    fn shutdown(&mut self) -> Result<(), GpuError>;
}

// Re-export vendor-specific modules
pub mod intel;
pub mod amd;
pub mod nvidia;