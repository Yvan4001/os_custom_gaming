//! Kernel heap allocator

#[cfg(not(feature = "std"))]
use linked_list_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
use x86_64::VirtAddr;

#[cfg(not(feature = "std"))]
use x86_64::PhysAddr;

#[cfg(not(feature = "std"))]
use x86_64::structures::paging::{
    Page, PhysFrame, Size4KiB, PageTableFlags,
    mapper::MapToError,
    // FrameAllocator as X64FrameAllocator, // Not needed if PMM is used directly
};

// MODIFIED: Use services from memory_manager and physical
#[cfg(not(feature = "std"))]
use crate::kernel::memory::{memory_manager, physical};

/// Start address of the kernel heap. Ensure this VA is available.
#[cfg(not(feature = "std"))]
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Size of the kernel heap.
#[cfg(not(feature = "std"))]
pub const HEAP_SIZE: usize = 256 * 1024; // 256 KiB

#[cfg(not(feature = "std"))]
#[global_allocator]
static ALLOCATOR: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

/// Initializes the kernel heap.
/// Maps the virtual memory range for the heap and initializes `ALLOCATOR`.
/// Called by `MemoryManager::init_services`.
#[cfg(not(feature = "std"))]
#[cfg(not(feature = "std"))]
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    log::info!(
        "Initializing kernel heap at VAddr {:#x} (size: {:#x} bytes)",
        HEAP_START,
        HEAP_SIZE
    );

    if HEAP_SIZE == 0 {
        log::error!("HEAP_SIZE cannot be zero for kernel heap.");
        return Err(MapToError::FrameAllocationFailed); // Or a more specific error if available
    }

    let heap_start_va = VirtAddr::new(HEAP_START as u64);
    // HEAP_SIZE must be page aligned for this to work perfectly, or adjust last page.
    // For simplicity, assume HEAP_SIZE is a multiple of PAGE_SIZE or that Page::range_inclusive handles it.
    let heap_end_va = heap_start_va + HEAP_SIZE as u64 - 1u64; // Inclusive end

    let heap_start_page = Page::<Size4KiB>::containing_address(heap_start_va);
    let heap_end_page = Page::<Size4KiB>::containing_address(heap_end_va);
    let page_range = Page::range_inclusive(heap_start_page, heap_end_page);

    let heap_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

    for page_to_map in page_range {
        // 1. Allocate a new physical frame for the current heap page
        let allocated_phys_addr = match physical::get_physical_memory_manager().allocate_phys_addr() {
            Some(addr) => addr,
            None => {
                log::error!(
                    "Heap init: Ran out of physical frames while allocating for virtual page {:?}",
                    page_to_map
                );
                // TODO: Attempt to unmap already mapped heap pages in this function before returning.
                return Err(MapToError::FrameAllocationFailed);
            }
        };

        // Convert the allocated PhysAddr to a PhysFrame for the mapping function
        let frame_to_map = PhysFrame::<Size4KiB>::containing_address(allocated_phys_addr);

        log::trace!(
            "Heap init: Mapping VP {:?} to PF {:?}",
            page_to_map,
            frame_to_map
        );

        // 2. Map the current heap page to the newly allocated physical frame
        match memory_manager::map_page_for_kernel(page_to_map, frame_to_map, heap_flags) {
            Ok(flush) => flush.flush(), // IMPORTANT: Flush the TLB for this mapping
            // MODIFIED: Dereference map_err or remove 'ref' because MapToError is Copy.
            // Using `err` directly as it's Copy.
            Err(err @ MapToError::PageAlreadyMapped(_)) => {
                // map_page_for_kernel should return Ok if identically mapped.
                // If it returns an error here, it's likely a conflict with a *different* mapping for this high-mem VA page.
                log::error!(
                    "Heap init: Virtual page {:?} mapping conflict (already mapped differently). Error: {:?}. Freeing allocated frame {:?}",
                    page_to_map,
                    err, // Log the error directly
                    frame_to_map // Log the frame we tried to map to
                );
                // Free the physical frame we just allocated but couldn't use
                physical::get_physical_memory_manager().free_phys_addr(allocated_phys_addr);
                return Err(err); // Return the error directly (it's Copy)
            }
            // MODIFIED: Using `err` directly as it's Copy.
            Err(err) => {
                log::error!(
                    "Heap init: Failed to map virtual page {:?} to physical frame {:?}: {:?}",
                    page_to_map,
                    frame_to_map,
                    err // Log the error directly
                );
                // Free the physical frame we just allocated but couldn't use
                physical::get_physical_memory_manager().free_phys_addr(allocated_phys_addr);
                return Err(err); // Return the error directly (it's Copy)
            }
        }
    }

    // Initialize the LockedHeap allocator with the base address and size of the mapped region
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    log::info!(
        "Kernel heap initialized successfully. Usable virtual range: {:#x} - {:#x}",
        HEAP_START,
        HEAP_START + HEAP_SIZE // This is exclusive end for range display
    );
    Ok(())
}

#[cfg(feature = "std")]
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    log::info!("Heap initialization skipped in std/test mode.");
    Ok(())
}
