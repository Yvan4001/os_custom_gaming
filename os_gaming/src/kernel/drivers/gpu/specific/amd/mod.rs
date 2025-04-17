//! AMD GPU Drivers
//!
//! This module provides drivers for AMD GPUs, including both GCN (Graphics Core Next)
//! and RDNA architectures.

extern crate alloc;
use alloc::boxed::Box;
use crate::kernel::drivers::gpu::pci::PciDevice;
use crate::kernel::drivers::gpu::GpuError;
use super::GpuDevice;

// Export submodules
pub mod common;
pub mod gcn;
pub mod rdna;

/// AMD GPU driver factory
///
/// Creates the appropriate AMD GPU driver based on the PCI device information.
/// This function determines whether the GPU is GCN or RDNA architecture and
/// creates the corresponding driver.
pub fn create_driver(device: &PciDevice) -> Result<Box<dyn GpuDevice>, GpuError> {
    // Check if it's an AMD GPU
    if device.vendor_id != 0x1002 {
        return Err(GpuError::InvalidDevice);
    }

    // Determine AMD GPU architecture based on device ID
    match device.device_id {
        // RDNA 2 (RX 6000 series)
        0x73BF | 0x73BE | 0x73A5 | 0x73A3 | 0x73DF | 0x73FF => {
            log::info!("Detected AMD RDNA 2 GPU: {}", device.device_name);
            rdna::create_driver(device)
        }
        
        // RDNA 1 (RX 5000 series)
        0x731F | 0x7340 | 0x7312 | 0x7360 => {
            log::info!("Detected AMD RDNA 1 GPU: {}", device.device_name);
            rdna::create_driver(device)
        }
        
        // GCN 5 (Vega)
        0x687F | 0x6863 => {
            log::info!("Detected AMD GCN 5 (Vega) GPU: {}", device.device_name);
            gcn::create_driver(device)
        }
        
        // GCN 4 (Polaris)
        0x67DF | 0x67CF | 0x67FF => {
            log::info!("Detected AMD GCN 4 (Polaris) GPU: {}", device.device_name);
            gcn::create_driver(device)
        }
        
        // GCN 3 (Fiji)
        0x7300 | 0x7312 => {
            log::info!("Detected AMD GCN 3 (Fiji) GPU: {}", device.device_name);
            gcn::create_driver(device)
        }
        
        // GCN 2 (Hawaii)
        0x67B0 | 0x67B1 => {
            log::info!("Detected AMD GCN 2 (Hawaii) GPU: {}", device.device_name);
            gcn::create_driver(device)
        }
        
        // Default to GCN for unknown AMD GPUs
        _ => {
            log::info!("Detected unknown AMD GPU, using GCN driver");
            gcn::create_driver(device)
        }
    }
}

/// Convert AMD GPU error to generic GPU error
pub fn convert_error(error: common::AmdGpuError) -> GpuError {
    match error {
        common::AmdGpuError::InitializationFailed => GpuError::InitializationFailed,
        common::AmdGpuError::UnsupportedDevice => GpuError::UnsupportedDevice,
        common::AmdGpuError::CommunicationError => GpuError::HardwareError,
    }
}