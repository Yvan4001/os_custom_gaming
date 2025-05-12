//! Memory management subsystem
//!
//! This module handles physical and virtual memory management,
//! including page allocation, memory mapping, and heap allocation.
extern crate alloc;
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
use x86_64::structures::paging::mapper::{Translate, TranslateResult, MappedFrame};
use x86_64::structures::paging::page_table::PageTableEntry;
use x86_64::structures::paging::{Size2MiB, Size1GiB};
use core::ptr::NonNull;
use bootloader::BootInfo;
use x86_64::structures::paging::mapper::MapToError;
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
    NoMemory,
}

#[derive(Debug)]
pub enum MemoryInitError {
    PageTableCreationFailed,
    PhysicalMemoryInitFailed,
    VirtualMemoryInitFailed,
    HeapInitFailed,
    DmaInitFailed,
    FrameAllocationFailed,
    MappingError(MapToError<Size4KiB>)
}

impl From<MapToError<Size4KiB>> for MemoryInitError {
    fn from(error: MapToError<Size4KiB>) -> Self {
        MemoryInitError::MappingError(error)
    }
}

unsafe impl FrameAllocator<Size4KiB> for PhysicalMemoryManager {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame_addr = match self.allocate_frames(1) {
            Some(addr) => addr,
            None => {
                log::error!("Failed to allocate frame");
                return None;
            }
        };

        // Créer et retourner un PhysFrame à partir de l'adresse
        Some(PhysFrame::containing_address(frame_addr))
    }
}

impl From<MemoryInitError> for &'static str {
    fn from(error: MemoryInitError) -> &'static str {
        match error {
            MemoryInitError::PageTableCreationFailed => "Failed to create page tables",
            MemoryInitError::PhysicalMemoryInitFailed => "Failed to initialize physical memory",
            MemoryInitError::VirtualMemoryInitFailed => "Failed to initialize virtual memory",
            MemoryInitError::HeapInitFailed => "Failed to initialize heap",
            MemoryInitError::DmaInitFailed => "Failed to initialize DMA",
            MemoryInitError::FrameAllocationFailed => "Failed to allocate frame",
            MemoryInitError::MappingError(_) => "Failed to map page",
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
pub struct MemoryManager {
    mapper: Option<OffsetPageTable<'static>>,
    frame_allocator: PhysicalMemoryManager,
    initialized: AtomicBool,
}

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
        MemoryManager {
            mapper: None,
            frame_allocator: PhysicalMemoryManager::new(),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the memory manager
    pub fn init(boot_info: &'static BootInfo) -> Result<(), &'static str> {
        log::info!("Initialising the Memory Manager...");
        if INITIALIZED.load(Ordering::SeqCst) {
            return Ok(());
        }

        #[cfg(not(feature = "std"))]
        {
            // Get the physical memory offset
            let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

            // Initialize physical memory management first
            unsafe {
                physical::init_frame_allocator(
                    &boot_info.memory_map,
                    PhysAddr::new(boot_info.physical_memory_offset),
                    PhysAddr::new(0) // Use the actual kernel addresses when available
                )?;
            }

            // Create page tables after physical memory initialization
            let page_tables = unsafe { Self::create_page_tables(phys_mem_offset)? };
            let mut instance = Self::new();
            instance.mapper = Some(page_tables);

            // Initialize virtual memory management with the page tables
            r#virtual::init(4096)?;

            // Configure the kernel heap once virtual memory is ready
            allocator::init_heap()
                .map_err(|_| "Failed to initialize kernel heap")?;
            log::info!("Kernel heap initialized.");

            // Initialize DMA memory management last
            dma::init()?;
        }

        INITIALIZED.store(true, Ordering::SeqCst);

        #[cfg(feature = "std")]
        log::info!("Memory management initialized (simulated mode)");

        #[cfg(not(feature = "std"))]
        log::info!("Memory management initialized");

        Ok(())
    }

    fn map_page(&mut self, page: Page, frame: PhysFrame, flags: PageTableFlags) -> Result<(), &'static str> {
        // Check if the page is already mapped
        if self.is_page_mapped(page) {
            // If the page is already mapped to the same frame, it's OK
            if self.get_frame_for_page(page) == Some(frame) {
                return Ok(());
            }
            // Otherwise, it's an error
            return Err("Page already mapped to a different frame");
        }

        // Map the page normally
        unsafe {
            self.mapper
                .as_mut()
                .expect("Map is not initialized")
                .map_to(page, frame, flags, &mut self.frame_allocator)
                .map_err(|_| "Échec de mappage de page")?
                .flush();
        }

        Ok(())
    }

    fn is_page_mapped(&self, page: Page) -> bool {
        unsafe {
            self.mapper
                .as_ref()
                .expect("Map is not initialized")
                .translate_page(page)
                .is_ok() // Converts Result to bool (true if Ok, false if Err)
        }
    }

    fn get_frame_for_page(&self, page: Page) -> Option<PhysFrame> {
        unsafe {
            self.mapper
                .as_ref()
                .expect("Map is not initialized")
                .translate_page(page)
                .ok() // Converts Result<PhysFrame, TranslateError> to Option<PhysFrame>
        }
    }

    fn map_page_to_frame(&mut self, page: Page, frame: PhysFrame, flags: PageTableFlags) -> Result<(), &'static str> {
        unsafe {
            self.mapper
                .as_mut()
                .expect("Mapper not initialized")
                .map_to(page, frame, flags, &mut self.frame_allocator)
                .map_err(|_| "Failed to map the page")?
                .flush();
        }
        Ok(())
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
            return Ok(());
        }

        let addr = VirtAddr::new(ptr as u64);
        free(addr, size)
    }

    /// Maps a physical address to a virtual address
    pub fn map_physical(
        &mut self,
        physical_address: PhysAddr,
        size: usize,
        flags: PageTableFlags,
        // Corriger la contrainte Translate (pas de générique)
        mapper: &mut (impl Mapper<Size4KiB> + Translate),
        frame_allocator: &mut impl FrameAllocator<Size4KiB>
    ) -> Result<VirtAddr, MemoryError> {
        if size == 0 {
            return Err(MemoryError::InvalidRange);
        }

        let phys_addr_value = physical_address.as_u64();
        let start_virt_addr = VirtAddr::new(0x_FFFF_C000_0000_0000 + phys_addr_value);

        let page_range = {
            let start_page = Page::<Size4KiB>::containing_address(start_virt_addr);
            let end_virt_addr = start_virt_addr + u64::try_from(size).map_err(|_| MemoryError::InvalidRange)? - 1u64;
            let end_page = Page::<Size4KiB>::containing_address(end_virt_addr);
            Page::range_inclusive(start_page, end_page)
        };

        for page in page_range {
            let frame_offset = page.start_address().as_u64() - start_virt_addr.as_u64();
            let phys_addr = PhysAddr::new(phys_addr_value + frame_offset);
            let frame = PhysFrame::<Size4KiB>::containing_address(phys_addr);

            unsafe {
                // Vérifier si la page est déjà mappée en utilisant translate
                // Match directement sur TranslateResult (pas Ok/Err)
                match mapper.translate(page.start_address()) {
                    TranslateResult::Mapped { frame: mapped_frame, offset: _, flags: existing_flags } => {
                        // Vérifier si c'est une trame de 4KiB
                        match mapped_frame {
                            // Utiliser la bonne variante MappedFrame::Size4KiB
                            MappedFrame::Size4KiB(phys_frame) => {
                                // Comparer la trame physique et les drapeaux
                                if phys_frame == frame && existing_flags == flags {
                                    log::trace!("Page {:?} already correctly mapped, skipping.", page);
                                    continue; // Passer à la page suivante
                                } else {
                                    log::warn!(
                                        "Page {:?} already mapped with different parameters (Frame: {:?}, Flags: {:?}). Expected (Frame: {:?}, Flags: {:?})",
                                        page, phys_frame, existing_flags, frame, flags
                                    );
                                    return Err(MemoryError::AlreadyMapped);
                                }
                            }
                            // Gérer les cas de pages énormes comme des erreurs ici
                            MappedFrame::Size2MiB(_) | MappedFrame::Size1GiB(_) => {
                                log::error!("Page {:?} is part of a huge page, cannot map individually.", page);
                                return Err(MemoryError::InvalidMapping);
                            }
                        }
                    }
                    TranslateResult::NotMapped => {
                        // La page n'est pas mappée, procéder au mappage (géré ci-dessous par map_to)
                        log::trace!("Page {:?} not mapped, proceeding to map.", page);
                    }
                    TranslateResult::InvalidFrameAddress(addr) => {
                        log::error!("Invalid frame address {:?} encountered during translation for page {:?}", addr, page);
                        return Err(MemoryError::InvalidMapping);
                    }
                    // Pas de Err(TranslateError) ici, translate ne retourne pas Result
                }

                // Si la page n'était pas mappée (TranslateResult::NotMapped), on procède au mappage.
                match mapper.map_to(page, frame, flags, frame_allocator) {
                    Ok(flush) => {
                        flush.flush();
                        log::trace!("Mapped page {:?} to frame {:?}", page, frame);
                    }
                    Err(MapToError::FrameAllocationFailed) => {
                        log::error!("Frame allocation failed for mapping page {:?}", page);
                        return Err(MemoryError::OutOfMemory);
                    }
                    Err(MapToError::ParentEntryHugePage) => {
                        log::error!("Cannot map page {:?} because a parent entry is a huge page.", page);
                        return Err(MemoryError::InvalidMapping);
                    }
                    Err(MapToError::PageAlreadyMapped(_existing_frame)) => {
                        // Revérifier car l'état a pu changer entre translate et map_to
                        match mapper.translate(page.start_address()) {
                            // Match directement sur TranslateResult
                            TranslateResult::Mapped { frame: current_mapped_frame, offset: _, flags: current_flags } => {
                                match current_mapped_frame {
                                    // Utiliser la bonne variante MappedFrame::Size4KiB
                                    MappedFrame::Size4KiB(current_phys_frame) => {
                                        if current_phys_frame == frame && current_flags == flags {
                                            log::trace!("Page {:?} was already mapped correctly (reported by map_to).", page);
                                            continue; // Le mappage correct existe
                                        } else {
                                            log::error!("map_to reported PageAlreadyMapped for page {:?}, but re-translation shows different parameters (Frame: {:?}, Flags: {:?}). Expected (Frame: {:?}, Flags: {:?})", page, current_phys_frame, current_flags, frame, flags);
                                            return Err(MemoryError::AlreadyMapped);
                                        }
                                    }
                                    // Gérer les cas de pages énormes
                                    MappedFrame::Size2MiB(_) | MappedFrame::Size1GiB(_) => {
                                        log::error!("map_to reported PageAlreadyMapped for page {:?}, but re-translation shows it's a huge page.", page);
                                        return Err(MemoryError::InvalidMapping); // Ou AlreadyMapped
                                    }
                                }
                            }
                            TranslateResult::NotMapped => {
                                log::error!("Inconsistent state: map_to reported PageAlreadyMapped for page {:?}, but re-translation shows NotMapped.", page);
                                return Err(MemoryError::InvalidMapping);
                            }
                            TranslateResult::InvalidFrameAddress(addr) => {
                                log::error!("Inconsistent state: map_to reported PageAlreadyMapped for page {:?}, but re-translation shows InvalidFrameAddress {:?}.", page, addr);
                                return Err(MemoryError::InvalidMapping);
                            }
                            // Pas de Err(TranslateError)
                        }
                    }
                    // Gérer d'autres erreurs potentielles de map_to si nécessaire
                    _ => {
                        log::error!("An unexpected error occurred while mapping page {:?}", page);
                        return Err(MemoryError::InvalidMapping);
                    }
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
