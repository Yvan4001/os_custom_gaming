//! Physical memory management
//! 
//! Handles physical memory allocation and tracking

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{PhysAddr, VirtAddr};
use bit_field::BitArray;

#[cfg(not(feature = "std"))]
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

/// Size of a page (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Physical memory frame bitmap
struct FrameBitmap {
    /// Bitmap where each bit represents a physical frame (1 = used, 0 = free)
    bitmap: [u64; 8192], // Supports up to 32GB with 4KB pages
    /// Total number of physical frames
    total_frames: usize,
    /// Number of free frames
    free_frames: AtomicUsize,
}

impl FrameBitmap {
    /// Create a new empty frame bitmap
    pub const fn new() -> Self {
        Self {
            bitmap: [0; 8192],
            total_frames: 0,
            free_frames: AtomicUsize::new(0),
        }
    }
    
    /// Initialize the bitmap from a memory map
    #[cfg(not(feature = "std"))]
    pub fn init(&mut self, memory_map: &'static MemoryMap) {
        // Calculate total memory size
        let mut max_frame = 0;
        
        // Mark all frames as used initially
        for i in 0..self.bitmap.len() {
            self.bitmap[i] = !0; // All bits set to 1 (used)
        }
        
        // Process memory regions
        for region in memory_map.iter() {
            let start_frame = region.range.start_addr() / PAGE_SIZE as u64;
            let end_frame = region.range.end_addr() / PAGE_SIZE as u64;
            
            if end_frame > max_frame {
                max_frame = end_frame;
            }
            
            // If region is usable, mark frames as free
            if region.region_type == MemoryRegionType::Usable {
                for frame in start_frame..end_frame {
                    self.set_frame(frame as usize, false);
                }
            }
        }
        
        self.total_frames = max_frame as usize;
        self.free_frames = AtomicUsize::new(self.count_free());
    }
    
    /// Set a frame as used or free
    pub fn set_frame(&mut self, frame: usize, used: bool) {
        let idx = frame / 64;
        let bit = frame % 64;
        
        if idx >= self.bitmap.len() {
            return;
        }
        
        if used {
            // Set bit to 1 (used)
            self.bitmap[idx] |= 1 << bit;
            self.free_frames.fetch_sub(1, Ordering::SeqCst);
        } else {
            // Set bit to 0 (free)
            self.bitmap[idx] &= !(1 << bit);
            self.free_frames.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    /// Check if a frame is used
    pub fn is_frame_used(&self, frame: usize) -> bool {
        let idx = frame / 64;
        let bit = frame % 64;
        
        if idx >= self.bitmap.len() {
            return true; // Out of range frames are considered used
        }
        
        (self.bitmap[idx] & (1 << bit)) != 0
    }
    
    /// Find a free frame
    pub fn allocate_frame(&mut self) -> Option<usize> {
        if self.free_frames.load(Ordering::SeqCst) == 0 {
            return None;
        }
        
        // Search for a free frame
        for i in 0..self.bitmap.len() {
            if self.bitmap[i] != !0 {
                // This block has at least one free frame
                for bit in 0..64 {
                    if (self.bitmap[i] & (1 << bit)) == 0 {
                        let frame = i * 64 + bit;
                        self.set_frame(frame, true);
                        return Some(frame);
                    }
                }
            }
        }
        
        None
    }
    
    /// Find a contiguous range of free frames
    pub fn allocate_frames(&mut self, count: usize) -> Option<usize> {
        if count == 0 || self.free_frames.load(Ordering::SeqCst) < count {
            return None;
        }
        
        // For single frame, use the faster method
        if count == 1 {
            return self.allocate_frame();
        }
        
        // Search for a contiguous range
        let mut start_frame = 0;
        let mut current_run = 0;
        
        for frame in 0..self.total_frames {
            if !self.is_frame_used(frame) {
                if current_run == 0 {
                    start_frame = frame;
                }
                current_run += 1;
                
                if current_run == count {
                    // Found enough contiguous frames
                    for f in start_frame..(start_frame + count) {
                        self.set_frame(f, true);
                    }
                    return Some(start_frame);
                }
            } else {
                // Reset the run on used frame
                current_run = 0;
            }
        }
        
        None
    }
    
    /// Free a previously allocated frame
    pub fn free_frame(&mut self, frame: usize) {
        self.set_frame(frame, false);
    }
    
    /// Free a range of previously allocated frames
    pub fn free_frames(&mut self, start_frame: usize, count: usize) {
        for frame in start_frame..(start_frame + count) {
            self.free_frame(frame);
        }
    }
    
    /// Count number of free frames
    fn count_free(&self) -> usize {
        let mut count = 0;
        
        for i in 0..self.bitmap.len() {
            let word = self.bitmap[i];
            count += (!word).count_ones() as usize;
        }
        
        count
    }
}

/// Physical memory manager
pub struct PhysicalMemoryManager {
    frame_bitmap: Mutex<FrameBitmap>,
    total_memory: AtomicUsize,
    kernel_size: AtomicUsize,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
    #[cfg(not(feature = "std"))]
    memory_map: Option<&'static MemoryMap>,
}

impl PhysicalMemoryManager {
    /// Create a new physical memory manager
    pub const fn new() -> Self {
        Self {
            frame_bitmap: Mutex::new(FrameBitmap::new()),
            total_memory: AtomicUsize::new(0),
            kernel_size: AtomicUsize::new(0),
            kernel_start: PhysAddr::new(0),
            kernel_end: PhysAddr::new(0),
            #[cfg(not(feature = "std"))]
            memory_map: None,
        }
    }
    
    /// Initialize the physical memory manager
    #[cfg(not(feature = "std"))]
    pub fn init(&mut self, memory_map: &'static MemoryMap, kernel_start: PhysAddr, kernel_end: PhysAddr) {
        self.memory_map = Some(memory_map);
        self.kernel_start = kernel_start;
        self.kernel_end = kernel_end;
        
        // Initialize the frame bitmap
        let mut bitmap = self.frame_bitmap.lock();
        bitmap.init(memory_map);
        
        // Calculate total memory
        let total_memory = bitmap.total_frames * PAGE_SIZE;
        self.total_memory.store(total_memory, Ordering::SeqCst);
        
        // Calculate kernel size
        let kernel_size = (kernel_end - kernel_start).as_usize();
        self.kernel_size.store(kernel_size, Ordering::SeqCst);
        
        // Mark kernel pages as used
        let kernel_start_frame = kernel_start.as_u64() as usize / PAGE_SIZE;
        let kernel_end_frame = (kernel_end.as_u64() as usize + PAGE_SIZE - 1) / PAGE_SIZE;
        
        for frame in kernel_start_frame..kernel_end_frame {
            bitmap.set_frame(frame, true);
        }
    }
    
    /// Allocate a physical frame
    pub fn allocate_frame(&self) -> Option<PhysAddr> {
        let mut bitmap = self.frame_bitmap.lock();
        
        bitmap.allocate_frame().map(|frame| {
            PhysAddr::new((frame * PAGE_SIZE) as u64)
        })
    }
    
    /// Allocate contiguous physical frames
    pub fn allocate_frames(&self, count: usize) -> Option<PhysAddr> {
        let mut bitmap = self.frame_bitmap.lock();
        
        bitmap.allocate_frames(count).map(|frame| {
            PhysAddr::new((frame * PAGE_SIZE) as u64)
        })
    }
    
    /// Free a physical frame
    pub fn free_frame(&self, addr: PhysAddr) {
        let frame = addr.as_u64() as usize / PAGE_SIZE;
        let mut bitmap = self.frame_bitmap.lock();
        bitmap.free_frame(frame);
    }
    
    /// Free contiguous physical frames
    pub fn free_frames(&self, addr: PhysAddr, count: usize) {
        let frame = addr.as_u64() as usize / PAGE_SIZE;
        let mut bitmap = self.frame_bitmap.lock();
        bitmap.free_frames(frame, count);
    }
    
    /// Check if a physical address is allocated
    pub fn is_allocated(&self, addr: PhysAddr) -> bool {
        let frame = addr.as_u64() as usize / PAGE_SIZE;
        let bitmap = self.frame_bitmap.lock();
        bitmap.is_frame_used(frame)
    }
    
    /// Get total physical memory size
    pub fn total_memory(&self) -> usize {
        self.total_memory.load(Ordering::SeqCst)
    }
    
    /// Get free physical memory size
    pub fn free_memory(&self) -> usize {
        let bitmap = self.frame_bitmap.lock();
        bitmap.free_frames.load(Ordering::SeqCst) * PAGE_SIZE
    }
    
    /// Get used physical memory size
    pub fn used_memory(&self) -> usize {
        self.total_memory() - self.free_memory()
    }
    
    /// Get reserved memory size (including kernel)
    pub fn reserved_memory(&self) -> usize {
        self.kernel_size.load(Ordering::SeqCst)
    }
    
    /// Get kernel size
    pub fn kernel_size(&self) -> usize {
        self.kernel_size.load(Ordering::SeqCst)
    }
}

lazy_static! {
    static ref PHYSICAL_MEMORY_MANAGER: PhysicalMemoryManager = PhysicalMemoryManager::new();
}

/// Initialize physical memory management
#[cfg(not(feature = "std"))]
pub fn init() -> Result<(), &'static str> {
    // This would typically use information from the bootloader
    use bootloader::bootinfo::BootInfo;
    
    // TODO: Get memory map from bootloader
    // let boot_info: &'static BootInfo = ...;
    // let memory_map = &boot_info.memory_map;
    
    // For now, create a dummy memory map
    // PHYSICAL_MEMORY_MANAGER.init(memory_map, kernel_start, kernel_end);
    
    Ok(())
}

#[cfg(feature = "std")]
pub fn init(multiboot_info_addr: usize) -> Result<(), &'static str> {
    // In std mode, we don't need to do much
    Ok(())
}

/// Get a reference to the physical memory manager
pub fn get_physical_memory_manager() -> &'static PhysicalMemoryManager {
    &PHYSICAL_MEMORY_MANAGER
}

/// Convert a physical address to a virtual address
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    // In a higher-half kernel, this would add the kernel base offset
    // For direct mapping, this is often + KERNEL_VIRTUAL_BASE
    #[cfg(not(feature = "std"))]
    {
        // Use the direct mapping offset (0xFFFF800000000000 is common)
        VirtAddr::new(phys.as_u64() + 0xFFFF800000000000)
    }
    
    #[cfg(feature = "std")]
    {
        // In std mode, this is just simulated
        VirtAddr::new(phys.as_u64())
    }
}

/// Convert a virtual address to a physical address
pub fn virt_to_phys(virt: VirtAddr) -> Option<PhysAddr> {
    #[cfg(not(feature = "std"))]
    {
        // Check if the address is in the direct mapping range
        if virt.as_u64() >= 0xFFFF800000000000 {
            Some(PhysAddr::new(virt.as_u64() - 0xFFFF800000000000))
        } else {
            // Use the page tables to translate
            use x86_64::structures::paging::Translate;
            use x86_64::structures::paging::page::Size4KiB;
            
            let page_table = super::current_page_table();
            
            unsafe {
                let page_table = &mut *page_table;
                page_table.translate_addr(virt)
            }
        }
    }
    
    #[cfg(feature = "std")]
    {
        // In std mode, this is just simulated
        Some(PhysAddr::new(virt.as_u64()))
    }
}