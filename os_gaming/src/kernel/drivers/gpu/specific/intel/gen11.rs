//! Intel Gen 11 GPU Driver
//!
//! This module provides driver implementations specific to Intel's Gen 11 architecture (Ice Lake).
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::kernel::drivers::gpu::pci::PciDevice;
use crate::kernel::drivers::gpu::{GpuInfo, GpuError, DisplayMode, TextureFormat, Feature};
use super::{GpuDevice};
use super::common;

/// Intel Gen11 (Ice Lake) GPU device
pub struct IntelGen11 {
    // Device identification
    device_id: u16,
    is_initialized: bool,
    
    // Memory management
    mmio_base: usize,
    mmio_size: usize,
    framebuffer: usize,
    vram_size: usize,
    
    // Display configuration
    width: u32,
    height: u32,
    bpp: u8,
    pitch: u32,
    
    // Rendering state
    clip_x: i32,
    clip_y: i32,
    clip_width: u32,
    clip_height: u32,
    clip_enabled: bool,
    blend_mode: u32,
    
    // Texture management
    next_texture_id: u32,
    textures: BTreeMap<u32, TextureInfo>,
    
    // Hardware acceleration
    acceleration_enabled: AtomicBool,
    
    // Device identification
    device_name: &'static str,
    eu_count: u32,      // Gen11 has more EUs than Gen9
    supports_dp14: bool, // DisplayPort 1.4 support
    supports_hdmi20: bool, // HDMI 2.0 support
}

/// Texture information for Gen11 GPUs
struct TextureInfo {
    id: u32,
    width: u32,
    height: u32,
    format: u32,
    data: Vec<u8>,
    gpu_handle: u64,  // GPU-specific handle to texture memory
    is_compressed: bool,
    mip_levels: u8,
}

impl IntelGen11 {
    /// Creates a new instance of the Intel Gen 11 GPU driver.
    pub fn new(device: &PciDevice) -> Result<Box<dyn GpuDevice>, GpuError> {
        // Check if it's really an Intel GPU
        if device.vendor_id != 0x8086 {
            return Err(GpuError::InvalidDevice);
        }
        
        // Map MMIO registers (usually in BAR0)
        let mmio_base = (device.bar0 & 0xFFFFFFF0) as usize;
        let mmio_size = 4 * 1024 * 1024; // 4MB typical for Gen11 GPU MMIO
        
        common::map_mmio(mmio_base, mmio_size)?;
        
        // Determine which specific Gen11 GPU we have - Ice Lake variants
        let (device_name, eu_count, supports_dp14, supports_hdmi20) = match device.device_id {
            // Ice Lake GT2
            0x8A52 => ("Intel Iris Plus Graphics (Ice Lake G7, 64EU)", 64, true, true),
            0x8A51 => ("Intel Iris Plus Graphics (Ice Lake G4, 48EU)", 48, true, true),
            0x8A50 => ("Intel UHD Graphics (Ice Lake G1, 32EU)", 32, true, true),
            // Default case
            _ => ("Intel Gen11 Graphics", 48, true, true),
        };
        
        // Find framebuffer (usually in BAR2)
        let framebuffer = (device.bar2 & 0xFFFFFFF0) as usize;
        
        // Intel integrated GPUs use system memory as VRAM
        // Gen11 typically can access more memory
        let vram_size = 1 * 1024 * 1024 * 1024; // 1GB allocation for Gen11
        
        // Create the driver instance
        let driver = IntelGen11 {
            device_id: device.device_id,
            is_initialized: true,
            mmio_base,
            mmio_size,
            framebuffer,
            vram_size,
            width: 1920, // Default to 1080p
            height: 1080,
            bpp: 32,
            pitch: 1920 * 4, // width * bytes per pixel
            clip_x: 0,
            clip_y: 0,
            clip_width: 0,
            clip_height: 0,
            clip_enabled: false,
            blend_mode: 0,
            next_texture_id: 1,
            textures: BTreeMap::new(),
            acceleration_enabled: AtomicBool::new(true),
            device_name,
            eu_count,
            supports_dp14,
            supports_hdmi20,
        };
        
        // Initialize the hardware
        let mut driver = Box::new(driver);
        driver.initialize_hardware()?;
        
        Ok(driver)
    }
    
    /// Initialize the GPU hardware
    fn initialize_hardware(&mut self) -> Result<(), GpuError> {
        // In a real driver, we would:
        // 1. Reset the GPU if needed
        // 2. Initialize display planes
        // 3. Set up command buffers
        // 4. Configure power management
        // 5. Enable interrupts
        
        // Gen11 specific initialization:
        // 1. Configure increased EU count
        // 2. Enable DisplayPort 1.4 features if supported
        // 3. Configure HDMI 2.0 if available
        
        log::info!("Initialized {} with {} execution units", self.device_name, self.eu_count);
        log::info!("Display capabilities: DP 1.4: {}, HDMI 2.0: {}", 
                 self.supports_dp14, self.supports_hdmi20);
        
        Ok(())
    }
    
    // Helper methods for hardware interaction
    
    /// Read from a GPU register
    fn read_reg32(&self, offset: usize) -> u32 {
        common::read_reg32(self.mmio_base, offset)
    }
    
    /// Write to a GPU register
    fn write_reg32(&self, offset: usize, value: u32) {
        common::write_reg32(self.mmio_base, offset, value)
    }
    
    /// Wait for a register bit
    fn wait_for_reg32(&self, offset: usize, mask: u32, value: u32) -> Result<(), GpuError> {
        common::wait_for_reg32(self.mmio_base, offset, mask, value, 1000)
    }
    
    /// Software implementation of rectangle fill
    fn sw_fill_rect(&self, mut x: i32, mut y: i32, mut width: u32, mut height: u32, color: u32) -> Result<(), GpuError> {
        // Apply clipping if enabled
        if self.clip_enabled {
            // Clip left/top
            if x < self.clip_x {
                width = width.saturating_sub((self.clip_x - x) as u32);
                x = self.clip_x;
            }
            if y < self.clip_y {
                height = height.saturating_sub((self.clip_y - y) as u32);
                y = self.clip_y;
            }
            
            // Clip right/bottom
            let clip_right = self.clip_x + self.clip_width as i32;
            let clip_bottom = self.clip_y + self.clip_height as i32;
            
            if x + width as i32 > clip_right {
                width = (clip_right - x) as u32;
            }
            if y + height as i32 > clip_bottom {
                height = (clip_bottom - y) as u32;
            }
        }
        
        // Bounds check against screen dimensions
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return Ok(());
        }
        
        if x + width as i32 <= 0 || y + height as i32 <= 0 {
            return Ok(());
        }
        
        // Adjust width/height to stay within bounds
        if x + width as i32 > self.width as i32 {
            width = (self.width as i32 - x) as u32;
        }
        if y + height as i32 > self.height as i32 {
            height = (self.height as i32 - y) as u32;
        }
        
        // Draw the rectangle
        unsafe {
            let framebuffer = self.framebuffer as *mut u32;
            let pitch = self.pitch / 4; // Convert from bytes to 32-bit words
            
            for row in 0..height {
                let row_offset = (y as u32 + row) * pitch + x as u32;
                let row_ptr = framebuffer.add(row_offset as usize);
                
                for col in 0..width {
                    *row_ptr.add(col as usize) = color;
                }
            }
        }
        
        Ok(())
    }
    
    /// Hardware implementation of rectangle fill using Gen11 blitter
    /// Gen11 has improved blitter performance over Gen9
    fn hw_fill_rect(&self, x: i32, y: i32, width: u32, height: u32, color: u32) -> Result<(), GpuError> {
        // In a real driver, we would:
        // 1. Create a blit command in the command buffer
        // 2. Set source and destination coordinates
        // 3. Set color and operation
        // 4. Submit the command
        
        // Gen11 specific register offsets (simplified)
        // These would differ from Gen9 in a real implementation
        const REG_BLIT_DST_BASE: usize = 0x80000;
        const REG_BLIT_DST_PITCH: usize = 0x80004;
        const REG_BLIT_COLOR: usize = 0x80008;
        const REG_BLIT_RECT: usize = 0x8000C;
        const REG_BLIT_CONTROL: usize = 0x80010;
        const REG_BLIT_STATUS: usize = 0x80014;
        
        // Write blit parameters
        self.write_reg32(REG_BLIT_DST_BASE, self.framebuffer as u32);
        self.write_reg32(REG_BLIT_DST_PITCH, self.pitch);
        self.write_reg32(REG_BLIT_COLOR, color);
        self.write_reg32(REG_BLIT_RECT, ((x & 0xFFFF) | ((y & 0xFFFF) << 16)).try_into().unwrap());
        self.write_reg32(REG_BLIT_RECT + 4, (width & 0xFFFF) | ((height & 0xFFFF) << 16));
        
        // Start the blit operation
        self.write_reg32(REG_BLIT_CONTROL, 0x01); // Start bit
        
        // Wait for completion
        self.wait_for_reg32(REG_BLIT_STATUS, 0x01, 0x01)?;
        
        Ok(())
    }
    
    // Additional Gen11-specific hardware acceleration methods
    
    /// Hardware-accelerated bilinear texture sampling
    fn hw_sample_texture(&self, texture: &TextureInfo, u: f32, v: f32) -> u32 {
        // In a real driver, this would use Gen11's texture sampler hardware
        // For this implementation, just return a placeholder color
        0xFFCCAAFF
    }
}

impl GpuDevice for IntelGen11 {
    fn get_info(&self) -> Result<GpuInfo, GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // List available display modes
        // Gen11 supports higher resolutions than Gen9
        let modes = [
            DisplayMode { width: 3840, height: 2160, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 2560, height: 1440, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 1920, height: 1080, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 1680, height: 1050, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 1600, height: 900, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 1366, height: 768, bpp: 32, refresh_rate: 60 },
            DisplayMode { width: 1280, height: 720, bpp: 32, refresh_rate: 60 },
        ];
        
        // Current mode
        let current_mode = DisplayMode {
            width: self.width,
            height: self.height,
            bpp: self.bpp,
            refresh_rate: 60,
        };
        
        // Gen11 has more features than Gen9
        let features = Feature::Acceleration2D as u32 | 
                      Feature::Blending as u32 | 
                      Feature::HardwareCursor as u32 | 
                      Feature::MemoryMapping as u32 |
                      Feature::Shaders as u32 |      // Gen11 has better shader support
                      Feature::RenderTargets as u32; // Gen11 supports render targets
        
        // Create GPU info with Intel-specific capabilities
        let info = GpuInfo {
            vendor: "Intel",
            device: self.device_name,
            vram_size: self.vram_size,
            max_texture_size: 16384,
            features,
            current_mode,
            available_modes: Box::leak(Box::new(modes)),
        };
        
        Ok(info)
    }

    fn get_framebuffer(&mut self, width: u32, height: u32) -> Result<usize, GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Check if mode change is needed
        if width != self.width || height != self.height {
            // Set new mode
            self.width = width;
            self.height = height;
            self.pitch = width * (self.bpp as u32 / 8);
            
            // In a real driver, we would configure the display controller
            
            log::debug!("Changed resolution to {}x{}", width, height);
        }
        
        Ok(self.framebuffer)
    }

    fn get_framebuffer_pitch(&self) -> Result<u32, GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        Ok(self.pitch)
    }

    fn clear(&mut self, color: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Use rectangle fill to clear the entire screen
        self.fill_rect(0, 0, self.width, self.height, color)
    }

    fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        if width == 0 || height == 0 {
            return Ok(());
        }
        
        // Use hardware acceleration if available, otherwise fall back to software
        if self.acceleration_enabled.load(Ordering::Relaxed) {
            match self.hw_fill_rect(x, y, width, height, color) {
                Ok(_) => return Ok(()),
                Err(_) => {
                    // Hardware acceleration failed, disable it and fall back to software
                    log::warn!("Hardware acceleration failed, falling back to software rendering");
                    self.acceleration_enabled.store(false, Ordering::Relaxed);
                }
            }
        }
        
        // Software fallback
        self.sw_fill_rect(x, y, width, height, color)
    }

    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Bresenham's line algorithm - same as Gen9 implementation
        let mut x = x1;
        let mut y = y1;
        
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = if dx > dy { dx } else { -dy } / 2;
        
        loop {
            // Check clip region and screen bounds
            if (!self.clip_enabled || 
                (x >= self.clip_x && x < self.clip_x + self.clip_width as i32 &&
                 y >= self.clip_y && y < self.clip_y + self.clip_height as i32)) &&
               x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
                
                // Plot the pixel
                unsafe {
                    let framebuffer = self.framebuffer as *mut u32;
                    let offset = y as usize * (self.pitch / 4) as usize + x as usize;
                    *framebuffer.add(offset) = color;
                }
            }
            
            // Exit condition
            if x == x2 && y == y2 {
                break;
            }
            
            // Update position
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

    fn create_texture(&mut self, width: u32, height: u32, format: u32, data: &[u8]) -> Result<u32, GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Validate parameters - Gen11 supports larger textures than Gen9
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(GpuError::InvalidParameter);
        }
        
        // Check format
        if format > 4 {
            return Err(GpuError::UnsupportedFormat);
        }
        
        // Calculate bytes per pixel
        let bytes_per_pixel = match format {
            0 | 2 => 4, // RGBA8 or BGRA8
            1 | 3 => 3, // RGB8 or BGR8
            4 => 1,     // A8
            _ => return Err(GpuError::UnsupportedFormat),
        };
        
        // Calculate expected size
        let expected_size = (width * height * bytes_per_pixel) as usize;
        
        // Validate data size
        if data.len() < expected_size {
            return Err(GpuError::InvalidParameter);
        }
        
        // Generate a texture ID
        let texture_id = self.next_texture_id;
        self.next_texture_id += 1;
        
        // Copy texture data
        let mut texture_data = Vec::with_capacity(expected_size);
        texture_data.extend_from_slice(&data[0..expected_size]);
        
        // Store texture info - Gen11 texture has additional fields
        let texture = TextureInfo {
            id: texture_id,
            width,
            height,
            format,
            data: texture_data,
            gpu_handle: 0,   // Would be a real handle in production
            is_compressed: false,
            mip_levels: 1,
        };
        
        self.textures.insert(texture_id, texture);
        
        log::debug!("Created texture ID {} with size {}x{}, format {}", 
                  texture_id, width, height, format);
        
        Ok(texture_id)
    }

    fn destroy_texture(&mut self, texture_id: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Find and remove texture
        if self.textures.remove(&texture_id).is_some() {
            log::debug!("Destroyed texture ID {}", texture_id);
            Ok(())
        } else {
            Err(GpuError::InvalidTexture)
        }
    }

    fn get_texture_data(&self, texture_id: u32) -> Result<&[u8], GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Find texture and return data reference
        if let Some(texture) = self.textures.get(&texture_id) {
            Ok(&texture.data)
        } else {
            Err(GpuError::InvalidTexture)
        }
    }

    fn draw_texture(&mut self, texture_id: u32, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Find texture
        let texture = match self.textures.get(&texture_id) {
            Some(tex) => tex,
            None => return Err(GpuError::InvalidTexture),
        };
        
        // Gen11 has improved texture sampling hardware
        // In a real driver, we would:
        // 1. Set up texture sampler state
        // 2. Configure filtering mode
        // 3. Submit a texture blit command
        
        // For now, use the same software implementation as Gen9
        // Simple scaling implementation
        let scale_x = texture.width as f32 / width as f32;
        let scale_y = texture.height as f32 / height as f32;
        
        // For each destination pixel, sample the source texture
        for dy in 0..height {
            let dst_y = y + dy as i32;
            
            // Skip if outside screen bounds or clip rect
            if dst_y < 0 || dst_y >= self.height as i32 {
                continue;
            }
            
            if self.clip_enabled && 
               (dst_y < self.clip_y || dst_y >= self.clip_y + self.clip_height as i32) {
                continue;
            }
            
            for dx in 0..width {
                let dst_x = x + dx as i32;
                
                // Skip if outside screen bounds or clip rect
                if dst_x < 0 || dst_x >= self.width as i32 {
                    continue;
                }
                
                if self.clip_enabled && 
                   (dst_x < self.clip_x || dst_x >= self.clip_x + self.clip_width as i32) {
                    continue;
                }
                
                // Calculate source coordinates
                let src_x = (dx as f32 * scale_x) as u32;
                let src_y = (dy as f32 * scale_y) as u32;
                
                if src_x >= texture.width || src_y >= texture.height {
                    continue;
                }
                
                // Get pixel from texture
                let bytes_per_pixel = match texture.format {
                    0 | 2 => 4, // RGBA8 or BGRA8
                    1 | 3 => 3, // RGB8 or BGR8
                    4 => 1,     // A8
                    _ => continue,
                };
                
                let src_offset = ((src_y * texture.width) + src_x) as usize * bytes_per_pixel;
                
                // Bounds check
                if src_offset + bytes_per_pixel > texture.data.len() {
                    continue;
                }
                
                // Convert pixel data to RGBA color
                let pixel = match texture.format {
                    0 => { // RGBA8
                        let r = texture.data[src_offset];
                        let g = texture.data[src_offset + 1];
                        let b = texture.data[src_offset + 2];
                        let a = texture.data[src_offset + 3];
                        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32
                    },
                    1 => { // RGB8
                        let r = texture.data[src_offset];
                        let g = texture.data[src_offset + 1];
                        let b = texture.data[src_offset + 2];
                        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 255
                    },
                    2 => { // BGRA8
                        let b = texture.data[src_offset];
                        let g = texture.data[src_offset + 1];
                        let r = texture.data[src_offset + 2];
                        let a = texture.data[src_offset + 3];
                        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32
                    },
                    3 => { // BGR8
                        let b = texture.data[src_offset];
                        let g = texture.data[src_offset + 1];
                        let r = texture.data[src_offset + 2];
                        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 255
                    },
                    4 => { // A8
                        let a = texture.data[src_offset];
                        0xFFFFFF00 | a as u32
                    },
                    _ => continue,
                };
                
                // Apply pixel to framebuffer
                unsafe {
                    let framebuffer = self.framebuffer as *mut u32;
                    let dst_offset = (dst_y as usize * (self.pitch / 4) as usize) + dst_x as usize;
                    *framebuffer.add(dst_offset) = pixel;
                }
            }
        }
        
        Ok(())
    }

    fn set_clip_rect(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Store clip rectangle
        self.clip_x = x;
        self.clip_y = y;
        self.clip_width = width;
        self.clip_height = height;
        self.clip_enabled = true;
        
        Ok(())
    }

    fn clear_clip_rect(&mut self) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Disable clipping
        self.clip_enabled = false;
        
        Ok(())
    }

    fn set_blend_mode(&mut self, mode: u32) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // Validate blend mode
        if mode > 3 {
            return Err(GpuError::InvalidParameter);
        }
        
        self.blend_mode = mode;
        
        Ok(())
    }

    fn present(&mut self) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Err(GpuError::NotInitialized);
        }
        
        // For a simple framebuffer model, there's nothing to do
        // In a real implementation with double/triple buffering:
        // 1. We would swap buffers
        // 2. Signal a page flip to the display controller
        
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), GpuError> {
        if !self.is_initialized {
            return Ok(());
        }
        
        // Free textures
        self.textures.clear();
        
        // Unmap MMIO
        common::unmap_mmio(self.mmio_base, self.mmio_size)?;
        
        self.is_initialized = false;
        log::info!("Shut down Intel Gen11 GPU: {}", self.device_name);
        
        Ok(())
    }
}

/// Create an Intel Gen11 driver for the specified PCI device
pub fn create_driver(device: &PciDevice) -> Result<Box<dyn GpuDevice>, GpuError> {
    IntelGen11::new(device)
}