//! GPU driver subsystem for OS Gaming
//!
//! This module provides hardware-accelerated graphics capabilities
//! through a unified API regardless of the underlying GPU.

extern crate alloc;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use alloc::vec::Vec;

mod detection;
mod vesa;
mod pci;
mod command;
mod memory;
mod specific;
mod common;

use specific::GpuDevice;

/// GPU capabilities and information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// GPU vendor name
    pub vendor: &'static str,
    /// GPU device name
    pub device: &'static str,
    /// Total video memory in bytes
    pub vram_size: usize,
    /// Maximum texture dimensions
    pub max_texture_size: u32,
    /// Supported features bitmap
    pub features: u32,
    /// Current display mode
    pub current_mode: DisplayMode,
    /// Available display modes
    pub available_modes: &'static [DisplayMode],
}

/// Display mode information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayMode {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Refresh rate (Hz)
    pub refresh_rate: u16,
}

/// GPU feature flags
#[allow(dead_code)]
#[repr(u64)]  // Specify u64 representation to ensure values fit on all targets
pub enum Feature {
    /// Hardware blending support
    Blending = 0x01,
    /// Hardware acceleration for 2D operations
    Acceleration2D = 0x02,
    /// 3D rendering support
    Rendering3D = 0x04,
    /// Shader support
    Shaders = 0x08,
    /// Multiple render targets
    RenderTargets = 0x10,
    /// Hardware cursor
    HardwareCursor = 0x20,
    /// VRAM memory mapping
    MemoryMapping = 0x40,
    /// DMA transfers
    DmaTransfers = 0x80,
    /// Texture compression
    TextureCompression = 0x100,
    /// Hardware video decoding
    VariableRefreshRate = 0x200,
    /// Hardware video encoding
    VariableRefresh = 0x400,

    // Additional features
    TensorAcceleration = 0x800,
    RayTracing = 0x1000,
    VideoAcceleration = 0x2000,
    ComputeAcceleration = 0x4000,
    DisplayPort = 0x8000,
    HDMI = 0x10000,
    VSync = 0x20000,
    FreeSync = 0x40000,
    GSync = 0x80000,
    AdaptiveSync = 0x100000,
    VariableRateShading = 0x200000,
    MeshShading = 0x400000,
    SamplerFeedback = 0x800000,
    TextureFiltering = 0x1000000,
    TextureArray = 0x2000000,
    TextureAtlas = 0x4000000,

    ComputeShaders = 0x8000000,
    GeometryShaders = 0x10000000,
    TessellationShaders = 0x20000000,
    ComputeUnits = 0x40000000,
    RayTracingCores = 0x80000000,
    TensorCores = 0x100000000,
    VideoCodecs = 0x200000000,
}

/// GPU texture formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    RGBA8 = 0,
    RGB8 = 1,
    BGRA8 = 2,
    BGR8 = 3,
    A8 = 4,
}

/// GPU blend modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    None = 0,
    Alpha = 1,
    Additive = 2,
    Multiply = 3,
}

/// GPU errors
#[derive(Debug)]
pub enum GpuError {
    /// No compatible GPU found
    NoDevice,
    /// Device initialization failed
    InitializationFailed,
    /// Invalid parameter
    InvalidParameter,
    // Invalid device
    InvalidDevice,
    /// Unsupported feature
    UnsupportedFeature,
    /// Out of VRAM
    OutOfMemory,
    /// Invalid command
    InvalidCommand,
    /// Invalid texture
    InvalidTexture,
    /// Buffer mapping failed
    MappingFailed,
    NotSupported,
    NotInitialized,
    UnsupportedFormat,
    UnsupportedDevice,
    HardwareError,
    ShutdownFailed,
    TextureCreationFailed,
    SetModeFailed,
    DrawingFailed,
    CommunicationError,
    DisplayModeFailed,
    OperationFailed
}

// Global GPU device instance
static GPU_DEVICE: Mutex<Option<Box<dyn GpuDevice>>> = Mutex::new(None);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the GPU subsystem
pub fn init() -> Result<(), GpuError> {
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    // Detect available GPU hardware
    let device = detection::detect_gpu()
        .map_err(|_| GpuError::NoDevice)?;
    
    // Store the device
    let mut gpu_lock = GPU_DEVICE.lock();
    *gpu_lock = Some(device);
    
    // Mark as initialized
    INITIALIZED.store(true, Ordering::SeqCst);
    
    // Initialize VESA fallback if no hardware acceleration
    if !supports_feature(Feature::Acceleration2D)? {
        vesa::init()?;
    }
    
    // Set up default display mode
    Ok(())
}

/// Shut down the GPU subsystem
pub fn shutdown() -> Result<(), GpuError> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.shutdown()?;
    }
    
    *gpu_lock = None;
    INITIALIZED.store(false, Ordering::SeqCst);
    Ok(())
}

/// Get information about the GPU
pub fn get_info() -> Result<GpuInfo, GpuError> {
    ensure_initialized()?;
    
    let gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_ref() {
        device.get_info()
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Get the framebuffer address
pub fn get_framebuffer(width: u32, height: u32) -> Result<usize, GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.get_framebuffer(width, height)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Get the framebuffer pitch (bytes per row)
pub fn get_framebuffer_pitch() -> Result<u32, GpuError> {
    ensure_initialized()?;
    
    let gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_ref() {
        device.get_framebuffer_pitch()
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Clear the screen with the specified color
pub fn clear(color: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.clear(color)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Draw a rectangle
pub fn fill_rect(x: i32, y: i32, width: u32, height: u32, color: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.fill_rect(x, y, width, height, color)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Draw a line
pub fn draw_line(x1: i32, y1: i32, x2: i32, y2: i32, color: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.draw_line(x1, y1, x2, y2, color)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Create a texture
pub fn create_texture(width: u32, height: u32, format: u32, data: &[u8]) -> Result<u32, GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.create_texture(width, height, format, data)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Destroy a texture
pub fn destroy_texture(texture_id: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.destroy_texture(texture_id)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Get texture data
pub fn get_texture_data(texture_id: u32) -> Result<Vec<u8>, GpuError> {
    ensure_initialized()?;
    
    let gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_ref() {
        device.get_texture_data(texture_id).map(|data| data.to_vec())
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Draw a texture
pub fn draw_texture(texture_id: u32, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.draw_texture(texture_id, x, y, width, height)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Set clipping rectangle
pub fn set_clip_rect(x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.set_clip_rect(x, y, width, height)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Clear clipping rectangle
pub fn clear_clip_rect() -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.clear_clip_rect()
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Set blend mode
pub fn set_blend_mode(mode: u32) -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.set_blend_mode(mode)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Present the frame to the screen
pub fn present() -> Result<(), GpuError> {
    ensure_initialized()?;
    
    let mut gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_mut() {
        device.present()
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Check if a feature is supported
pub fn supports_feature(feature: Feature) -> Result<bool, GpuError> {
    ensure_initialized()?;
    
    let gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_ref() {
        let info = device.get_info()?;
        Ok((info.features & feature as u32) != 0)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Get maximum texture size
pub fn get_max_texture_size() -> Result<u32, GpuError> {
    ensure_initialized()?;
    
    let gpu_lock = GPU_DEVICE.lock();
    if let Some(device) = gpu_lock.as_ref() {
        let info = device.get_info()?;
        Ok(info.max_texture_size)
    } else {
        Err(GpuError::NoDevice)
    }
}

/// Ensure GPU is initialized
fn ensure_initialized() -> Result<(), GpuError> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        Err(GpuError::NoDevice)
    } else {
        Ok(())
    }
}