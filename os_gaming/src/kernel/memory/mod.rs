//! Memory Management Subsystem
//! 
//! This module provides memory management functionality for the OS kernel,
//! including physical and virtual memory management, memory allocation,
//! and DMA (Direct Memory Access) support for hardware operations.

mod dma;
mod memory_manager;
pub mod physical;
pub mod r#virtual;
pub mod allocator;

use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::structures::paging::{FrameAllocator, Mapper, PageSize, PageTableFlags};
use x86_64::structures::paging::Translate;
pub(crate) use memory_manager::{MemoryError, MemoryManager, PhysicalMemoryManager};
use bootloader::BootInfo;
use x86_64::PhysAddr;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::kernel::memory::dma::DmaManager;
use crate::kernel::memory::r#virtual::VirtualMemoryManager;

// Create thread-safe static reference to the memory manager
lazy_static! {
    static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());
    static ref DMA_MANAGER: Mutex<DmaManager> = Mutex::new(DmaManager::new());
}

/// Initialize memory management subsystem
pub fn memory_init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    // Use a lock to prevent multiple initializations
    static INITIALIZED: AtomicBool = AtomicBool::new(false);

    // If already initialized, return immediately
    if INITIALIZED.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Get the physical memory offset
    let phys_mem_offset = boot_info.physical_memory_offset;

    // Configure the offset information in the allocator
    crate::kernel::memory::allocator::set_memory_offset_info(phys_mem_offset);

    // Initialize the memory manager
    let result = MemoryManager::init(boot_info);

    // Mark as initialized only on success
    if result.is_ok() {
        INITIALIZED.store(true, Ordering::SeqCst);
    }

    // Initialize IOMMU if available
    if result.is_ok() {
        if let Err(e) = DMA_MANAGER.lock().initialize_iommu() {
            log::warn!("IOMMU not available or initialization failed: {}", e);
            // Don't fail the entire initialization if IOMMU fails
        }
    }

    result
}


pub fn deallocate_virtual(ptr: *mut u8) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.deallocate(ptr, 0);
}

pub fn allocate_virtual(size: usize) -> Result<*mut u8, MemoryError> {
    let mut manager = MEMORY_MANAGER.lock();
    manager.allocate(size, 0).map(|non_null| non_null.as_ptr())
}


/// Free previously allocated virtual memory
pub fn free_virtual(ptr: *mut u8) {
    let mut manager = MEMORY_MANAGER.lock();
    manager.free(ptr, 0);
}

/// Map physical memory to virtual address space
pub fn map_physical(
    phys_addr: x86_64::PhysAddr,
    size: usize,
    flags: PageTableFlags,
    // Ajouter la contrainte + Translate ici
    mapper: &mut (impl Mapper<x86_64::structures::paging::Size4KiB> + Translate),
    allocator: &mut impl FrameAllocator<x86_64::structures::paging::Size4KiB>
) -> Result<*mut u8, MemoryError> {
    let mut manager = MEMORY_MANAGER.lock();
    // L'appel ici est maintenant correct car `mapper` a la contrainte Translate
    manager.map_physical(phys_addr, size, flags, mapper, allocator)
        .map(|virt_addr| virt_addr.as_mut_ptr())
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

pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    // Vérifier si déjà initialisé
    static INITIALIZED: AtomicBool = AtomicBool::new(false);

    if INITIALIZED.swap(true, Ordering::SeqCst) {
        // Déjà initialisé, retourner sans erreur
        return Ok(());
    }

    // Initialiser l'allocateur de tas du noyau
    allocator::set_memory_offset_info(boot_info.physical_memory_offset);

    // Initialiser la mémoire physique
    physical::init(boot_info)?;

    // Initialiser le gestionnaire de mémoire virtuelle avec vérification des mappages existants
    r#virtual::init(32)?;

    // Initialiser le support DMA
    dma::init()?;

    Ok(())
}
