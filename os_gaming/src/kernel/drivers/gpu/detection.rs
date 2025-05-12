//! GPU hardware detection
//!
//! Detects available GPU hardware and creates appropriate driver instances.
extern crate alloc;
use alloc::boxed::Box;
use super::{GpuDevice, GpuError};
use super::pci;
use super::specific;

/// Detect available GPU hardware and return the most suitable driver
pub fn detect_gpu() -> Result<Box<dyn GpuDevice>, GpuError> {
    // First, try PCI enumeration to find discrete GPUs
    if let Ok(pci_devices) = pci::enumerate_gpus() {
        for device in pci_devices {
            // Try to initialize the appropriate driver based on vendor ID
            match device.vendor_id {
                0x8086 => {
                    // Intel
                    if let Ok(driver) = specific::intel::create_driver(&device) {
                        return Ok(driver);
                    }
                }
                0x1002 => {
                    // AMD
                    if let Ok(driver) = specific::amd::create_driver(&device) {
                        return Ok(driver);
                    }
                }
                0x10DE => {
                    // NVIDIA
                    if let Ok(driver) = specific::nvidia::create_driver(&device) {
                        return Ok(driver);
                    }
                }
                _ => {
                    // Unknown vendor, skip
                    continue;
                }
            }
        }
    }
    
    // If no discrete GPU found or initialization failed, try VESA/VBE
    if let Ok(driver) = super::vesa::create_driver() {
        return Ok(driver);
    }
    
    // No suitable GPU found
    Err(GpuError::NoDevice)
}