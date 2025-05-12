//! NVIDIA GPU drivers
//!
//! This module provides drivers for NVIDIA GeForce graphics hardware.
extern crate alloc;
use alloc::boxed::Box;
use crate::kernel::drivers::gpu::pci::PciDevice;
use crate::kernel::drivers::gpu::{GpuInfo, GpuError};
use super::GpuDevice;

mod ampere;
mod turing;
mod cuda;
mod common;

/// Create an appropriate NVIDIA GPU driver based on the device ID
pub fn create_driver(device: &PciDevice) -> Result<Box<dyn GpuDevice>, GpuError> {
    // NVIDIA device ID ranges are more complex
    // RTX 30 series (Ampere): 0x2200-0x24FF
    // RTX 20 series (Turing): 0x1E00-0x1FFF
    // GTX 16 series (Turing): 0x1F00-0x1FFF
    // GTX 10 series (Pascal): 0x1B00-0x1DFF
    
    let device_id = device.device_id;
    
    if (0x2200..=0x24FF).contains(&device_id) {
        // Ampere architecture (RTX 30 series)
        ampere::create_driver(device)
    }
    else if (0x1E00..=0x1FFF).contains(&device_id) {
        // Turing architecture (RTX 20, GTX 16 series)
        turing::create_driver(device)
    }
    else {
        // Use CUDA-based generic driver for older cards
        cuda::create_driver(device)
    }
}