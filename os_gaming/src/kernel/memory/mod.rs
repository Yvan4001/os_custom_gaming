//! Memory Management Subsystem
//! 
//! This module provides memory management functionality for the OS kernel,
//! including physical and virtual memory management, memory allocation,
//! and DMA (Direct Memory Access) support for hardware operations.

mod dma;
mod memory_manager;
pub mod physical;
pub mod r#virtual;
mod allocator;

use spin::Mutex;
use lazy_static::lazy_static;
use memory_manager::{MemoryError, MemoryManager};

// Create thread-safe static reference to the memory manager
lazy_static! {
    static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());
}

/// Initialize memory management subsystem
pub fn memory_init(multiboot_info_addr: usize) -> Result<(), &'static str> {
    // Initialize the physical memory manager first
    physical::init(multiboot_info_addr)?;
    
    // Initialize the kernel heap allocator
    allocator::init(0)?;
    
    // Initialize the virtual memory manager (paging)
    r#virtual::init()?;
    
    // Initialize DMA support for devices
    dma::init()?;
    
    Ok(())
}

/// Allocate virtual memory with specified size
pub fn allocate_virtual(size: usize, align: usize) -> Result<*mut u8, MemoryError> {
    let mut manager = MEMORY_MANAGER.lock();
    manager.allocate(size, align)
}

pub fn deallocate_virtual(ptr: *mut u8) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.deallocate(ptr);
}

/// Free previously allocated virtual memory
pub fn free_virtual(ptr: *mut u8) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.free(ptr);
}

/// Map physical memory to virtual address space
pub fn map_physical(phys_addr: usize, size: usize) -> Result<*mut u8, MemoryError> {
    let mut manager = MEMORY_MANAGER.lock();
    manager.map_physical(phys_addr, size)
}

/// Get current memory usage statistics
pub fn get_memory_stats() -> MemoryStats {
    MEMORY_MANAGER.lock().get_stats()
}

/// Memory usage statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_ram: usize,        // Total physical RAM in bytes
    pub available_ram: usize,    // Available physical RAM in bytes
    pub used_ram: usize,         // Used physical RAM in bytes
    pub total_swap: usize,       // Total swap space in bytes
    pub available_swap: usize,   // Available swap space in bytes
    pub kernel_heap_used: usize, // Kernel heap usage in bytes
}

pub fn init() -> Result<(), &'static str> {
    // Initialize the memory management subsystem
    memory_init(0)?;
    
    // Initialize the memory manager
    let _ = MEMORY_MANAGER.lock();
    
    // Call the init function directly
    memory_manager::MemoryManager::init()?;
    
    Ok(())
}