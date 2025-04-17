//! Intel GPU drivers
//!
//! This module provides drivers for Intel integrated graphics hardware.
extern crate alloc;
use alloc::boxed::Box;
use crate::kernel::drivers::gpu::pci::PciDevice;
use crate::kernel::drivers::gpu::{GpuInfo, GpuError, DisplayMode};
use super::GpuDevice;

mod gen9;
mod gen11;
mod gen12;
mod common;

/// Create an appropriate Intel GPU driver based on the device ID
pub fn create_driver(device: &PciDevice) -> Result<Box<dyn GpuDevice>, GpuError> {
    match device.device_id {
        // Intel Xe Graphics (Gen12)
        0x4905 | 0x4906 | 0x4907 | 0x4908 => gen12::create_driver(device),
        
        // Intel Iris Plus Graphics (Gen11)
        0x8A50 | 0x8A51 | 0x8A52 | 0x8A53 => gen11::create_driver(device),
        
        // Intel UHD Graphics (Gen9)
        0x3E90 | 0x3E91 | 0x3E92 | 0x3E93 | 0x3E94 => gen9::create_driver(device),
        
        // Unknown or unsupported device
        _ => Err(GpuError::UnsupportedFeature),
    }
}