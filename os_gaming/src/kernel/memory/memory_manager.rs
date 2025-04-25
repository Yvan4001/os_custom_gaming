//! Memory management subsystem
//!
//! This module handles physical and virtual memory management,
//! including page allocation, memory mapping, and heap allocation.

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::{PhysAddr, VirtAddr};

use crate::kernel::memory::physical;
#[cfg(not(feature = "std"))]
pub(crate) use crate::kernel::memory::physical::PhysicalMemoryManager;
use crate::kernel::memory::r#virtual;
use crate::kernel::memory::r#virtual::free;
use crate::kernel::memory::dma;
use crate::kernel::memory::allocator;
#[cfg(not(feature = "std"))]
use x86_64::structures::paging::{PageTable, PhysFrame, Size4KiB};
use x86_64::structures::paging::{PageSize, PageTableFlags};
use x86_64::structures::paging::Page;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::FrameAllocator;
use core::ptr::NonNull;

/// Memory management error types
#[derive(Debug)]
pub enum MemoryError {
    AllocationFailed,
    InvalidAddress,
    PermissionDenied,
    PageFault,
    OutOfMemory,
    InvalidMapping,
    AlreadyMapped,
    NotMapped,
    InvalidRange,
}

/// Memory protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryProtection {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub user: bool,
    pub cache_type: CacheType
}

/// Cache types for memory regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    Uncacheable,
    WriteCombining,
    WriteThrough,
    WriteProtected,
    WriteBack,
}

/// Memory types for different regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Regular RAM
    Normal,
    /// Memory-mapped device
    Device,
    /// DMA buffer memory
    DMA,
    /// Video/GPU memory
    Video,
    /// Code/executable memory
    Code,
}

lazy_static! {
    /// Track if the memory system has been initialized
    static ref INITIALIZED: AtomicBool = AtomicBool::new(false);
}
pub struct MemoryManager {}

impl MemoryManager {
    /// Create a new memory manager instance
    pub fn new() -> Self {
        MemoryManager {}
    }

    /// Allocate memory with the specified size and alignment
    pub fn allocate(&mut self, size: usize, _align: usize) -> Result<NonNull<u8>, MemoryError> {
        // REMOVE: libc::malloc call
        log::error!("MemoryManager::allocate called directly. Use global allocator (Box, Vec).");
        if size == 0 {
            return Err(MemoryError::InvalidAddress);
        }
        // This function should likely be removed entirely.
        Err(MemoryError::AllocationFailed)
    }

    pub fn deallocate(&mut self, ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
        if ptr.is_null() {
            return Ok(());
        }

        let virt_addr = VirtAddr::from_ptr(ptr);
        unsafe { free(virt_addr, size) }
    }


    /// Free previously allocated memory
    pub fn free(&mut self, ptr: *mut u8, size: usize) -> Result<(), MemoryError> {
        if ptr.is_null() {
            return Ok(());  // Ou retourner une erreur appropriée
        }

        let addr = VirtAddr::new(ptr as u64);
        free(addr, size)
    }


    /// Map physical memory to virtual address space
    pub fn map_physical(
        &mut self,
        physical_address: PhysicalMemoryManager,
        size: usize,
        flags: PageTableFlags,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>
    ) -> Result<VirtAddr, MemoryError> {
        if size == 0 {
            return Err(MemoryError::InvalidRange);
        }

        let phys_addr_value = physical_address.as_u64();
        let start_virt_addr = VirtAddr::new(0x_FFFF_C000_0000_0000 + phys_addr_value);

        let page_range = {
            let start_page = Page::<Size4KiB>::containing_address(start_virt_addr);
            let end_virt_addr = start_virt_addr + u64::try_from(size).unwrap() - 1u64;
            let end_page = Page::<Size4KiB>::containing_address(end_virt_addr);
            Page::range_inclusive(start_page, end_page)
        };

        for page in page_range {
            let frame_offset = page.start_address().as_u64() - start_virt_addr.as_u64();
            let phys_addr = PhysAddr::new(phys_addr_value + frame_offset);
            let frame = PhysFrame::<Size4KiB>::containing_address(phys_addr);

            unsafe {
                match mapper.map_to(page, frame, flags, frame_allocator) {
                    Ok(flush) => flush.flush(),
                    Err(MapToError::FrameAllocationFailed) => return Err(MemoryError::OutOfMemory),
                    Err(MapToError::ParentEntryHugePage) => return Err(MemoryError::InvalidMapping),
                    Err(MapToError::PageAlreadyMapped(_)) => return Err(MemoryError::AlreadyMapped),
                    _ => return Err(MemoryError::InvalidMapping),
                }
            }
        }
        Ok(start_virt_addr)
    }


    /// Get current memory statistics
    pub fn get_stats(&self) -> super::MemoryStats {
        super::MemoryStats {
            total_ram: 0,
            available_ram: 0,
            used_ram: 0,
            total_swap: 0,
            available_swap: 0,
            kernel_heap_used: 0,
        }
    }
    pub fn init() -> Result<(), &'static str> {
        log::info!("Initializing Memory Manager...");
        if INITIALIZED.load(Ordering::SeqCst) {
            return Ok(());
        }

        #[cfg(not(feature = "std"))]
        {
            // Initialize physical memory management first
            physical::init()?;

            // Initialize virtual memory management
            r#virtual::init(32)?;

            // Set up the kernel heap
            allocator::init_heap().map_err(|_| MemoryError::AllocationFailed); // Assuming init_heap returns Result<(), MapToError>
            log::info!("Kernel heap initialized.");

            // Initialize DMA memory management
            dma::init()?;
        }

        INITIALIZED.store(true, Ordering::SeqCst);

        #[cfg(feature = "std")]
        log::info!("Memory management initialized (simulated mode)");

        #[cfg(not(feature = "std"))]
        log::info!("Memory management initialized");

        Ok(())
    }
}

/// Get the current page table
#[cfg(not(feature = "std"))]
pub fn current_page_table() -> &'static mut PageTable {
    let (frame, _) = Cr3::read();
    let phys_addr = frame.start_address();
    let virt_addr = physical::phys_to_virt(phys_addr);

    unsafe { &mut *(virt_addr.as_mut_ptr() as *mut PageTable) }
}

/// Information about system memory
pub fn memory_info() -> MemoryInfo {
    #[cfg(feature = "std")]
    {
        // Simulated memory info in std mode
        MemoryInfo {
            total_ram: 1024 * 1024 * 1024, // 1GB
            free_ram: 512 * 1024 * 1024,   // 512MB
            used_ram: 512 * 1024 * 1024,   // 512MB
            reserved_ram: 0,
            kernel_size: 8 * 1024 * 1024, // 8MB
            page_size: 4096,
        }
    }

    #[cfg(not(feature = "std"))]
    {
        let pmm = physical::get_physical_memory_manager();

        MemoryInfo {
            total_ram: pmm.total_memory(),
            free_ram: pmm.free_memory(),
            used_ram: pmm.used_memory(),
            reserved_ram: pmm.reserved_memory(),
            kernel_size: pmm.kernel_size(),
            page_size: 4096,
        }
    }
}

/// Information about system memory
pub struct MemoryInfo {
    pub total_ram: usize,
    pub free_ram: usize,
    pub used_ram: usize,
    pub reserved_ram: usize,
    pub kernel_size: usize,
    pub page_size: usize,
}

impl MemoryInfo {
    fn new() -> Self {
        Self {
            total_ram: 0,
            free_ram: 0,
            used_ram: 0,
            reserved_ram: 0,
            kernel_size: 0,
            page_size: 4096,
        }
    }
}
