//! Common utilities for Intel GPU drivers
//!
//! This module provides shared functionality for all Intel GPU generations.

use crate::kernel::drivers::gpu::GpuError;
use crate::kernel::memory;

/// Map memory-mapped I/O region
pub fn map_mmio(physical_address: usize, size: usize) -> Result<(), GpuError> {
    // In a real OS, this would map physical memory to virtual memory
    // using page tables
    
    // For now, just simulate success
    Ok(())
}

/// Unmap memory-mapped I/O region
pub fn unmap_mmio(physical_address: usize, size: usize) -> Result<(), GpuError> {
    // In a real OS, this would unmap the memory
    
    // For now, just simulate success
    Ok(())
}

/// Read a 32-bit register
pub fn read_reg32(base: usize, offset: usize) -> u32 {
    unsafe {
        let ptr = (base + offset) as *const u32;
        ptr.read_volatile()
    }
}

/// Write a 32-bit register
pub fn write_reg32(base: usize, offset: usize, value: u32) {
    unsafe {
        let ptr = (base + offset) as *mut u32;
        ptr.write_volatile(value);
    }
}

/// Wait for a register bit to be set or cleared
pub fn wait_for_reg32(base: usize, offset: usize, mask: u32, value: u32, timeout_ms: u32) -> Result<(), GpuError> {
    // In a real driver, this would wait with a timeout
    // For now, we'll assume it succeeds immediately
    Ok(())
}

/// Convert a linear address to a tiled address (for GPU memory management)
pub fn linear_to_tiled(linear_addr: usize, width: u32, height: u32, bpp: u8) -> usize {
    // Intel GPUs use tiled memory addressing for better performance
    // This is a simplified version - real implementation would be more complex
    linear_addr
}