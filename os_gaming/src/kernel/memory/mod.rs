//! Kernel Memory Management Subsystem
//!
//! This module orchestrates the initialization and provides high-level access
//! to memory-related functionalities.

pub mod allocator;
pub mod dma;
pub mod r#virtual;
pub mod physical;
pub mod memory_manager;
use x86_64::{PhysAddr, VirtAddr, structures::paging::PageTableFlags};

// Re-export important types for convenience
use crate::boot::info::{CustomBootInfo, MemoryRegion, MemoryRegionType};
use crate::kernel::memory::memory_manager::{self as mm, MemoryInitError, MemoryError, MemoryProtectionFlags, CacheType, MemoryType, MemoryInfo};
use crate::kernel::memory::physical::{self as pmm, PAGE_SIZE};
use core::sync::atomic::Ordering;
use alloc::string::String;
use core::sync::atomic::AtomicBool;
use core::ptr::NonNull;

pub use memory_manager::{
    MemoryError as KernelMemoryError, // Using alias for clarity if needed elsewhere
    MemoryProtectionFlags as KernelMemoryProtectionFlags,
    CacheType as KernelCacheType,
    MemoryType as KernelMemoryType,
    MemoryInfo as KernelMemoryInfo,
    MemoryInitError as KernelMemoryInitError, // Re-export if needed
    map_page_for_kernel, map_physical_memory, unmap_region, get_physical_memory_offset
};

static MEMORY_SYSTEM_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initializes the entire memory subsystem.
pub fn init(boot_info: &'static CustomBootInfo) -> Result<(), &'static str> {
    if MEMORY_SYSTEM_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
        log::warn!("Memory subsystem already initialized.");
        return Ok(());
    }
    log::info!("Memory Subsystem: init(CustomBootInfo) called.");

    if let Err(e) = adapt_and_init_core_memory(boot_info) {
         let err_msg: &'static str = e.into();
         log::error!("Core memory manager initialization failed: {}", err_msg);
         MEMORY_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
         return Err(err_msg);
    }
    log::info!("Core memory manager (PMM & Mapper) initialized.");

    if let Err(e) = memory_manager::MemoryManager::init_services() {
        let err_msg: &'static str = e.into();
        log::error!("Memory services (Heap/DMA) initialization failed: {}", err_msg);
        MEMORY_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
        return Err(err_msg);
    }
    log::info!("Memory services (Heap & DMA) initialized.");
    log::info!("Main memory initialization complete.");
    Ok(())
}

fn adapt_and_init_core_memory(custom_boot_info: &'static CustomBootInfo) -> Result<(), MemoryInitError> {
    log::debug!("Adapting CustomBootInfo for core memory initialization...");

    let phys_mem_offset_u64 = custom_boot_info.physical_memory_offset
        .ok_or(MemoryInitError::PhysicalOffsetMissing)?;
    let phys_mem_offset_val = VirtAddr::new(phys_mem_offset_u64);
    memory_manager::PHYSICAL_MEMORY_OFFSET.store(phys_mem_offset_u64, Ordering::SeqCst);
    log::debug!("Physical memory offset from CustomBootInfo: {:#x}", phys_mem_offset_u64);

    let kernel_start_phys = PhysAddr::new(unsafe { &memory_manager::__kernel_physical_start as *const _ as u64 });
    let kernel_end_phys = PhysAddr::new(unsafe { &memory_manager::__kernel_physical_end as *const _ as u64 });
    if kernel_start_phys.is_null() || kernel_end_phys.is_null() || kernel_start_phys >= kernel_end_phys {
        return Err(MemoryInitError::KernelAddressMissing);
    }

    // MODIFIED: Use custom_boot_info.memory_map_regions directly
    let memory_map_regions_slice = &custom_boot_info.memory_map_regions;
    if memory_map_regions_slice.is_empty() && custom_boot_info.physical_memory_offset.is_some() { // Check if map is empty but expected
        log::warn!("Memory map from CustomBootInfo is empty, PMM might not initialize correctly.");
        // Depending on strictness, you might return an error here:
        // return Err(MemoryInitError::PhysicalMemoryInitFailed("Empty memory map".into()));
    }
    log::debug!("Passing {} memory regions from CustomBootInfo to PMM init.", memory_map_regions_slice.len());

    physical::init_frame_allocator(
        memory_map_regions_slice.iter(), // Iterates over &MemoryRegion
        kernel_start_phys,
        kernel_end_phys
    ).map_err(|e_str| MemoryInitError::PhysicalMemoryInitFailed(String::from(e_str)))?;
    log::info!("Physical Frame Allocator (PMM) initialized with custom map.");

    let mut mm_guard = memory_manager::MEMORY_MANAGER.lock();
    // MODIFIED: Make create_page_tables pub(crate) in memory_manager.rs
    let page_tables = unsafe {
        crate::kernel::memory::memory_manager::MemoryManager::create_page_tables(phys_mem_offset_val)
    }?;
    // MODIFIED: Make mapper field pub(crate) in memory_manager.rs or add a setter
    mm_guard.mapper = Some(page_tables);
    drop(mm_guard);

    log::info!("Core Mapper initialized with CustomBootInfo offset.");
    memory_manager::CORE_MM_INITIALIZED.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn alloc_virtual_backed_memory(size: usize, protection: MemoryProtectionFlags, mem_type: MemoryType) -> Result<NonNull<u8>, MemoryError> {
    if !memory_manager::CORE_MM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Err(MemoryError::InvalidRange); }
    r#virtual::allocate_and_map(size, protection, mem_type)
        .map(|vaddr| NonNull::new(vaddr.as_mut_ptr()).ok_or(MemoryError::AllocationFailed))?
}
pub fn free_virtual_backed_memory(ptr: NonNull<u8>, size: usize) -> Result<(), MemoryError> {
    if !memory_manager::CORE_MM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Ok(()); }
    r#virtual::free_and_unmap(VirtAddr::from_ptr(ptr.as_ptr()), size)
}
pub fn map_phys_mem_to_kernel_virt(phys_addr: PhysAddr, size: usize, flags: PageTableFlags) -> Result<VirtAddr, MemoryError> {
    if !memory_manager::CORE_MM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Err(MemoryError::InvalidRange); }
    memory_manager::map_physical_memory(phys_addr, size, flags)
}
pub fn unmap_kernel_virt_region(virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
    if !memory_manager::CORE_MM_INITIALIZED.load(Ordering::Acquire) { return Err(MemoryError::InvalidState); }
    if size == 0 { return Ok(()); }
    memory_manager::unmap_region(virt_addr, size)
}
pub fn get_memory_statistics() -> MemoryInfo {
    if !memory_manager::CORE_MM_INITIALIZED.load(Ordering::Acquire) {
        return MemoryInfo { total_ram:0, free_ram:0, used_ram:0, reserved_ram:0, kernel_size:0, page_size: PAGE_SIZE};
    }
    memory_manager::memory_info()
}
