//! Physical memory management
//! 
//! Handles physical memory allocation and tracking

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Translate, RecursivePageTable};
use bit_field::BitArray;
use x86_64::{
    structures::paging::{OffsetPageTable, PageTable},
};
use crate::kernel::memory::MemoryManager;
use super::memory_init;
use crate::kernel::memory::allocator::get_physical_memory_offset;
use crate::kernel::memory::memory_manager::current_page_table;



#[cfg(not(feature = "std"))]
use bootloader::bootinfo::{BootInfo, MemoryRegion, MemoryRegionType};
use bootloader::bootinfo::MemoryMap;


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
    /// Initialize the frame bitmap using the memory map
    pub fn init(&mut self, memory_map: &'static MemoryMap) {
        // Clear the bitmap first
        self.bitmap.fill(0);
        self.total_frames = 0;

        // Calculate total memory and frames from usable regions
        for region in memory_map.iter() {
            if region.region_type == MemoryRegionType::Usable {
                let start_frame = region.range.start_addr() / PAGE_SIZE as u64;
                let end_frame = region.range.end_addr() / PAGE_SIZE as u64;
                let frames_in_region = (end_frame - start_frame) as usize;
                self.total_frames += frames_in_region;
            }
        }

        // Initialize all frames as used (1)
        for byte in self.bitmap.iter_mut() {
            *byte = !0; // Set all bits to 1
        }

        // Mark usable regions as free (0)
        for region in memory_map.iter() {
            if region.region_type == MemoryRegionType::Usable {
                let start_frame = region.range.start_addr() / PAGE_SIZE as u64;
                let end_frame = region.range.end_addr() / PAGE_SIZE as u64;

                for frame in start_frame..end_frame {
                    self.set_frame(frame as usize, false);
                }
            }
        }

        // Initialize free frames count
        self.free_frames.store(self.total_frames, Ordering::SeqCst);
    }

    /// Initialize frame allocator from memory regions
    pub fn init_with_regions(
        memory_regions: impl Iterator<Item = &'static MemoryRegion>,
        phys_mem_offset: VirtAddr,
    ) -> Result<(), &'static str> {
        let pmm = get_physical_memory_manager();

        // Collect memory regions into a Vec so we can iterate multiple times
        let regions: Vec<_> = memory_regions.collect();

        // Calculate total usable memory
        let mut total_memory = 0;
        for region in regions.iter() {
            if region.region_type == MemoryRegionType::Usable {
                total_memory += region.range.end_addr() - region.range.start_addr();
            }
        }

        pmm.total_memory.store(total_memory as usize, Ordering::SeqCst);

        // Initialize frame bitmap with usable regions
        let mut bitmap = pmm.frame_bitmap.lock();
        for region in regions.iter() {
            if region.region_type == MemoryRegionType::Usable {
                let start_frame = region.range.start_addr() / PAGE_SIZE as u64;
                let end_frame = region.range.end_addr() / PAGE_SIZE as u64;

                for frame in start_frame..end_frame {
                    bitmap.set_frame(frame as usize, false);
                }
            }
        }

        Ok(())
    }

    /// Initialize the frame allocator with memory ranges and kernel boundaries
    pub fn init_frame_allocator(&mut self,
                                memory_ranges: impl Iterator<Item = core::ops::Range<u64>>,
                                kernel_start: PhysAddr,
                                kernel_end: PhysAddr
    ) {
        // Clear the bitmap first
        self.bitmap.fill(0);
        self.total_frames = 0;

        // Mark all frames as used initially
        for word in self.bitmap.iter_mut() {
            *word = !0; // All bits set to 1
        }

        // Mark usable ranges as free
        for range in memory_ranges {
            let start_frame = (range.start / PAGE_SIZE as u64) as usize;
            let end_frame = (range.end / PAGE_SIZE as u64) as usize;

            for frame in start_frame..end_frame {
                self.set_frame(frame, false);
                self.total_frames += 1;
            }
        }

        // Mark kernel frames as used
        let kernel_start_frame = (kernel_start.as_u64() / PAGE_SIZE as u64) as usize;
        let kernel_end_frame = ((kernel_end.as_u64() + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        for frame in kernel_start_frame..kernel_end_frame {
            self.set_frame(frame, true);
        }

        // Initialize free frames count
        let used_frames = self.count_used_frames();
        self.free_frames.store(self.total_frames - used_frames, Ordering::SeqCst);
    }

    /// Count the number of used frames in the bitmap
    fn count_used_frames(&self) -> usize {
        self.bitmap.iter()
            .map(|word| word.count_ones() as usize)
            .sum()
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

    pub fn allocate_contiguous(&mut self, count: usize) -> Option<usize> {
        if count == 0 || self.free_frames.load(Ordering::SeqCst) < count {
            return None;
        }

        // Search for a contiguous range of free frames
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
        let kernel_size = (kernel_end - kernel_start);
        self.kernel_size.store(kernel_size as usize, Ordering::SeqCst);
        
        // Mark kernel pages as used
        let kernel_start_frame = kernel_start.as_u64() as usize / PAGE_SIZE;
        let kernel_end_frame = (kernel_end.as_u64() as usize + PAGE_SIZE - 1) / PAGE_SIZE;
        
        for frame in kernel_start_frame..kernel_end_frame {
            bitmap.set_frame(frame, true);
        }
    }
    
    pub fn as_u64(self) -> u64 {
        self.kernel_start.as_u64()
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

/// Initialize memory management using bootloader information
pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    log::info!("Initializing memory subsystem...");

    // Create memory manager instance
    let mut memory_manager = MemoryManager::new();

    // Get the physical memory offset
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    // Initialize memory regions from bootloader info
    let memory_regions = boot_info.memory_map.iter()
        .filter(|region| region.region_type == MemoryRegionType::Usable);

    // Initialize physical memory management
    FrameBitmap::init_with_regions(memory_regions, phys_mem_offset)?;
    
    Ok(())
}


/// Creates the page tables using the physical memory offset
unsafe fn create_page_tables(phys_mem_offset: VirtAddr) -> Result<OffsetPageTable<'static>, &'static str> {
    let level_4_table = active_level_4_table(phys_mem_offset);
    Ok(OffsetPageTable::new(level_4_table, phys_mem_offset))
}

/// Returns a mutable reference to the active level 4 page table
unsafe fn active_level_4_table(phys_mem_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = phys_mem_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// Initialise l'allocateur de frames avec la carte mémoire du bootloader
pub fn init_frame_allocator(
    memory_map: &'static MemoryMap,
    phys_mem_offset: PhysAddr,
    kernel_start: PhysAddr
) -> Result<(), &'static str> {
    log::info!("Initializing frame allocator with bootloader memory map");

    // Calcul de la fin du noyau (estimation)
    let kernel_end = PhysAddr::new(kernel_start.as_u64() + 8 * 1024 * 1024); // 8MB estimation

    // Convertir les régions mémoire en plages utilisables
    let usable_ranges = memory_map
        .iter()
        .filter(|r| r.region_type == MemoryRegionType::Usable)
        .map(|r| r.range.start_addr()..r.range.end_addr());

    // Initialiser la bitmap des frames avec les plages
    let pmm = get_physical_memory_manager();
    let mut bitmap = pmm.frame_bitmap.lock();
    bitmap.init_frame_allocator(usable_ranges, kernel_start, kernel_end);

    // Calculer la mémoire totale
    let total_memory = memory_map
        .iter()
        .filter(|r| r.region_type == MemoryRegionType::Usable)
        .map(|r| r.range.end_addr() - r.range.start_addr())
        .sum::<u64>() as usize;

    pmm.total_memory.store(total_memory, Ordering::SeqCst);

    // Calculer la taille du noyau
    let kernel_size = kernel_end.as_u64().saturating_sub(kernel_start.as_u64()) as usize;
    pmm.kernel_size.store(kernel_size, Ordering::SeqCst);

    Ok(())
}


/// Get mutable reference to the frame bitmap
fn with_frame_bitmap_mut<F, R>(f: F) -> Result<R, &'static str>
where
    F: FnOnce(&mut FrameBitmap) -> R,
{
    let pmm = get_physical_memory_manager();
    let mut bitmap = pmm.frame_bitmap.lock();
    Ok(f(&mut *bitmap))
}


#[cfg(debug_assertions)]
fn print_memory_map(memory_map: &'static MemoryMap) {
    log::debug!("Memory map:");
    for region in memory_map.iter() {
        log::debug!(
            "  {:#x}-{:#x} {:?}",
            region.range.start_addr(),
            region.range.end_addr(),
            region.region_type
        );
    }
}


#[cfg(feature = "std")]
pub fn init(multiboot_info_addr: usize) -> Result<(), &'static str> {
    // In std mode, we don't need to do much
    Ok(())
}

/// Get a reference to the physical memory manager
pub fn get_physical_memory_manager() -> &'static mut PhysicalMemoryManager {
    // Pour un objet créé avec lazy_static, on doit utiliser un autre pattern
    static mut PMM_PTR: Option<*mut PhysicalMemoryManager> = None;

    unsafe {
        if PMM_PTR.is_none() {
            PMM_PTR = Some(&PHYSICAL_MEMORY_MANAGER as *const _ as *mut _);
        }
        &mut *PMM_PTR.unwrap()
    }
}

/// Convert a physical address to a virtual address
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    #[cfg(not(feature = "std"))]
    {
        // Récupérer l'offset dynamiquement
        let phys_mem_offset = get_physical_memory_offset();
        phys_mem_offset + phys.as_u64()
    }

    #[cfg(feature = "std")]
    {
        // En mode std, c'est juste simulé
        VirtAddr::new(phys.as_u64())
    }
}

/// Allocates a contiguous block of physical memory suitable for DMA.
///
/// # Arguments
///
/// * `size`: The requested size of the memory block in bytes.
/// * `alignment`: The required alignment for the starting physical address.
/// * `limit`: The maximum physical address allowed for the allocation (exclusive).
///
/// # Returns
///
/// An `Option<PhysAddr>` containing the starting physical address of the
/// allocated block if successful, or `None` if allocation fails (e.g., due to
/// insufficient memory or exceeding the limit).
pub fn allocate_contiguous_dma(size: usize, alignment: usize, limit: usize) -> Option<PhysAddr> {
    // Align the requested size up to the nearest page boundary.
    // This ensures we allocate whole pages.
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Align the address limit up to the nearest page boundary.
    let aligned_limit = (limit + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    // Check if the required aligned size exceeds the specified physical address limit.
    // Note: This check might be slightly incorrect if `limit` is meant to be the *end* address limit.
    // If `limit` is the highest allowed address, the check should be `start_addr + aligned_size > limit`.
    // However, without knowing the exact allocation strategy (e.g., searching from low addresses),
    // this check prevents allocating a block that *could* potentially cross the limit.
    if aligned_size > aligned_limit {
        // Allocation is impossible within the given constraints.
        return None;
    }

    // TODO: The `alignment` parameter is currently unused. The allocation should
    // search for a block that meets the specified alignment requirement.

    // Obtain a reference to the global physical memory manager.
    let pmm = get_physical_memory_manager();
    // Acquire a lock on the frame bitmap to ensure exclusive access during allocation.
    let mut bitmap = pmm.frame_bitmap.lock();
    // Calculate the number of contiguous physical memory frames (pages) required.
    let num_pages = aligned_size / PAGE_SIZE;
    // Attempt to find and allocate a contiguous sequence of `num_pages` frames using the bitmap.
    // The `?` operator propagates `None` if `allocate_contiguous` fails.
    let start_frame = bitmap.allocate_contiguous(num_pages)?;

    // Calculate the physical address corresponding to the start of the allocated block.
    // Each frame corresponds to `PAGE_SIZE` bytes.
    let phys_addr = PhysAddr::new((start_frame * PAGE_SIZE) as u64);

    // Return the starting physical address wrapped in `Some` to indicate success.
    Some(phys_addr)
}

/// Convert a virtual address to a physical address
pub fn virt_to_phys(virt: VirtAddr) -> Option<PhysAddr> {
    #[cfg(not(feature = "std"))]
    {
        // Récupérer l'offset dynamiquement
        let phys_mem_offset = get_physical_memory_offset();
        let phys_mem_offset_u64 = phys_mem_offset.as_u64();

        // Vérifier si l'adresse est dans la plage de mappage direct
        // La plage commence à l'offset de mémoire physique
        if virt.as_u64() >= phys_mem_offset_u64 {
            // L'adresse est probablement dans la plage de mappage direct
            Some(PhysAddr::new(virt.as_u64() - phys_mem_offset_u64))
        } else {
            // L'adresse n'est pas dans la plage de mappage direct, utiliser la traduction de table de pages

            // Obtenir la table de pages courante en utilisant l'offset correct
            let page_table = unsafe {
                // Assurez-vous que current_page_table() retourne &'static mut PageTable
                let level_4_table_ptr = current_page_table();
                OffsetPageTable::new(&mut *level_4_table_ptr, phys_mem_offset)
            };

            // Traduire l'adresse virtuelle en adresse physique
            page_table.translate_addr(virt)
        }
    }

    #[cfg(feature = "std")]
    {
        // En mode std, c'est juste simulé
        Some(PhysAddr::new(virt.as_u64()))
    }
}