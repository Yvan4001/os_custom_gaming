use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
#[cfg(feature = "std")]
use sysinfo::{System};
#[cfg(feature = "std")]
use std::process::Command;

/// HDMI resolution configuration
#[derive(Debug, Clone, Copy)]
pub struct HdmiResolution {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

/// HDMI color format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HdmiColorFormat {
    RGB,
    YCbCr444,
    YCbCr422,
    YCbCr420,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    AMD,
    Intel,
    VMware,
    VirtualBox,
    Other(String),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model: String,
    pub vram_mb: Option<u32>,
    pub driver_version: Option<String>,
    pub supports_vulkan: bool,
    pub supports_opengl: bool,
    pub max_resolution: Option<HdmiResolution>,
    pub supports_hdr: bool,
    pub supports_variable_refresh: bool, // G-Sync, FreeSync, etc.
    pub supports_raytracing: bool,
}

/// HDMI driver state
pub struct HdmiDriver {
    initialized: AtomicBool,
    current_resolution: Option<HdmiResolution>,
    color_format: HdmiColorFormat,
    framebuffer: Option<*mut u8>,
    framebuffer_size: usize,
    detected_gpus: Vec<GpuInfo>,
    primary_gpu: Option<usize>, // Index into detected_gpus
}

#[derive(Debug, Clone)]
pub struct MonitorCapabilities {
    pub manufacturer: String,
    pub product_code: u16,
    pub serial: u32,
    pub manufacture_date: String,
    pub name: String,
    pub max_resolution: HdmiResolution,
    pub supports_hdr: bool,
    pub supports_freesync: bool,
    pub supports_gsync: bool,
}

#[derive(Debug, Clone)]
pub struct GamingRequirements {
    pub min_vram_mb: u32,
    pub requires_vulkan: bool,
    pub requires_opengl: bool,
    pub requires_raytracing: bool,
    pub min_width: u32,
    pub min_height: u32,
    pub min_refresh_rate: u32,
}

// Public interface for GPU detection
pub fn detect_graphics_hardware() -> Result<(), &'static str> {
    HDMI_DRIVER.lock().detect_graphics_hardware()
}

pub fn get_primary_gpu() -> Option<GpuInfo> {
    HDMI_DRIVER.lock().get_primary_gpu().cloned()
}

pub fn meets_gaming_requirements(requirements: &GamingRequirements) -> bool {
    HDMI_DRIVER.lock().meets_gaming_requirements(requirements)
}

pub fn set_resolution(resolution: HdmiResolution) -> Result<(), &'static str> {
    HDMI_DRIVER.lock().init_with_resolution(resolution)
}

// Explicitly implement Send and Sync for HdmiDriver
// This is safe because we ensure proper synchronization through AtomicBool and Mutex
unsafe impl Send for HdmiDriver {}
unsafe impl Sync for HdmiDriver {}

impl HdmiDriver {
    /// Create a new HDMI driver instance
    pub fn new() -> Self {
        HdmiDriver {
            initialized: AtomicBool::new(false),
            current_resolution: None,
            color_format: HdmiColorFormat::RGB,
            framebuffer: None,
            framebuffer_size: 0,
            detected_gpus: Vec::new(),
            primary_gpu: None,
        }
    }

    /// Initialize the HDMI driver with default resolution
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Default to 1080p if possible
        self.init_with_resolution(HdmiResolution {
            width: 1920,
            height: 1080,
            refresh_rate: 60,
        })
    }

    /// Initialize the HDMI driver with specific resolution
    pub fn init_with_resolution(&mut self, resolution: HdmiResolution) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Err("HDMI driver already initialized");
        }

        // Here you would typically:
        // 1. Detect HDMI presence
        // 2. Negotiate EDID information with the display
        // 3. Set up the hardware registers for the requested mode
        // 4. Allocate and map a framebuffer

        // For this example, we'll simulate setting up a framebuffer
        let bytes_per_pixel = 4; // RGBA
        let fb_size = resolution.width as usize * resolution.height as usize * bytes_per_pixel;

        // Allocate framebuffer memory - in a real implementation this would be mapped to
        // physical HDMI controller memory
        let framebuffer = self.allocate_framebuffer(fb_size)?;

        self.framebuffer = Some(framebuffer);
        self.framebuffer_size = fb_size;
        self.current_resolution = Some(resolution);

        // Mark as initialized
        self.initialized.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Allocate a framebuffer of the specified size
    fn allocate_framebuffer(&self, size: usize) -> Result<*mut u8, &'static str> {
        // In a real implementation, this would allocate DMA-able memory
        // and set up the mapping with the HDMI controller
        // For this example, we'll just create a placeholder

        // Safety: This is just a placeholder. In a real implementation,
        // you would properly allocate physical memory for the framebuffer.
        let mut buffer = Vec::with_capacity(size);
        let ptr = buffer.as_mut_ptr();
        core::mem::forget(buffer); // Prevent buffer from being deallocated
        Ok(ptr)
    }

    /// Clear the screen to a specific color
    pub fn clear_screen(&self, r: u8, g: u8, b: u8) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("HDMI driver not initialized");
        }

        let framebuffer = match self.framebuffer {
            Some(fb) => fb,
            None => return Err("Framebuffer not allocated"),
        };

        // Fill the framebuffer with the specified color
        unsafe {
            let fb_slice = slice::from_raw_parts_mut(framebuffer, self.framebuffer_size);

            for i in (0..self.framebuffer_size).step_by(4) {
                fb_slice[i] = r; // R
                fb_slice[i + 1] = g; // G
                fb_slice[i + 2] = b; // B
                fb_slice[i + 3] = 255; // A (fully opaque)
            }
        }

        Ok(())
    }

    /// Get current resolution
    pub fn resolution(&self) -> Option<HdmiResolution> {
        self.current_resolution
    }

    /// Set a pixel at the specified coordinates
    pub fn set_pixel(
        &self,
        x: u32,
        y: u32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    ) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("HDMI driver not initialized");
        }

        let resolution = match self.current_resolution {
            Some(res) => res,
            None => return Err("Resolution not set"),
        };

        if x >= resolution.width || y >= resolution.height {
            return Err("Coordinates out of bounds");
        }

        let framebuffer = match self.framebuffer {
            Some(fb) => fb,
            None => return Err("Framebuffer not allocated"),
        };

        // Calculate pixel offset
        let offset = ((y * resolution.width + x) * 4) as usize;

        // Safety: We've checked bounds above, so this should be safe
        unsafe {
            *framebuffer.add(offset) = r;
            *framebuffer.add(offset + 1) = g;
            *framebuffer.add(offset + 2) = b;
            *framebuffer.add(offset + 3) = a;
        }

        Ok(())
    }

    /// Flush changes to the screen
    pub fn flush(&self) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("HDMI driver not initialized");
        }

        // In a real implementation, this might trigger a DMA transfer or
        // update hardware registers to show the current framebuffer

        // Simulate syncing with hardware
        Ok(())
    }
    // Add this method to detect graphics cards
    pub fn detect_graphics_hardware(&mut self) -> Result<(), &'static str> {
        // Clear any previous detections
        self.detected_gpus.clear();
        self.primary_gpu = None;
        
        #[cfg(feature = "std")]
        {
            // In standard mode, use system APIs to detect graphics hardware
            self.detect_graphics_hardware_std()?;
        }
        
        #[cfg(not(feature = "std"))]
        {
            // In OS mode, use direct hardware access
            self.detect_graphics_hardware_os()?;
        }
        
        // If we detected any GPUs, set the first one as primary
        if !self.detected_gpus.is_empty() {
            self.primary_gpu = Some(0);
            
            // Log detected hardware
            #[cfg(feature = "std")]
            {
                let gpu = &self.detected_gpus[0];
                log::info!("Primary GPU: {} {}", 
                    match &gpu.vendor {
                        GpuVendor::Nvidia => "NVIDIA",
                        GpuVendor::AMD => "AMD",
                        GpuVendor::Intel => "Intel",
                        GpuVendor::VMware => "VMware",
                        GpuVendor::VirtualBox => "VirtualBox",
                        GpuVendor::Other(name) => name,
                        GpuVendor::Unknown => "Unknown",
                    },
                    gpu.model
                );
                
                if let Some(vram) = gpu.vram_mb {
                    log::info!("GPU VRAM: {} MB", vram);
                }
                
                if let Some(driver) = &gpu.driver_version {
                    log::info!("GPU Driver: {}", driver);
                }
                
                log::info!("Gaming features: Vulkan: {}, OpenGL: {}, HDR: {}, Variable Refresh: {}, Ray Tracing: {}",
                    gpu.supports_vulkan, 
                    gpu.supports_opengl, 
                    gpu.supports_hdr, 
                    gpu.supports_variable_refresh,
                    gpu.supports_raytracing
                );
            }
            
            Ok(())
        } else {
            Err("No graphics hardware detected")
        }
    }
    
    #[cfg(feature = "std")]
    fn detect_graphics_hardware_std(&mut self) -> Result<(), &'static str> {
        // Try multiple detection methods and combine results
        self.detect_via_lspci()?;
        self.detect_via_glx()?;
        self.detect_vulkan_support()?;
        self.detect_monitor_capabilities()?;
        
        Ok(())
    }
    
    #[cfg(feature = "std")]
    fn detect_via_lspci(&mut self) -> Result<(), &'static str> {
        // Use lspci command to detect PCI graphics cards
        if let Ok(output) = Command::new("lspci")
            .arg("-v")
            .output() 
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Process each line looking for VGA/3D controllers
            for line in output_str.lines() {
                if line.contains("VGA compatible controller") || 
                   line.contains("3D controller") || 
                   line.contains("Display controller") 
                {
                    let mut gpu_info = GpuInfo {
                        vendor: GpuVendor::Unknown,
                        model: "Unknown".to_string(),
                        vram_mb: None,
                        driver_version: None,
                        supports_vulkan: false,
                        supports_opengl: true, // Most GPUs support OpenGL
                        max_resolution: None,
                        supports_hdr: false,
                        supports_variable_refresh: false,
                        supports_raytracing: false,
                    };
                    
                    // Extract vendor and model
                    if line.contains("NVIDIA") {
                        gpu_info.vendor = GpuVendor::Nvidia;
                        
                        // NVIDIA usually has good Vulkan support
                        gpu_info.supports_vulkan = true;
                        
                        // Extract model name
                        if let Some(idx) = line.find("NVIDIA") {
                            let model_part = &line[idx..];
                            if let Some(end) = model_part.find("[") {
                                gpu_info.model = model_part[..end].trim().to_string();
                            } else {
                                gpu_info.model = model_part.trim().to_string();
                            }
                            
                            // Check for ray tracing support (RTX cards)
                            if model_part.contains("RTX") {
                                gpu_info.supports_raytracing = true;
                            }
                        }
                        
                        // NVIDIA typically supports G-Sync on newer cards
                        gpu_info.supports_variable_refresh = true;
                        
                    } else if line.contains("AMD") || line.contains("ATI") {
                        gpu_info.vendor = GpuVendor::AMD;
                        gpu_info.supports_vulkan = true;
                        
                        // Extract model name
                        if let Some(idx) = line.find("AMD") {
                            let model_part = &line[idx..];
                            if let Some(end) = model_part.find("[") {
                                gpu_info.model = model_part[..end].trim().to_string();
                            } else {
                                gpu_info.model = model_part.trim().to_string();
                            }
                            
                            // Check for ray tracing support (RX 6000 series and newer)
                            if model_part.contains("Radeon RX 6") || 
                               model_part.contains("Radeon RX 7") {
                                gpu_info.supports_raytracing = true;
                            }
                        }
                        
                        // AMD typically supports FreeSync on newer cards
                        gpu_info.supports_variable_refresh = true;
                        
                    } else if line.contains("Intel") {
                        gpu_info.vendor = GpuVendor::Intel;
                        
                        // Extract model name
                        if let Some(idx) = line.find("Intel") {
                            let model_part = &line[idx..];
                            if let Some(end) = model_part.find("[") {
                                gpu_info.model = model_part[..end].trim().to_string();
                            } else {
                                gpu_info.model = model_part.trim().to_string();
                            }
                            
                            // Check for newer Intel GPUs with ray tracing
                            if model_part.contains("Arc") {
                                gpu_info.supports_raytracing = true;
                            }
                        }
                        
                        // Intel has variable refresh on newer GPUs
                        if gpu_info.model.contains("UHD") || 
                           gpu_info.model.contains("Iris") || 
                           gpu_info.model.contains("Arc") {
                            gpu_info.supports_variable_refresh = true;
                        }
                        
                    } else if line.contains("VMware") {
                        gpu_info.vendor = GpuVendor::VMware;
                        gpu_info.model = "VMware Virtual GPU".to_string();
                    } else if line.contains("VirtualBox") {
                        gpu_info.vendor = GpuVendor::VirtualBox;
                        gpu_info.model = "VirtualBox Graphics Adapter".to_string();
                    } else {
                        // Extract whatever identifier we can find
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() > 1 {
                            let desc = parts[1].trim();
                            gpu_info.vendor = GpuVendor::Other(desc.to_string());
                            gpu_info.model = desc.to_string();
                        }
                    }
                    
                    // Try to find VRAM info
                    if let Some(mem_idx) = line.find("Memory at") {
                        // This is not accurate for VRAM size, just indicates mapped regions
                        // Real VRAM detection is more complex
                    }
                    
                    self.detected_gpus.push(gpu_info);
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(feature = "std")]
    fn detect_via_glx(&mut self) -> Result<(), &'static str> {
        // Try to get information via glxinfo if available
        if let Ok(output) = Command::new("glxinfo")
            .output() 
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // If we already detected a GPU, update its info
            if !self.detected_gpus.is_empty() {
                let gpu = &mut self.detected_gpus[0];
                
                // Extract OpenGL driver version
                for line in output_str.lines() {
                    if line.starts_with("OpenGL version string:") {
                        gpu.driver_version = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
                        gpu.supports_opengl = true;
                    }
                    
                    // Check for Vulkan support
                    if line.contains("Vulkan") {
                        gpu.supports_vulkan = true;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(feature = "std")]
    fn detect_vulkan_support(&mut self) -> Result<(), &'static str> {
        // Try to detect Vulkan support via vulkaninfo if available
        if let Ok(output) = Command::new("vulkaninfo")
            .output() 
        {
            // If we get here, Vulkan is likely supported
            if !self.detected_gpus.is_empty() {
                self.detected_gpus[0].supports_vulkan = true;
                
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                // Check for ray tracing extension support
                if output_str.contains("VK_KHR_ray_tracing") || 
                   output_str.contains("VK_NV_ray_tracing") {
                    self.detected_gpus[0].supports_raytracing = true;
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(feature = "std")]
    fn detect_monitor_capabilities(&mut self) -> Result<(), &'static str> {
        // Detect monitor capabilities via xrandr or other means
        if let Ok(output) = Command::new("xrandr")
            .arg("--prop")
            .output() 
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Look for connected displays
            let mut max_width = 0;
            let mut max_height = 0;
            let mut max_refresh = 0;
            let mut has_hdr = false;
            
            for line in output_str.lines() {
                // Check for connected monitor line
                if line.contains(" connected ") {
                    // Next lines will have resolution modes
                    continue;
                }
                
                // Parse resolution mode lines
                // Format: "1920x1080 60.00*+" where * indicates preferred and + current
                if line.trim().starts_with("19") || 
                   line.trim().starts_with("25") || 
                   line.trim().starts_with("38") {
                    
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if let Some(res_part) = parts.first() {
                        if let Some(separator) = res_part.find('x') {
                            if let Ok(width) = res_part[..separator].parse::<u32>() {
                                if let Ok(height) = res_part[separator+1..].parse::<u32>() {
                                    // Check if this is the highest resolution
                                    if width * height > max_width * max_height {
                                        max_width = width;
                                        max_height = height;
                                    }
                                }
                            }
                        }
                    }
                    
                    // Check refresh rate
                    if parts.len() > 1 {
                        if let Ok(refresh) = parts[1].parse::<f32>() {
                            if refresh as u32 > max_refresh {
                                max_refresh = refresh as u32;
                            }
                        }
                    }
                }
                
                // Check for HDR support
                if line.contains("Colorimetry:") && line.contains("BT2020") {
                    has_hdr = true;
                }
            }
            
            // If we detected a GPU, update its resolution info
            if !self.detected_gpus.is_empty() && max_width > 0 && max_height > 0 {
                self.detected_gpus[0].max_resolution = Some(HdmiResolution {
                    width: max_width,
                    height: max_height,
                    refresh_rate: max_refresh,
                });
                
                self.detected_gpus[0].supports_hdr = has_hdr;
            }
        }
        
        Ok(())
    }
    
    #[cfg(not(feature = "std"))]
    fn detect_graphics_hardware_os(&mut self) -> Result<(), &'static str> {
        // In OS mode (bare metal), we need to directly access PCI configuration space
        
        // This is a simplified version - real code would enumerate PCI bus and check
        // for devices with class code 0x03 (Display controller)
        let mut gpu_info = GpuInfo {
            vendor: GpuVendor::Unknown,
            model: "Unknown".to_string(),
            vram_mb: Some(2048), // Default to 2GB VRAM as placeholder
            driver_version: None,
            supports_vulkan: false,
            supports_opengl: true,
            max_resolution: Some(HdmiResolution {
                width: 1920,
                height: 1080,
                refresh_rate: 60,
            }),
            supports_hdr: false,
            supports_variable_refresh: false,
            supports_raytracing: false,
        };
        
        // In a real implementation, we'd read the PCI vendor and device IDs
        // and look them up in a database of known GPUs
        
        // For demonstration, we'll assume we detected something
        self.detected_gpus.push(gpu_info);
        
        Ok(())
    }
    
    // Method to read EDID data from the monitor
    pub fn read_edid(&self) -> Result<Vec<u8>, &'static str> {
        #[cfg(feature = "std")]
        {
            // Try to read EDID using system commands
            if let Ok(output) = Command::new("xrandr")
                .args(["--prop", "--verbose"])
                .output() 
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                // Extract EDID block
                let mut edid_data = Vec::new();
                let mut capture = false;
                
                for line in output_str.lines() {
                    if line.contains("EDID:") {
                        capture = true;
                        continue;
                    }
                    
                    if capture && line.trim().is_empty() {
                        capture = false;
                        continue;
                    }
                    
                    if capture {
                        // Parse hex bytes from EDID dump
                        for byte_str in line.trim().split_whitespace() {
                            if let Ok(byte) = u8::from_str_radix(byte_str, 16) {
                                edid_data.push(byte);
                            }
                        }
                    }
                }
                
                if !edid_data.is_empty() {
                    return Ok(edid_data);
                }
            }
            
            // Alternative method using i2c if xrandr didn't work
            #[cfg(target_os = "linux")]
            {
                if let Ok(mut file) = std::fs::File::open("/sys/class/drm/card0-HDMI-A-1/edid") {
                    let mut edid_data = Vec::new();
                    use std::io::Read;
                    if file.read_to_end(&mut edid_data).is_ok() && !edid_data.is_empty() {
                        return Ok(edid_data);
                    }
                }
            }
        }
        
        #[cfg(not(feature = "std"))]
        {
            // In OS mode, we'd read EDID directly from the HDMI controller registers
            // This is highly hardware-specific and would require direct I/O access
            
            // Mock EDID data for example purposes (this is not real EDID)
            let mock_edid = vec![
                0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, // Header
                0x10, 0xAC, 0x23, 0xF0, 0x00, 0x00, 0x00, 0x00, // Manufacturer ID
                0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, // Product ID, serial
                0x01, 0x1D, 0x01, 0x03, 0x80, 0x50, 0x2D, 0x78, // Version
                0x0A, 0xF0, 0x9D, 0xA3, 0x55, 0x49, 0x9B, 0x25, // Basic display
                0x0F, 0x50, 0x54, 0xBF, 0xEF, 0x80, 0x71, 0x4F, // Color characteristics
                0x81, 0xC0, 0x81, 0x00, 0x95, 0x00, 0xB3, 0x00, // Established timings
                0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, // Standard timings
                0x02, 0x3A, 0x80, 0xD0, 0x72, 0x38, 0x2D, 0x40, // 1920x1080@60Hz
                0x10, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0xFC, 0x00, 0x47, // Monitor name
                0x61, 0x6D, 0x69, 0x6E, 0x67, 0x20, 0x4D, 0x6F, // "Gaming Mo"
                0x6E, 0x69, 0x74, 0x6F, 0x72, 0x00, 0x00, 0x00, // "nitor"
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32, // Checksum
            ];
            
            return Ok(mock_edid);
        }
        
        Err("Could not read EDID data")
    }
    
    // Parse EDID data to extract monitor capabilities
    pub fn parse_edid(&self, edid: &[u8]) -> Result<MonitorCapabilities, &'static str> {
        if edid.len() < 128 {
            return Err("EDID data too short");
        }
        
        // Check EDID header
        if &edid[0..8] != &[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
            return Err("Invalid EDID header");
        }
        
        // Extract manufacturer ID (3 character code)
        let mut manufacturer = String::new();
        let id = ((edid[8] as u16) << 8) | (edid[9] as u16);
        manufacturer.push(char::from_u32((((id >> 10) & 0x1F) + 64).into()).unwrap_or('?'));
        manufacturer.push(char::from_u32((((id >> 5) & 0x1F) + 64).into()).unwrap_or('?'));
        manufacturer.push(char::from_u32(((id & 0x1F) + 64).into()).unwrap_or('?'));
        
        // Extract product code
        let product_code = ((edid[10] as u16) << 8) | (edid[11] as u16);
        
        // Extract serial number
        let serial = ((edid[12] as u32) << 24) | 
                     ((edid[13] as u32) << 16) | 
                     ((edid[14] as u32) << 8) | 
                     (edid[15] as u32);
        
        // Extract manufacture date
        let week = edid[16];
        let year = 1990 + edid[17] as u16;
        
        // Extract preferred resolution from detailed timing descriptor
        let width = ((edid[54] as u32) + ((edid[56] as u32 & 0xF0) << 4)) as u32;
        let height = ((edid[57] as u32) + ((edid[59] as u32 & 0xF0) << 4)) as u32;
        
        // Extract refresh rate
        let pixel_clock = ((edid[54] as u32) << 8) | (edid[55] as u32);
        let h_active = ((edid[56] as u32 & 0xF0) << 4) | (edid[58] as u32);
        let v_active = ((edid[59] as u32 & 0xF0) << 4) | (edid[61] as u32);
        let h_blanking = ((edid[56] as u32 & 0x0F) << 8) | (edid[57] as u32);
        let v_blanking = ((edid[59] as u32 & 0x0F) << 8) | (edid[60] as u32);
        
        let refresh_rate = if pixel_clock > 0 && h_active > 0 && v_active > 0 &&
                             h_blanking > 0 && v_blanking > 0 {
            let total_pixels = (h_active + h_blanking) * (v_active + v_blanking);
            (pixel_clock * 10000) / total_pixels
        } else {
            60 // Default to 60Hz if calculation fails
        };
        
        // Extract monitor name from descriptor blocks
        let mut monitor_name = String::new();
        for block in 0..4 {
            let offset = 54 + block * 18;
            if offset + 18 <= edid.len() && 
               edid[offset] == 0 && edid[offset+1] == 0 && 
               edid[offset+2] == 0 && edid[offset+3] == 0xFC {
                
                for i in 0..13 {
                    let byte = edid[offset + 5 + i];
                    if byte == 0x0A {
                        break;
                    }
                    if byte >= 0x20 {
                        monitor_name.push(byte as char);
                    }
                }
                break;
            }
        }
        
        // Check for HDR and other gaming features
        // Note: Full HDR detection requires parsing extension blocks
        let supports_hdr = false; // Default
        
        // Check for variable refresh rate support (this is a simplification)
        let supports_freesync = false; // Default
        let supports_gsync = false; // Default
        
        Ok(MonitorCapabilities {
            manufacturer,
            product_code,
            serial,
            manufacture_date: format!("{}-{}", year, week),
            name: monitor_name,
            max_resolution: HdmiResolution {
                width,
                height,
                refresh_rate,
            },
            supports_hdr,
            supports_freesync,
            supports_gsync,
        })
    }
    
    // Get primary GPU information
    pub fn get_primary_gpu(&self) -> Option<&GpuInfo> {
        match self.primary_gpu {
            Some(idx) if idx < self.detected_gpus.len() => Some(&self.detected_gpus[idx]),
            _ => None,
        }
    }
    
    // Get all detected GPUs
    pub fn get_detected_gpus(&self) -> &[GpuInfo] {
        &self.detected_gpus
    }
    
    // Detect if the system meets minimum gaming requirements
    pub fn meets_gaming_requirements(&self, requirements: &GamingRequirements) -> bool {
        if let Some(gpu) = self.get_primary_gpu() {
            // Check GPU VRAM
            if let Some(vram) = gpu.vram_mb {
                if vram < requirements.min_vram_mb {
                    return false;
                }
            } else if requirements.min_vram_mb > 0 {
                // If we can't detect VRAM and requirements need it, assume false
                return false;
            }
            
            // Check API support
            if requirements.requires_vulkan && !gpu.supports_vulkan {
                return false;
            }
            
            if requirements.requires_opengl && !gpu.supports_opengl {
                return false;
            }
            
            // Check ray tracing
            if requirements.requires_raytracing && !gpu.supports_raytracing {
                return false;
            }
            
            // Check resolution
            if let Some(res) = &gpu.max_resolution {
                if res.width < requirements.min_width || 
                   res.height < requirements.min_height ||
                   res.refresh_rate < requirements.min_refresh_rate {
                    return false;
                }
            }
            
            // All checks passed
            true
        } else {
            false
        }
    }
}

// Implement proper cleanup when the driver is dropped
impl Drop for HdmiDriver {
    fn drop(&mut self) {
        if self.initialized.load(Ordering::SeqCst) {
            // Perform cleanup - disable HDMI output, free resources, etc.
            if let Some(fb) = self.framebuffer {
                // In a real implementation, you would deallocate the framebuffer here
                unsafe {
                    let _ = Vec::from_raw_parts(fb, self.framebuffer_size, self.framebuffer_size);
                }
            }
        }
    }
}

// Global HDMI driver instance
lazy_static! {
    pub static ref HDMI_DRIVER: Mutex<HdmiDriver> = Mutex::new(HdmiDriver::new());
}

// Public interface for HDMI operations
pub fn init() -> Result<(), &'static str> {
    HDMI_DRIVER.lock().init()
}

pub fn clear_screen(r: u8, g: u8, b: u8) -> Result<(), &'static str> {
    HDMI_DRIVER.lock().clear_screen(r, g, b)
}

pub fn set_pixel(x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) -> Result<(), &'static str> {
    HDMI_DRIVER.lock().set_pixel(x, y, r, g, b, a)
}

pub fn flush() -> Result<(), &'static str> {
    HDMI_DRIVER.lock().flush()
}
