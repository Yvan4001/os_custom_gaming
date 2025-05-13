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
pub const HEAP_START: usize = 0x_4444_4444_0000; // Example: 256TiB + 1MiB region (adjust as needed)
/// Size of the kernel heap.
#[cfg(not(feature = "std"))]
pub const HEAP_SIZE: usize = 256 * 1024; // 256 KiB (can be grown later if needed)

#[cfg(not(feature = "std"))]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initializes the kernel heap.
/// Maps the virtual memory range for the heap and initializes `ALLOCATOR`.
/// Called by `MemoryManager::init_services`.
#[cfg(not(feature = "std"))]
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    log::info!("Initializing kernel heap at {:#x} (size: {:#x} bytes)", HEAP_START, HEAP_SIZE);

    if HEAP_SIZE == 0 {
        log::error!("HEAP_SIZE cannot be zero for kernel heap.");
        // This error type isn't perfect, but MapToError is what init_services expects.
        return Err(MapToError::FrameAllocationFailed);
    }

    let heap_start_va = VirtAddr::new(HEAP_START as u64);
    let heap_end_va = heap_start_va + HEAP_SIZE as u64 - 1u64; // Inclusive end
    let heap_start_pa = heap_start_va.as_u64() as u64;
    let heap_end_pa = heap_end_va.as_u64() as u64;
    let phys_addr = PhysFrame::containing_address(PhysAddr::new(heap_start_pa));

    let heap_start_page = Page::<Size4KiB>::containing_address(heap_start_va);
    let heap_end_page = Page::<Size4KiB>::containing_address(heap_end_va);
    let page_range = Page::range_inclusive(heap_start_page, heap_end_page);

    // Standard flags for kernel heap: Present, Writable, NoExecute
    let heap_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

    for page in page_range {
        // 1. Allocate a physical frame using the global PMM
        let frame = match physical::get_physical_memory_manager().allocate_phys_addr() {
             // allocate_frame() is from the X64FrameAllocator trait impl on PhysicalMemoryManager
            Some(f) => f,
            None => {
                log::error!("Heap init: Ran out of physical frames for page {:?}", page);
                // TODO: Attempt to unmap already mapped heap pages before failing.
                return Err(MapToError::FrameAllocationFailed);
            }
        };

        log::trace!("Heap init: Mapping page {:?} to frame {:?}", page, frame);

        // 2. Map the page using the memory_manager's service
        match memory_manager::map_page_for_kernel(page, phys_addr, heap_flags) {
            Ok(flush) => flush.flush(), // IMPORTANT: Flush the TLB
            Err(e @ MapToError::PageAlreadyMapped(_)) => {
                // map_page_for_kernel should return Ok if identically mapped.
                // If it returns PageAlreadyMapped error, it's a conflict.
                log::error!("Heap init: Page {:?} mapping conflict (already mapped differently). Error: {:?}. Freeing frame {:?}", page, e, frame);
                physical::get_physical_memory_manager().free_phys_addr(phys_addr.start_address());
                return Err(e);
            }
            Err(e) => {
                log::error!("Heap init: Failed to map page {:?} to frame {:?}: {:?}", page, frame, e);
                physical::get_physical_memory_manager().free_phys_addr(phys_addr.start_address());
                return Err(e);
            }
        }
    }

    // Initialize the LockedHeap with the mapped virtual memory region
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    log::info!("Kernel heap initialized. Usable range: {:#x} - {:#x}", HEAP_START, HEAP_START + HEAP_SIZE);
    Ok(())
}

#[cfg(feature = "std")]
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    log::info!("Heap initialization skipped in std/test mode.");
    Ok(())
}
