//! Physical memory management
//!
//! Handles physical memory allocation and tracking

// REMOVED: use alloc::vec::Vec; // Only keep if FrameBitmap truly needs it. Assume not for now.
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{
    FrameAllocator as X64FrameAllocator, PhysFrame, Size4KiB,
    // REMOVED: PageTable - Not directly used by PMM's core logic
};
use crate::boot::info::{MemoryRegion, MemoryRegionType};


// MODIFIED: Standardized to 4KB page size
pub const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub struct FrameBitmap {
    bitmap: [u64; 131072],
    total_managed_frames: usize,
    free_frames: AtomicUsize,
}

impl FrameBitmap {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; 131072], // Ensure this matches the array size
            total_managed_frames: 0,
            free_frames: AtomicUsize::new(0),
        }
    }

    // Helper to set/get bit
    fn set_bit(&mut self, frame_idx: usize, used: bool) {
        let idx = frame_idx / 64;
        let bit = frame_idx % 64;
        if idx < self.bitmap.len() {
            if used {
                self.bitmap[idx] |= 1 << bit;
            } else {
                self.bitmap[idx] &= !(1 << bit);
            }
        } else {
            log::warn!("FrameBitmap: set_bit out of bounds: frame_idx {}", frame_idx);
        }
    }

    fn get_bit(&self, frame_idx: usize) -> bool {
        let idx = frame_idx / 64;
        let bit = frame_idx % 64;
        if idx < self.bitmap.len() {
            (self.bitmap[idx] & (1 << bit)) != 0
        } else {
            log::warn!("FrameBitmap: get_bit out of bounds: frame_idx {}", frame_idx);
            true // Out of bounds considered used
        }
    }

    pub fn init_frame_allocator<'a>(
        &mut self,
        memory_regions_iter: impl Iterator<Item = & 'a MemoryRegion>, // Takes &MemoryRegion
        kernel_start: PhysAddr,
        kernel_end: PhysAddr,
    ) {
        self.bitmap.fill(!0);
        self.total_managed_frames = 0;
        let mut calculated_free_frames = 0;

        for region in memory_regions_iter {
            if region.region_type == MemoryRegionType::Usable {
                let start_addr = region.start_address().as_u64();
                let end_addr = region.end_address().as_u64();
                let first_frame_idx = (start_addr + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64;
                let last_frame_idx = end_addr / PAGE_SIZE as u64;

                if last_frame_idx <= first_frame_idx { continue; }

                for frame_idx_u64 in first_frame_idx..last_frame_idx {
                    let frame_idx = frame_idx_u64 as usize;
                    if frame_idx / 64 < self.bitmap.len() {
                        self.set_bit(frame_idx, false);
                        calculated_free_frames += 1;
                        if frame_idx >= self.total_managed_frames {
                            self.total_managed_frames = frame_idx + 1;
                        }
                    } else { log::warn!("FB: Frame {} out of bounds during init.", frame_idx); }
                }
            }
        }

        let kernel_start_frame_idx = (kernel_start.as_u64() / PAGE_SIZE as u64) as usize;
        let kernel_end_frame_idx = ((kernel_end.as_u64() + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        for frame_idx in kernel_start_frame_idx..kernel_end_frame_idx {
            if frame_idx / 64 < self.bitmap.len() {
                if !self.get_bit(frame_idx) {
                    if calculated_free_frames > 0 { calculated_free_frames -= 1; }
                }
                self.set_bit(frame_idx, true);
            } else { log::warn!("FB: Kernel frame {} out of bounds.", frame_idx); }
        }
        self.free_frames.store(calculated_free_frames, Ordering::SeqCst);
        log::info!("FB Init: Managed Frames: {}, Free After Kernel: {}", self.total_managed_frames, calculated_free_frames);
    }

    pub fn set_frame(&mut self, frame_idx: usize, used: bool) {
        if frame_idx >= self.total_managed_frames && frame_idx / 64 >= self.bitmap.len() {
            log::warn!("Attempt to set_frame for out-of-bounds index: {}", frame_idx);
            return;
        }
        let was_used = self.get_bit(frame_idx);
        self.set_bit(frame_idx, used);

        if used && !was_used {
            self.free_frames.fetch_sub(1, Ordering::Relaxed);
        } else if !used && was_used {
            self.free_frames.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn is_frame_used(&self, frame_idx: usize) -> bool {
        if frame_idx >= self.total_managed_frames { true } // Consider out of managed range as used
        else { self.get_bit(frame_idx) }
    }

    pub fn allocate_frame(&mut self) -> Option<usize> {
        if self.free_frames.load(Ordering::Acquire) == 0 { return None; }
        // Iterate up to total_managed_frames, not bitmap.len()
        for frame_idx in 0..self.total_managed_frames {
            if !self.get_bit(frame_idx) { // If frame is free
                self.set_frame(frame_idx, true); // Marks used and updates count
                return Some(frame_idx);
            }
        }
        None // Should not happen if free_frames > 0, indicates inconsistency
    }

    pub fn free_frame(&mut self, frame_idx: usize) {
        if frame_idx < self.total_managed_frames {
            self.set_frame(frame_idx, false);
        } else {
            log::warn!("Attempt to free out-of-bounds frame_idx: {}", frame_idx);
        }
    }
    // ... allocate_frames, free_frames, allocate_contiguous (ensure they use total_managed_frames) ...
    pub fn allocate_frames(&mut self, count: usize) -> Option<usize> {
        if count == 0 || self.free_frames.load(Ordering::Acquire) < count { return None; }
        if count == 1 { return self.allocate_frame(); }
        // ... (rest of the logic, ensure iteration is up to self.total_managed_frames) ...
        None
    }
     pub fn free_frames(&mut self, start_frame_idx: usize, count: usize) {
        for i in 0..count {
            self.free_frame(start_frame_idx + i);
        }
    }
}

pub struct PhysicalMemoryManager {
    frame_bitmap: Mutex<FrameBitmap>,
    total_physical_memory: AtomicUsize, // Total physical memory reported by bootloader/firmware
    kernel_size: AtomicUsize,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
}

impl PhysicalMemoryManager {
    pub const fn new() -> Self {
        Self {
            frame_bitmap: Mutex::new(FrameBitmap::new()),
            total_physical_memory: AtomicUsize::new(0),
            kernel_size: AtomicUsize::new(0),
            kernel_start: PhysAddr::new(0),
            kernel_end: PhysAddr::new(0),
        }
    }

    pub fn allocate_phys_addr(&self) -> Option<PhysAddr> {
        self.frame_bitmap.lock().allocate_frame()
            .map(|frame_idx| PhysAddr::new(frame_idx as u64 * PAGE_SIZE as u64))
    }

    pub fn free_phys_addr(&self, addr: PhysAddr) {
        let frame_idx = addr.as_u64() / PAGE_SIZE as u64;
        self.frame_bitmap.lock().free_frame(frame_idx as usize);
    }

    pub fn free_phys_addrs(&self, addr: PhysAddr, count: usize) {
        let start_frame_idx = addr.as_u64() / PAGE_SIZE as u64;
        self.frame_bitmap.lock().free_frames(start_frame_idx as usize, count);
    }
    
    pub fn total_memory(&self) -> usize { self.total_physical_memory.load(Ordering::SeqCst) }
    pub fn free_memory(&self) -> usize { self.frame_bitmap.lock().free_frames.load(Ordering::SeqCst) * PAGE_SIZE }
    pub fn used_memory(&self) -> usize { self.total_memory().saturating_sub(self.free_memory()) }
    pub fn kernel_size(&self) -> usize { self.kernel_size.load(Ordering::SeqCst) }
}

unsafe impl X64FrameAllocator<Size4KiB> for PhysicalMemoryManager {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate_phys_addr()
            .map(PhysFrame::containing_address)
    }
}

lazy_static! {
    static ref PHYSICAL_MEMORY_MANAGER: PhysicalMemoryManager = PhysicalMemoryManager::new();
}

pub fn init_frame_allocator<'a>(
    memory_regions_iter: impl Iterator<Item = &'a MemoryRegion> + Clone,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
) -> Result<(), &'static str> {
    log::info!("PMM: init_frame_allocator. Kernel: {:?} - {:?}", kernel_start, kernel_end);
    let pmm = get_physical_memory_manager();

    pmm.kernel_start = kernel_start;
    pmm.kernel_end = kernel_end;
    let kernel_size_bytes = kernel_end.as_u64().saturating_sub(kernel_start.as_u64());
    pmm.kernel_size.store(kernel_size_bytes as usize, Ordering::SeqCst);

    // Calculate total physical memory from the sum of all region sizes in the input iterator
    // This requires iterating once for total, then again for bitmap init.
    // Or, if FrameBitmap's total_managed_frames is accurate for *all* memory, use that.
    // For now, let's calculate total physical memory from the provided regions.
    // This needs the iterator to be Clone, or collect it first.
    let mut total_mem_from_map: u64 = 0;
    for region in memory_regions_iter.clone() {
        total_mem_from_map += region.size();
    }
    pmm.total_physical_memory.store(total_mem_from_map as usize, Ordering::SeqCst);


    let mut bitmap_guard = pmm.frame_bitmap.lock();
    bitmap_guard.init_frame_allocator(memory_regions_iter, kernel_start, kernel_end); // Pass the original iterator
    drop(bitmap_guard);

    log::info!(
        "PMM initialized. Total RAM (from map): {} MiB, Free Now: {} MiB, Kernel Size: {} KiB",
        pmm.total_memory() / (1024 * 1024),
        pmm.free_memory() / (1024 * 1024),
        kernel_size_bytes / 1024
    );
    Ok(())
}

pub fn get_physical_memory_manager() -> &'static mut PhysicalMemoryManager { /* ... as before ... */
    unsafe { static mut PMM_PTR: *mut PhysicalMemoryManager = core::ptr::null_mut(); if PMM_PTR.is_null() { PMM_PTR = &*PHYSICAL_MEMORY_MANAGER as *const _ as *mut _; } &mut *PMM_PTR }
}

pub fn allocate_contiguous_dma(size: usize, alignment: usize, limit_phys_addr_opt: Option<u64>) -> Option<PhysAddr> {
    // ... (implementation as before, ensure it uses the corrected FrameBitmap logic and PAGE_SIZE) ...
    let pmm = get_physical_memory_manager();
    let mut bitmap_guard = pmm.frame_bitmap.lock();
    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Uses corrected PAGE_SIZE

    let max_frame_idx_opt = limit_phys_addr_opt.map(|limit_addr| (limit_addr / PAGE_SIZE as u64) as usize);

    let mut found_start_frame: Option<usize> = None;
    // Ensure iteration limit is correct based on total_managed_frames
    'search: for start_f in 0..(bitmap_guard.total_managed_frames.saturating_sub(num_pages)) {
        if let Some(max_f_idx) = max_frame_idx_opt {
            if (start_f + num_pages -1) > max_f_idx {
                continue 'search;
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

// REMOVED: phys_to_virt, virt_to_phys, create_page_tables, active_level_4_table
// These are responsibilities of memory_manager.rs
