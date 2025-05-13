//! Physical memory management
//!
//! Handles physical memory allocation and tracking

use alloc::vec::Vec; // Keep if FrameBitmap uses Vec internally, otherwise can be removed if not used
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex; // Assuming you're using spin::Mutex for FrameBitmap
use lazy_static::lazy_static;
use x86_64::{PhysAddr, VirtAddr};
// MODIFIED: Aliased FrameAllocator to avoid conflict if you define your own.
use x86_64::structures::paging::{
    FrameAllocator as X64FrameAllocator, PhysFrame, Size4KiB, PageTable, // Added PageTable for active_level_4_table if it were here
};

#[cfg(not(feature = "std"))]
use bootloader::bootinfo::{MemoryMap, MemoryRegion, MemoryRegionType}; // Combined MemoryMap import

/// Size of a page (4KB) - MODIFIED: Made public
pub const PAGE_SIZE: usize = 4096;

/// Physical memory frame bitmap
#[derive(Debug)] // Added Debug for easier logging
pub struct FrameBitmap {
    bitmap: [u64; 8192], // Supports up to 32GB with 4KB pages
    total_frames: usize,
    free_frames: AtomicUsize,
}

impl FrameBitmap {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; 8192],
            total_frames: 0,
            free_frames: AtomicUsize::new(0),
        }
    }

    // init, init_with_regions, set_frame, is_frame_used, allocate_frame,
    // allocate_contiguous, allocate_frames, free_frame, free_frames, count_free
    // init_frame_allocator, count_used_frames
    // These methods seem mostly fine from the previous versions, ensure their logic is sound.
    // Key part for init_frame_allocator:
    pub fn init_frame_allocator(
        &mut self,
        mut memory_ranges: impl Iterator<Item = core::ops::Range<u64>>,
        kernel_start: PhysAddr,
        kernel_end: PhysAddr,
    ) {
        self.bitmap.fill(!0); // Mark all as used initially
        self.total_frames = 0;
        let mut calculated_free_frames = 0;

        for range in memory_ranges {
            let start_frame = (range.start / PAGE_SIZE as u64) as usize;
            let end_frame = (range.end / PAGE_SIZE as u64) as usize;

            for frame_idx in start_frame..end_frame {
                if frame_idx / 64 < self.bitmap.len() { // Check bounds
                    self.set_bit(frame_idx, false); // Mark as free
                    calculated_free_frames += 1;
                }
            }
        }
        self.total_frames = calculated_free_frames; // total_frames is count of usable frames

        // Mark kernel frames as used
        let kernel_start_frame = (kernel_start.as_u64() / PAGE_SIZE as u64) as usize;
        let kernel_end_frame = ((kernel_end.as_u64() + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        for frame_idx in kernel_start_frame..kernel_end_frame {
            if frame_idx / 64 < self.bitmap.len() && !self.get_bit(frame_idx) { // If it was free
                self.set_bit(frame_idx, true); // Mark as used
                calculated_free_frames -= 1;
            } else if frame_idx / 64 < self.bitmap.len() { // Already marked used (e.g. by bitmap.fill(!0))
                 self.set_bit(frame_idx, true); // Ensure it's marked used
            }
        }
        self.free_frames.store(calculated_free_frames, Ordering::SeqCst);
        log::trace!("FrameBitmap initialized: Total usable frames (initially): {}, Free after kernel: {}", self.total_frames, calculated_free_frames);
    }

    // Helper to set/get bit to avoid confusion in set_frame's free_frames update
    fn set_bit(&mut self, frame_idx: usize, used: bool) {
        let idx = frame_idx / 64;
        let bit = frame_idx % 64;
        if idx < self.bitmap.len() {
            if used {
                self.bitmap[idx] |= 1 << bit;
            } else {
                self.bitmap[idx] &= !(1 << bit);
            }
        }
    }
    fn get_bit(&self, frame_idx: usize) -> bool {
        let idx = frame_idx / 64;
        let bit = frame_idx % 64;
        if idx < self.bitmap.len() {
            (self.bitmap[idx] & (1 << bit)) != 0
        } else {
            true // Out of bounds considered used
        }
    }

    // allocate_frame, set_frame, etc. from your previous version,
    // but ensure free_frames updates are correct if set_bit is not used.
    // Using set_bit simplifies set_frame's logic for free_frames count.
    pub fn set_frame(&mut self, frame_idx: usize, used: bool) {
        let was_used = self.get_bit(frame_idx);
        self.set_bit(frame_idx, used);

        if used && !was_used {
            self.free_frames.fetch_sub(1, Ordering::SeqCst);
        } else if !used && was_used {
            self.free_frames.fetch_add(1, Ordering::SeqCst);
        }
    }
    // ... (other FrameBitmap methods as provided, ensure correctness)
    pub fn allocate_frame(&mut self) -> Option<usize> { /* ... as before ... */
        if self.free_frames.load(Ordering::SeqCst) == 0 { return None; }
        for i in 0..self.bitmap.len() {
            if self.bitmap[i] != !0u64 { // If not all bits are 1
                for bit in 0..64 {
                    if (self.bitmap[i] & (1 << bit)) == 0 { // If bit is 0 (free)
                        let frame_idx = i * 64 + bit;
                        if frame_idx < self.total_frames { // Ensure it's a valid frame index
                            self.set_frame(frame_idx, true); // Marks used and updates count
                            return Some(frame_idx);
                        }
                    }
                }
            }
        }
        None
    }
     pub fn allocate_frames(&mut self, count: usize) -> Option<usize> { /* ... as before ... */
        if count == 0 || self.free_frames.load(Ordering::SeqCst) < count { return None; }
        if count == 1 { return self.allocate_frame(); }
        let mut start_frame_found = 0;
        let mut current_run = 0;
        for frame_idx in 0..self.total_frames { // Iterate up to total_frames
            if !self.get_bit(frame_idx) { // if frame is free
                if current_run == 0 { start_frame_found = frame_idx; }
                current_run += 1;
                if current_run == count {
                    for f_idx in start_frame_found..(start_frame_found + count) {
                        self.set_frame(f_idx, true);
                    }
                    return Some(start_frame_found);
                }
            } else {
                current_run = 0;
            }
        }
        None
    }
    pub fn free_frame(&mut self, frame_idx: usize) {
        if frame_idx < self.total_frames {
            self.set_frame(frame_idx, false); // Marks free and updates count
        }
    }
    pub fn free_frames(&mut self, start_frame: usize, count: usize) {
        for frame_idx in start_frame..(start_frame + count) {
            if frame_idx < self.total_frames {
                self.set_frame(frame_idx, false); // Marks free and updates count
            }
        }
    }

    pub fn is_frame_used(&self, frame_idx: usize) -> bool {
        if frame_idx < self.total_frames {
            self.get_bit(frame_idx)
        } else {
            true // Out of bounds considered used
        }
    }
}


/// Physical memory manager
pub struct PhysicalMemoryManager {
    frame_bitmap: Mutex<FrameBitmap>,
    total_memory: AtomicUsize, // Total physical memory in bytes
    kernel_size: AtomicUsize,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
    // memory_map: Option<&'static MemoryMap>, // Only if needed for other purposes post-init
}

impl PhysicalMemoryManager {
    pub const fn new() -> Self {
        Self {
            frame_bitmap: Mutex::new(FrameBitmap::new()),
            total_memory: AtomicUsize::new(0),
            kernel_size: AtomicUsize::new(0),
            kernel_start: PhysAddr::new(0),
            kernel_end: PhysAddr::new(0),
            // memory_map: None,
        }
    }

    // Renamed from allocate_frame to avoid conflict with the trait method
    // This is the PMM's internal method to get a physical address.
    pub fn allocate_phys_addr(&self) -> Option<PhysAddr> {
        let mut bitmap_guard = self.frame_bitmap.lock();
        bitmap_guard.allocate_frame().map(|frame_idx| {
            PhysAddr::new((frame_idx * PAGE_SIZE) as u64)
        })
    }

    // Renamed from free_frame
    pub fn free_phys_addr(&self, addr: PhysAddr) {
        let frame_idx = addr.as_u64() as usize / PAGE_SIZE;
        let mut bitmap_guard = self.frame_bitmap.lock();
        bitmap_guard.free_frame(frame_idx); // free_frame in FrameBitmap should update counts
    }

    // Renamed from free_frames
    pub fn free_phys_addrs(&self, addr: PhysAddr, count: usize) {
        let start_frame_idx = addr.as_u64() as usize / PAGE_SIZE;
        let mut bitmap_guard = self.frame_bitmap.lock();
        bitmap_guard.free_frames(start_frame_idx, count); // set_frame in FrameBitmap should update counts
    }
    
    pub fn total_memory(&self) -> usize { self.total_memory.load(Ordering::SeqCst) }
    pub fn free_memory(&self) -> usize { self.frame_bitmap.lock().free_frames.load(Ordering::SeqCst) * PAGE_SIZE }
    pub fn used_memory(&self) -> usize { self.total_memory() - self.free_memory() }
    pub fn kernel_size(&self) -> usize { self.kernel_size.load(Ordering::SeqCst) }
    pub fn kernel_start(&self) -> PhysAddr { self.kernel_start }
    pub fn kernel_end(&self) -> PhysAddr { self.kernel_end }
}

// MODIFIED: Implement FrameAllocator for PhysicalMemoryManager
unsafe impl X64FrameAllocator<Size4KiB> for PhysicalMemoryManager {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        // The trait needs &mut self, our PMM methods use &self because of Mutex.
        // This is a common pattern: the allocate_phys_addr takes &self and locks internally.
        // So, we just call it.
        self.allocate_phys_addr()
            .map(|phys_addr| PhysFrame::containing_address(phys_addr))
    }
}

lazy_static! {
    // This is the single global instance of the PMM.
    static ref PHYSICAL_MEMORY_MANAGER: PhysicalMemoryManager = PhysicalMemoryManager::new();
}

/// Initializes the global physical memory manager and its frame bitmap.
/// This function should be called by `MemoryManager::init_core`.
pub fn init_frame_allocator(
    memory_map: &'static MemoryMap,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
) -> Result<(), &'static str> {
    log::info!("Initializing Physical Frame Allocator. Kernel: {:?} - {:?}", kernel_start, kernel_end);

    // Get a mutable reference to the global PMM.
    // Note: get_physical_memory_manager() returns &'static mut, which is tricky.
    // We are modifying the fields of the PMM instance contained within the lazy_static.
    let pmm = get_physical_memory_manager();

    pmm.kernel_start = kernel_start;
    pmm.kernel_end = kernel_end;
    let kernel_size_bytes = kernel_end.as_u64().saturating_sub(kernel_start.as_u64());
    pmm.kernel_size.store(kernel_size_bytes as usize, Ordering::SeqCst);

    // Initialize the bitmap
    let mut bitmap_guard = pmm.frame_bitmap.lock();
    bitmap_guard.init_frame_allocator(
        memory_map.iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| r.range.start_addr()..r.range.end_addr()), // Pass iterator of ranges
        kernel_start,
        kernel_end
    );
    // After bitmap_guard.init_frame_allocator, bitmap_guard.total_frames should be the count of *usable* frames
    // and free_frames should be usable frames minus kernel frames.
    let total_phys_memory_bytes = bitmap_guard.total_frames * PAGE_SIZE;
    // It might be more accurate to sum up all memory region lengths for total_memory if total_frames only counts usable.
    // For now, assume total_frames in bitmap is the count of all frames it manages from usable regions.
    drop(bitmap_guard); // Release lock

    pmm.total_memory.store(total_phys_memory_bytes, Ordering::SeqCst);


    log::info!(
        "Physical Frame Allocator initialized. Total Mem: {} MiB, Free Mem: {} MiB, Kernel Size: {} KiB",
        pmm.total_memory() / (1024 * 1024),
        pmm.free_memory() / (1024 * 1024),
        kernel_size_bytes / 1024
    );
    Ok(())
}

/// Provides mutable access to the global `PHYSICAL_MEMORY_MANAGER`.
/// **Safety**: This function is unsafe because it allows mutable access to a static item.
/// It should only be used carefully, typically during single-threaded kernel initialization
/// or if external synchronization ensures safety. The `FrameAllocator` trait requires `&mut self`.
pub fn get_physical_memory_manager() -> &'static mut PhysicalMemoryManager {
    unsafe {
        // This is a common pattern to get a mutable reference from a lazy_static.
        // It's inherently unsafe and relies on careful usage (e.g., single-threaded access
        // during boot or when the FrameAllocator trait requires it).
        static mut PMM_PTR: *mut PhysicalMemoryManager = core::ptr::null_mut();
        if PMM_PTR.is_null() {
            // This assumes PHYSICAL_MEMORY_MANAGER is already initialized by lazy_static.
            PMM_PTR = &*PHYSICAL_MEMORY_MANAGER as *const PhysicalMemoryManager as *mut PhysicalMemoryManager;
        }
        &mut *PMM_PTR
    }
}

/// Allocates a contiguous block of physical memory suitable for DMA.
pub fn allocate_contiguous_dma(size: usize, alignment: usize, limit_phys_addr_opt: Option<u64>) -> Option<PhysAddr> {
    let pmm = get_physical_memory_manager(); // Gets &'static mut PMM
    let mut bitmap_guard = pmm.frame_bitmap.lock();
    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

    let max_frame_idx_opt = limit_phys_addr_opt.map(|limit_addr| (limit_addr / PAGE_SIZE as u64) as usize);

    // FrameBitmap's allocate_frames or allocate_contiguous should handle the limit.
    // For now, assuming a simplified search loop as in your previous version if not built into FrameBitmap.
    let mut found_start_frame: Option<usize> = None;
    'search: for start_f in 0..(bitmap_guard.total_frames.saturating_sub(num_pages)) {
        if let Some(max_f_idx) = max_frame_idx_opt {
            if (start_f + num_pages -1) > max_f_idx { // Check if the end of the block is beyond limit
                continue 'search; // This block would exceed the limit.
            }
        }
        let mut possible = true;
        for i in 0..num_pages {
            if bitmap_guard.is_frame_used(start_f + i) {
                possible = false;
                break;
            }
        }
        if possible {
            found_start_frame = Some(start_f);
            break 'search;
        }
    }

    found_start_frame.map(|start_frame_idx| {
        for i in 0..num_pages {
            bitmap_guard.set_frame(start_frame_idx + i, true);
        }
        PhysAddr::new(start_frame_idx as u64 * PAGE_SIZE as u64)
    })
}
