//! Kernel Memory Management Subsystem
//!
//! This module orchestrates the initialization and provides high-level access
//! to memory-related functionalities.

pub mod allocator;
pub mod dma;
pub mod memory_manager;
pub mod physical;
pub mod r#virtual;

// Re-export important types for convenience
pub use memory_manager::{
    MemoryError, MemoryInitError, MemoryProtection, CacheType, MemoryType, MemoryInfo, MemoryProtectionFlags,
    map_page_for_kernel, // For direct use by allocator or other low-level kernel parts
    // Public mapping functions are also available directly from memory_manager:
    // memory_manager::map_physical_memory, memory_manager::unmap_region
};
pub use physical::PAGE_SIZE;

use bootloader::BootInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::PageTableFlags;
use core::ptr::NonNull;

/// Flag to ensure memory initialization happens only once.
static MEMORY_SYSTEM_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the entire memory subsystem.
pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    if MEMORY_SYSTEM_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
        log::warn!("Memory subsystem already initialized.");
        return Ok(());
    }

    log::info!("Initializing Kernel Memory Subsystem...");

    // 1. Initialize Core Memory Manager (PMM through physical::init_frame_allocator, Mapper)
    if let Err(e) = memory_manager::MemoryManager::init_core(boot_info) {
        let err_msg: &'static str = e.into();
        log::error!("Core memory manager initialization failed: {}", err_msg);
        MEMORY_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(err_msg);
    }
    log::info!("Core memory manager (PMM & Mapper) initialized.");

    // 2. Initialize Virtual Memory Abstractions
    if let Err(e) = r#virtual::init_virtual_manager() { // Calls the VMM's own init
        log::error!("Virtual memory manager initialization failed: {}", e);
        MEMORY_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(e);
    }
    log::info!("Virtual memory manager abstractions initialized.");

    // 3. Initialize Higher-Level Memory Services (Kernel Heap, DMA)
    if let Err(e) = memory_manager::MemoryManager::init_services() { // This calls allocator::init_heap and dma::init
        let err_msg: &'static str = e.into();
        log::error!("Memory services (Heap/DMA) initialization failed: {}", err_msg);
        MEMORY_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(err_msg);
    }
    log::info!("Memory services (Heap & DMA) initialized.");

    let mem_info = memory_manager::memory_info();
    log::info!(
        "Memory Stats Post-Init: Total RAM: {} MiB, Free RAM: {} MiB, Used RAM: {} MiB",
        mem_info.total_ram / (1024 * 1024),
        mem_info.free_ram / (1024 * 1024),
        mem_info.used_ram / (1024 * 1024)
    );

    log::info!("Kernel Memory Subsystem successfully initialized.");
    Ok(())
}

// --- High-Level Public API Wrappers (Examples) ---

pub fn alloc_virtual_backed_memory(
    size: usize,
    protection: MemoryProtectionFlags,
    mem_type: MemoryType,
) -> Result<NonNull<u8>, MemoryError> {
    if !MEMORY_SYSTEM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Err(MemoryError::InvalidRange); }
    r#virtual::allocate_and_map(size, protection, mem_type) // From virtual.rs
        .map(|vaddr| NonNull::new(vaddr.as_mut_ptr()).ok_or(MemoryError::AllocationFailed))?
}

pub fn free_virtual_backed_memory(ptr: NonNull<u8>, size: usize) -> Result<(), MemoryError> {
    if !MEMORY_SYSTEM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Ok(()); }
    r#virtual::free_and_unmap(VirtAddr::from_ptr(ptr.as_ptr()), size) // From virtual.rs
}

pub fn map_phys_mem_to_kernel_virt(
    phys_addr: PhysAddr,
    size: usize,
    flags: PageTableFlags,
) -> Result<VirtAddr, MemoryError> {
    if !MEMORY_SYSTEM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Err(MemoryError::InvalidRange); }
    memory_manager::map_physical_memory(phys_addr, size, flags) // From memory_manager.rs
}

pub fn unmap_kernel_virt_region(virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
    if !MEMORY_SYSTEM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Ok(()); }
    memory_manager::unmap_region(virt_addr, size) // From memory_manager.rs
}

pub fn get_memory_statistics() -> MemoryInfo {
    if !MEMORY_SYSTEM_INITIALIZED.load(Ordering::Acquire) {
        return MemoryInfo { total_ram:0, free_ram:0, used_ram:0, reserved_ram:0, kernel_size:0, page_size: PAGE_SIZE};
    }
    memory_manager::memory_info() // From memory_manager.rs
}
