//! Virtual memory management abstractions
//!
//! Handles higher-level virtual address space management.

use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use x86_64::{VirtAddr, PhysAddr}; // PhysAddr might not be needed if only dealing with VA
use x86_64::structures::paging::{
    Page, PageTableFlags, PhysFrame, Size4KiB,
    // Mapper, OffsetPageTable, FrameAllocator // These should not be directly used here now
};
use x86_64::structures::paging::FrameAllocator;
use crate::kernel::memory::{
    physical::{self, PAGE_SIZE}, // Use physical::PAGE_SIZE
    memory_manager::{self, MemoryError, MemoryProtectionFlags, MemoryType, CacheType},
};

/// Manages the kernel's virtual address space.
/// This is a simplified VMM focusing on allocating ranges.
#[derive(Debug)] // Added Debug
pub struct VirtualMemoryManager {
    next_kernel_va: AtomicUsize, // For allocating kernel virtual space
    // TODO: Add tracking for user space, different regions, etc.
    // For simplicity, starting with a linear allocator for a specific kernel region.
    kernel_dynamic_va_start: VirtAddr,
    kernel_dynamic_va_end: VirtAddr,
}

impl VirtualMemoryManager {
    pub const fn new() -> Self {
        // Define a region for dynamic kernel allocations (e.g., for heap, DMA buffers mapped to kernel)
        // These addresses must be chosen carefully to not overlap with kernel code/data, MMIO, etc.
        Self {
            // Example: Start dynamic allocations at 0xFFFF_C000_0000_0000 (adjust as needed)
            next_kernel_va: AtomicUsize::new(0xFFFF_C000_0000_0000),
            kernel_dynamic_va_start: VirtAddr::new(0xFFFF_C000_0000_0000),
            // Example: End at 0xFFFF_DFFF_FFFF_FFFF (256 TiB region for dynamic kernel VA)
            kernel_dynamic_va_end: VirtAddr::new(0xFFFF_DFFF_FFFF_FFFF),
        }
    }

    /// Initializes the VMM. Called once.
    pub fn init_manager(&self) -> Result<(), &'static str> {
        // Any VMM-specific setup can go here.
        // For this simple version, nothing much is needed beyond the lazy_static creation.
        log::info!("Virtual Memory Manager initialized. Kernel dynamic VA: {:?} - {:?}",
            self.kernel_dynamic_va_start, self.kernel_dynamic_va_end);
        Ok(())
    }

    /// Allocates a contiguous block of virtual address space in the kernel's dynamic region.
    /// Does NOT back it with physical memory.
    pub fn allocate_kernel_virtual_range(&self, size: usize, alignment: usize) -> Result<VirtAddr, MemoryError> {
        if size == 0 { return Err(MemoryError::InvalidRange); }
        let align = if alignment == 0 { PAGE_SIZE } else { alignment }; // Default to page alignment
        let alloc_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1); // Round up to page size

        // Simple linear allocation (bump pointer)
        loop {
            let current_base = self.next_kernel_va.load(Ordering::Relaxed);
            let aligned_base = (current_base + align - 1) & !(align - 1);

            if VirtAddr::new(aligned_base as u64) + alloc_size.try_into().unwrap() > self.kernel_dynamic_va_end {
                log::error!("Kernel virtual address space exhausted in dynamic region.");
                return Err(MemoryError::NoMemory); // Or OutOfVirtualAddressSpace
            }

            if self.next_kernel_va.compare_exchange(
                current_base,
                aligned_base + alloc_size,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ).is_ok() {
                return Ok(VirtAddr::new(aligned_base as u64));
            }
            // CAS failed, another thread/core allocated, retry.
            core::hint::spin_loop();
        }
    }

    /// Allocates a virtual address range and backs it with newly allocated physical frames.
    pub fn allocate_and_map_memory(
        &self,
        size: usize,
        protection: MemoryProtectionFlags,
        _mem_type: MemoryType, // mem_type could influence flags or physical memory type
    ) -> Result<VirtAddr, MemoryError> {
        let virt_addr = self.allocate_kernel_virtual_range(size, PAGE_SIZE)?;
        let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        let mut page_flags = PageTableFlags::PRESENT;
        if protection.write { page_flags |= PageTableFlags::WRITABLE; }
        if !protection.execute { page_flags |= PageTableFlags::NO_EXECUTE; }
        // Kernel memory is typically not USER_ACCESSIBLE unless specified
        if protection.user { page_flags |= PageTableFlags::USER_ACCESSIBLE; }
        match protection.cache {
            CacheType::Uncacheable => page_flags |= PageTableFlags::NO_CACHE,
            CacheType::WriteThrough => page_flags |= PageTableFlags::WRITE_THROUGH,
            _ => {}
        }

        for i in 0..num_pages {
            let pmm = physical::get_physical_memory_manager();
            // allocate_frame() from the X64FrameAllocator trait
            let frame = pmm.allocate_frame().ok_or(MemoryError::OutOfMemory)?;
            let page = Page::containing_address(virt_addr + (i * PAGE_SIZE) as u64);

            memory_manager::map_page_for_kernel(page, frame, page_flags)
                .map_err(|e| {
                    log::error!("Failed to map page {:?} for new allocation: {:?}", page, e);
                    // TODO: Unmap already mapped pages in this allocation and free frames
                    MemoryError::InvalidMapping
                })?
                .flush();
        }
        Ok(virt_addr)
    }

    /// Frees a previously allocated (and mapped) virtual memory region.
    /// Unmaps the pages and frees the corresponding physical frames.
    pub fn free_and_unmap_memory(&self, virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
        if size == 0 { return Ok(()); }
        let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        // To free physical frames, we need to know which frames were backing this VA range.
        // This requires translating each virtual page to its physical frame *before* unmapping,
        // or if the unmap operation returned the frame.
        // memory_manager::unmap_region does not return frames.
        // We need a way to query the mapping.
        // For now, this is a simplified version that only unmaps.
        // A full VMM would track mappings or use a translate function.

        // Step 1: Translate and collect frames (conceptual, needs mapper access)
        let mut frames_to_free: Vec<PhysFrame> = Vec::new();
        // This part needs access to the global mapper, e.g., via a memory_manager function.
        // For now, this is a placeholder for how it *should* work.
        // let mm_guard = memory_manager::MEMORY_MANAGER.lock(); // If MM had a translate_page_global
        // for i in 0..num_pages {
        //     let page = Page::containing_address(virt_addr + (i * PAGE_SIZE) as u64);
        //     if let Some(frame) = mm_guard.translate_page_global(page) { // translate_page_global would be a new fn
        //         frames_to_free.push(frame);
        //     }
        // }
        // drop(mm_guard);

        // Step 2: Unmap the region
        memory_manager::unmap_region(virt_addr, size)?;

        // Step 3: Free the collected physical frames (if collected)
        // let pmm = physical::get_physical_memory_manager();
        // for frame in frames_to_free {
        //     pmm.free_phys_addr(frame.start_address());
        // }
        log::warn!("free_and_unmap_memory in virtual.rs: Physical frame freeing is currently conceptual and not fully implemented due to needing pre-unmap translation.");


        // TODO: Mark the virtual address range [virt_addr, virt_addr+size) as free in this VMM's tracking.
        // (Not implemented in this simple linear allocator).
        Ok(())
    }
}

lazy_static! {
    static ref VIRTUAL_MEMORY_MANAGER: VirtualMemoryManager = VirtualMemoryManager::new();
}

/// Public function to initialize the VMM. Called from `memory::init`.
pub fn init_virtual_manager() -> Result<(), &'static str> {
    VIRTUAL_MEMORY_MANAGER.init_manager()
}

// --- Public API for this virtual memory module ---

pub fn allocate_and_map(
    size: usize,
    protection: MemoryProtectionFlags,
    mem_type: MemoryType,
) -> Result<VirtAddr, MemoryError> {
    VIRTUAL_MEMORY_MANAGER.allocate_and_map_memory(size, protection, mem_type)
}

pub fn free_and_unmap(addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
    VIRTUAL_MEMORY_MANAGER.free_and_unmap_memory(addr, size)
}

// REMOVED: map, unmap, map_physical_memory, map_physical_region, protect functions
// that took an external `mapper`. These operations should be requested through
// memory_manager.rs services or high-level VMM functions like allocate_and_map.
// The VMM's role is more about VA space management.
