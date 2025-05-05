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
pub(crate) use memory_manager::{MemoryError, MemoryManager, PhysicalMemoryManager};
use bootloader::BootInfo;
use x86_64::PhysAddr;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::kernel::memory::r#virtual::VirtualMemoryManager;

// Create thread-safe static reference to the memory manager
lazy_static! {
    static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new());
}

/// Initialize memory management subsystem
pub fn memory_init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    // Vérifier si une page est déjà mappée avant d'essayer de la mapper
    let phys_mem_offset = boot_info.physical_memory_offset;
    crate::kernel::memory::allocator::set_memory_offset_info(phys_mem_offset);

    // Utiliser un verrou pour éviter les initialisations multiples
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    if INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        // Déjà initialisé, ne rien faire
        return Ok(());
    }

    // Initialiser le gestionnaire de mémoire avec vérification des mappages existants
    MemoryManager::init(boot_info)?;

    Ok(())
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
    phys_addr: x86_64::PhysAddr,  // Changé de PhysicalMemoryManager à PhysAddr
    size: usize,
    flags: PageTableFlags,
    mapper: &mut impl Mapper<x86_64::structures::paging::Size4KiB>,
    allocator: &mut impl FrameAllocator<x86_64::structures::paging::Size4KiB>
) -> Result<*mut u8, MemoryError> {
    let mut manager = MEMORY_MANAGER.lock();
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
