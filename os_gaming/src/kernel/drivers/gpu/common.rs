use core::{fmt, result};


/// Possible errors that can occur during GPU operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuError {
    /// The GPU device was not found or couldn't be initialized
    DeviceNotFound,
    /// The requested operation is not supported by this GPU
    NotSupported,
    /// The GPU command failed to execute
    CommandFailed,
    /// Invalid parameter was passed to a GPU function
    InvalidParameter,
    /// The GPU is out of memory
    OutOfMemory,
    /// The operation timed out
    Timeout,
    /// Any other error
    Unknown,

    InitializationFailed,
    DriverNotFound,
    MemoryAllocationFailed,
    InvalidHandle,
    KernelLaunchFailed,
    UnsupportedOperation,
    NoDevice,
    DeviceNotReady,
    DeviceLost,
    InvalidValue,
    UnknownError,
    NotInitialized,
    NotImplemented,
    InvalidParameters,
    OperationFailed,
    UnsupportedDevice
}

pub enum GpuDevice {
    /// Represents a generic GPU device
    Generic,
    /// Represents a specific GPU device (e.g., NVIDIA, AMD)
    Specific,
}
impl GpuDevice {
    /// Get the device type
    pub fn device_type(&self) -> &'static str {
        match self {
            GpuDevice::Generic => "Generic GPU",
            GpuDevice::Specific => "Specific GPU",
        }
    }
}

pub enum GpuOperation   {
    /// Render a frame
    Render(Vec<u8>),
    /// Run a compute kernel
    Compute(Vec<u32>),
    /// Any other operation
    Other,
}

impl fmt::Display for GpuDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.device_type())
    }
}
impl fmt::Display for GpuOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuOperation::Render(_) => write!(f, "Render operation"),
            GpuOperation::Compute(_) => write!(f, "Compute operation"),
            GpuOperation::Other => write!(f, "Other operation"),
        }
    }
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuError::DeviceNotFound => write!(f, "GPU device not found"),
            GpuError::NotSupported => write!(f, "Operation not supported"),
            GpuError::CommandFailed => write!(f, "GPU command failed"),
            GpuError::InvalidParameter => write!(f, "Invalid parameter"),
            GpuError::OutOfMemory => write!(f, "GPU out of memory"),
            GpuError::Timeout => write!(f, "Operation timed out"),
            GpuError::Unknown => write!(f, "Unknown GPU error"),
            GpuError::InitializationFailed => write!(f, "GPU initialization failed"),
            GpuError::DriverNotFound => write!(f, "GPU driver not found"),
            GpuError::MemoryAllocationFailed => write!(f, "GPU memory allocation failed"),
            GpuError::InvalidHandle => write!(f, "Invalid GPU handle"),
            GpuError::KernelLaunchFailed => write!(f, "GPU kernel launch failed"),
            GpuError::UnsupportedOperation => write!(f, "Unsupported GPU operation"),
            GpuError::NoDevice => write!(f, "No GPU device found"),
            GpuError::DeviceNotReady => write!(f, "GPU device not ready"),
            GpuError::DeviceLost => write!(f, "GPU device lost"),
            GpuError::InvalidValue => write!(f, "Invalid value"),
            GpuError::UnknownError => write!(f, "Unknown error"),
            GpuError::NotInitialized => write!(f, "GPU not initialized"),
            GpuError::NotImplemented => write!(f, "GPU operation not implemented"),
            GpuError::InvalidParameters => write!(f, "Invalid parameters"),
            GpuError::OperationFailed => write!(f, "GPU operation failed"),
            GpuError::UnsupportedDevice => write!(f, "Unsupported GPU device"),
        }
    }
}

/// Result type for GPU operations
pub type Result<T> = result::Result<T, GpuError>;

/// Basic information about a GPU device
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Name or model of the GPU
    pub name: &'static str,
    /// Vendor name or ID
    pub vendor: &'static str,
    /// Available VRAM in bytes
    pub vram: usize,
}

/// Represents a framebuffer or a portion of video memory
#[derive(Debug)]
pub struct Framebuffer {
    /// Pointer to the framebuffer memory
    pub address: *mut u8,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bytes per pixel
    pub bpp: u8,
    /// Pitch (bytes per row)
    pub pitch: u32,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

/// Interface that all GPU drivers must implement
pub trait GpuDriver {
    /// Initialize the GPU
    fn init(&mut self) -> Result<()>;
    
    /// Get information about the GPU
    fn get_info(&self) -> Result<GpuInfo>;
    
    /// Get the primary framebuffer for display
    fn get_framebuffer(&mut self) -> Result<&mut Framebuffer>;
    
    /// Set a specific video mode
    fn set_mode(&mut self, width: u32, height: u32, bpp: u8) -> Result<()>;
    
    /// Clear the screen with a specific color (RGB format)
    fn clear_screen(&mut self, r: u8, g: u8, b: u8) -> Result<()>;
    
    /// Flush any pending changes to the display
    fn flush(&mut self) -> Result<()>;
    
    /// Shut down the GPU driver
    fn shutdown(&mut self) -> Result<()>;
}

/// GPU capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuCapabilities {
    /// Supports 2D hardware acceleration
    pub hw_accel_2d: bool,
    /// Supports 3D hardware acceleration
    pub hw_accel_3d: bool,
    /// Maximum supported resolution width
    pub max_width: u32,
    /// Maximum supported resolution height
    pub max_height: u32,
    /// Supported color depths
    pub supported_bpp: &'static [u8],
}