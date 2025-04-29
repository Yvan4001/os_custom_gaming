//! Memory management subsystem
//!
//! This module handles physical and virtual memory management,
//! including page allocation, memory mapping, and heap allocation.

use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::registers::control::Cr3;

use crate::kernel::memory::physical;
#[cfg(not(feature = "std"))]
pub(crate) use crate::kernel::memory::physical::PhysicalMemoryManager;
use crate::kernel::memory::r#virtual;
use crate::kernel::memory::r#virtual::free;
use crate::kernel::memory::dma;
use crate::kernel::memory::allocator;
#[cfg(not(feature = "std"))]
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, Page, PageTable, PhysFrame, Size4KiB,
        OffsetPageTable, PageTableFlags
    },
    VirtAddr, PhysAddr,
};
use x86_64::structures::paging::mapper::TranslateError;
use x86_64::structures::paging::page_table::PageTableEntry;

use core::ptr::NonNull;
use bootloader::BootInfo;
use x86_64::structures::paging::mapper::MapToError;
extern crate alloc;
use alloc::string::String;
use crate::kernel::memory::physical::PAGE_SIZE;
use x86_64::structures::paging::mapper::MapperFlush;

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

#[derive(Debug)]
pub enum MemoryInitError {
    PageTableCreationFailed,
    PhysicalMemoryInitFailed,
    VirtualMemoryInitFailed,
    HeapInitFailed,
    DmaInitFailed,
}

impl From<MemoryInitError> for &'static str {
    fn from(error: MemoryInitError) -> &'static str {
        match error {
            MemoryInitError::PageTableCreationFailed => "Failed to create page tables",
            MemoryInitError::PhysicalMemoryInitFailed => "Failed to initialize physical memory",
            MemoryInitError::VirtualMemoryInitFailed => "Failed to initialize virtual memory",
            MemoryInitError::HeapInitFailed => "Failed to initialize heap",
            MemoryInitError::DmaInitFailed => "Failed to initialize DMA",
        }
    }
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

pub fn create_page_mapping(
    page: Page<Size4KiB>,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    // Check if the page is already mapped
    if let Ok(mapped_frame) = mapper.translate_page(page) {
        if mapped_frame == frame {
            // If the page is already mapped to the same frame, return success
            return Ok(());
        } else {
            // If the page is mapped to a different frame, return an error
            return Err("Page already mapped to a different frame");
        }
    }

    // Attempt to map the page to the frame
    unsafe {
        match mapper.map_to(page, frame, flags, frame_allocator) {
            Ok(flush) => {
                flush.flush(); // Ensure the mapping is flushed
                Ok(())
            }
            Err(MapToError::PageAlreadyMapped(existing_frame)) => {
                // Handle the case where the page is already mapped
                if existing_frame == frame {
                    // If it's mapped to the same frame, return success
                    Ok(())
                } else {
                    // Otherwise, return an error
                    Err("Page already mapped to a different frame")
                }
            }
            Err(MapToError::FrameAllocationFailed) => Err("Failed to allocate frame"),
            Err(MapToError::ParentEntryHugePage) => Err("Parent entry is a huge page"),
            _ => Err("Invalid mapping"),
        }
    }
}

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
    unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
        use x86_64::registers::control::Cr3;

        let (level_4_table_frame, _) = Cr3::read();
        let phys = level_4_table_frame.start_address();
        let virt = phys_mem_offset + phys.as_u64();
        let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

        &mut *page_table_ptr
    }

    unsafe fn create_page_tables(phys_mem_offset: VirtAddr) -> Result<OffsetPageTable<'static>, &'static str> {
        let level_4_table = Self::active_level_4_table(phys_mem_offset);
        Ok(OffsetPageTable::new(level_4_table, phys_mem_offset))
    }

    pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
        log::info!("Initializing Memory Manager...");
        if INITIALIZED.load(Ordering::SeqCst) {
            return Ok(());
        }

        #[cfg(not(feature = "std"))]
        {
            // Get the physical memory offset
            let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

            // Initialize physical memory management first
            physical::init(boot_info)?;

            // Create page tables
            let mut page_tables = unsafe { Self::create_page_tables(phys_mem_offset)? };

            // Initialize virtual memory management
            r#virtual::init(4096)?;

            // Set up the kernel heap
            allocator::init_heap()
                .map_err(|_| "Failed to initialize kernel heap")?;
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
