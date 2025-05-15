//! Memory management subsystem core: Mapper and core services.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{
    FrameAllocator as X64FrameAllocator, Mapper, Page, PageTable, PhysFrame, Size4KiB,
    OffsetPageTable, PageTableFlags,
    mapper::{MapperFlush, MapToError, UnmapError, TranslateResult, MappedFrame, Translate},
    // MappedPageTable, page_table::PageTableEntry, Size2MiB, Size1GiB // Keep if using huge pages
};
use core::ptr::NonNull;
use bootloader::BootInfo;
use alloc::string::String; // Only if MemoryInitError::PhysicalMemoryInitFailed uses it.

use crate::kernel::memory::physical::{self, PAGE_SIZE}; // Use physical::PAGE_SIZE
use crate::kernel::memory::{allocator, dma, r#virtual}; // For init_services

// --- Error Types ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    AllocationFailed, InvalidAddress, PermissionDenied, PageFault, OutOfMemory,
    InvalidMapping, AlreadyMapped, NotMapped, InvalidRange, NoMemory, InvalidState, NotImplemented,
}

#[derive(Debug)]
pub enum MemoryInitError {
    PageTableCreationFailed,
    PhysicalMemoryInitFailed(String),
    VirtualMemoryInitFailed,
    HeapInitFailed,
    DmaInitFailed,
    FrameAllocationFailed, // From FrameAllocator trait if PMM fails
    MappingError(MapToError<Size4KiB>),
    KernelAddressMissing,
}

pub struct MemoryInfo {
    pub total_ram: usize,
    pub free_ram: usize,
    pub used_ram: usize,
    pub reserved_ram: usize,
    pub kernel_size: usize,
    pub page_size: usize,
}

impl From<MapToError<Size4KiB>> for MemoryInitError {
    fn from(error: MapToError<Size4KiB>) -> Self {
        MemoryInitError::MappingError(error)
    }
}

impl From<MemoryInitError> for &'static str {
    fn from(error: MemoryInitError) -> &'static str {
        match error {
            MemoryInitError::PageTableCreationFailed => "MM: Page table creation failed",
            MemoryInitError::PhysicalMemoryInitFailed(_) => "MM: Physical memory init failed",
            MemoryInitError::VirtualMemoryInitFailed => "MM: Virtual memory init failed",
            MemoryInitError::HeapInitFailed => "MM: Heap init failed",
            MemoryInitError::DmaInitFailed => "MM: DMA init failed",
            MemoryInitError::FrameAllocationFailed => "MM: Frame allocation failed during mapping",
            MemoryInitError::MappingError(_) => "MM: Page mapping error",
            MemoryInitError::KernelAddressMissing => "MM: Kernel physical addresses missing/invalid",
        }
    }
}

// --- Memory Flags ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProtection {
    NoAccess, ReadOnly, ReadWrite, Execute, ExecuteRead, ExecuteReadWrite,
}

pub struct MemoryProtectionFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub user: bool,
    pub cache: CacheType,
    pub memory_type: MemoryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType { Uncacheable, WriteCombining, WriteThrough, WriteProtected, WriteBack }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType { Normal, Device, DMA, Video, Code, Data, Stack, Heap }

impl MemoryProtectionFlags {
    pub fn new(read: bool, write: bool, execute: bool, user: bool, cache: CacheType, memory_type: MemoryType) -> Self {
        MemoryProtectionFlags { read, write, execute, user, cache, memory_type }
    }

    pub fn to_page_table_flags(&self) -> PageTableFlags {
        let mut flags = PageTableFlags::empty();
        if self.read { flags |= PageTableFlags::PRESENT; }
        if self.write { flags |= PageTableFlags::WRITABLE; }
        if self.execute { flags |= PageTableFlags::NO_EXECUTE; }
        if self.user { flags |= PageTableFlags::USER_ACCESSIBLE; }
        if self.cache == CacheType::Uncacheable { flags |= PageTableFlags::NO_CACHE; }
        flags
    }

    pub fn from_page_table_flags(flags: PageTableFlags) -> Self {
        MemoryProtectionFlags {
            read: flags.contains(PageTableFlags::PRESENT),
            write: flags.contains(PageTableFlags::WRITABLE),
            execute: !flags.contains(PageTableFlags::NO_EXECUTE),
            user: flags.contains(PageTableFlags::USER_ACCESSIBLE),
            cache: if flags.contains(PageTableFlags::NO_CACHE) { CacheType::Uncacheable } else { CacheType::WriteBack },
            memory_type: MemoryType::Normal, // Default, can be set based on context
        }
    }

    pub fn is_executable(&self) -> bool {
        self.execute
    }

    pub fn is_readable(&self) -> bool {
        self.read
    }

    pub fn is_writable(&self) -> bool {
        self.write
    }
}

impl Default for MemoryProtectionFlags {
    fn default() -> Self {
        MemoryProtectionFlags { read: false, write: false, execute: false, user: false, cache: CacheType::Uncacheable, memory_type: MemoryType::Normal }
    }
}


// --- Global State ---
lazy_static! {
    // This is the single global instance of the MemoryManager, holding the mapper.
    static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new_uninit());
}

/// Tracks if the core memory subsystem (PMM and Mapper) has been initialized.
static CORE_MM_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Stores the physical memory offset provided by the bootloader.
/// Initialized by `MemoryManager::init_core`.
static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);

/// External symbols for kernel physical bounds (must be defined in linker script).
extern "C" {
    static __kernel_physical_start: u8;
    static __kernel_physical_end: u8;
}

pub struct MemoryManager {
    mapper: Option<OffsetPageTable<'static>>,
}

impl MemoryManager {
    /// Creates a new, uninitialized MemoryManager.
    /// Should only be called by the `MEMORY_MANAGER` lazy_static.
    fn new_uninit() -> Self {
        MemoryManager { mapper: None }
    }

    /// Initializes the core memory system: PMM and the kernel's page table mapper.
    /// Must be called only once.
    pub fn init_core(boot_info: &'static BootInfo) -> Result<(), MemoryInitError> {
        if CORE_MM_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
            log::warn!("MemoryManager::init_core called more than once.");
            return Ok(());
        }
        log::info!("Initializing Core Memory Manager (PMM & Mapper)...");

        // 1. Get kernel physical addresses from linker symbols
        let kernel_start_phys = PhysAddr::new(unsafe { &__kernel_physical_start as *const _ as u64 });
        let kernel_end_phys = PhysAddr::new(unsafe { &__kernel_physical_end as *const _ as u64 });

        if kernel_start_phys.is_null() || kernel_end_phys.is_null() || kernel_start_phys >= kernel_end_phys {
            CORE_MM_INITIALIZED.store(false, Ordering::SeqCst); // Rollback
            return Err(MemoryInitError::KernelAddressMissing);
        }
        log::debug!("Kernel physical bounds: {:#x} - {:#x}", kernel_start_phys, kernel_end_phys);

        // 2. Initialize the global Physical Memory Manager (Frame Allocator)
        physical::init_frame_allocator(&boot_info.memory_map, kernel_start_phys, kernel_end_phys)
            .map_err(|e_str| MemoryInitError::PhysicalMemoryInitFailed(String::from(e_str)))?;
        log::info!("Physical Memory Manager (PMM) initialized.");

        // 3. Store physical memory offset and create page tables (Mapper)
        let phys_mem_offset_val = VirtAddr::new(boot_info.physical_memory_offset);
        PHYSICAL_MEMORY_OFFSET.store(phys_mem_offset_val.as_u64(), Ordering::SeqCst);
        log::debug!("Physical memory offset stored: {:#x}", phys_mem_offset_val.as_u64());

        let mut mm_guard = MEMORY_MANAGER.lock(); // Lock the global MemoryManager instance
        let page_tables = unsafe {
            // Pass the now stored offset to create_page_tables
            Self::create_page_tables(phys_mem_offset_val)
        }
            .map_err(|e_str| MemoryInitError::PageTableCreationFailed)?;
        mm_guard.mapper = Some(page_tables);
        drop(mm_guard); // Release lock

        log::info!("Kernel Page Table Mapper initialized. Core MM setup complete.");
        Ok(())
    }

    /// Initializes higher-level memory services that depend on the core MM.
    pub fn init_services() -> Result<(), MemoryInitError> {
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) {
            log::error!("Attempted to init services before core MM.");
            return Err(MemoryInitError::PhysicalMemoryInitFailed("Core MM not ready".into()));
        }
        log::info!("Initializing Memory Services (Heap, DMA)...");

        allocator::init_heap().map_err(|e_map| MemoryInitError::HeapInitFailed)?; // Assuming init_heap returns MapToError
        log::info!("Kernel Heap initialized.");

        dma::init().map_err(|e_str| MemoryInitError::DmaInitFailed)?; // Assuming dma::init returns &'static str
        log::info!("DMA Subsystem initialized.");

        log::info!("All memory services initialized.");
        Ok(())
    }

    /// Gets a mutable reference to the internal `OffsetPageTable` mapper.
    /// Panics if the mapper is not initialized (i.e., `init_core` hasn't run).
    fn mapper_mut(&mut self) -> &mut OffsetPageTable<'static> {
        self.mapper.as_mut().expect("MemoryManager: Mapper not initialized. Call init_core first.")
    }

    /// Internal function to map a single kernel page. Robust against re-mapping identical pages.
    pub fn map_kernel_page_internal( // Renamed to avoid conflict with public map_page_for_kernel
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size4KiB>, MapToError<Size4KiB>> {
        // Use the global PMM via physical::get_physical_memory_manager()
        // The FrameAllocator trait is implemented on PhysicalMemoryManager
        let pmm = physical::get_physical_memory_manager();

        match self.mapper_mut().translate(page.start_address()) {
            TranslateResult::Mapped { frame: mapped_raw_frame, offset: _, flags: existing_flags } => {
                if let MappedFrame::Size4KiB(mapped_frame) = mapped_raw_frame {
                    if mapped_frame == frame && existing_flags == flags {
                        return Ok(MapperFlush::new(page)); // Already identically mapped
                    } else {
                        return Err(MapToError::PageAlreadyMapped(mapped_frame)); // Mapped differently
                    }
                } else {
                    return Err(MapToError::ParentEntryHugePage); // Mapped as huge page
                }
            }
            TranslateResult::NotMapped => { /* Page is not mapped, proceed */ }
            TranslateResult::InvalidFrameAddress(addr) => {
                log::error!("map_kernel_page: Invalid frame address {:?} during translate for page {:?}", addr, page);
                return Err(MapToError::ParentEntryHugePage); // Or a more specific error
            }
        }
        // This is safe because we're mapping kernel pages, and PMM handles frame validity.
        unsafe { self.mapper_mut().map_to(page, frame, flags, pmm) }
    }

    /// Maps a physical memory range to a dynamically chosen virtual address range (placeholder for VA mgmt).
    pub fn map_physical_memory_internal( // Renamed to avoid conflict
        &mut self,
        physical_address: PhysAddr,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<VirtAddr, MemoryError> {
        if size == 0 { return Err(MemoryError::InvalidRange); }
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) { return Err(MemoryError::InvalidState); }

        // VERY BASIC VA allocation: This needs a proper Virtual Address Space manager.
        // For now, using a fixed high-memory region for MMIO/physical mappings.
        // This is NOT robust for general use.
        const MMIO_BASE: u64 = 0xFFFF_E000_0000_0000; // Example fixed base
        // A real VA manager would find a free slot.
        // For simplicity, we'll map phys_addr to MMIO_BASE + phys_addr. This is common for device memory.
        // Ensure no overlap if phys_addr is very large.
        let start_virt_addr = VirtAddr::new(MMIO_BASE + (physical_address.as_u64() & 0x0000_FFFF_FFFF_FFFF)); // Mask to keep it in a reasonable range

        let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        for i in 0..num_pages {
            let page_offset = i as u64 * PAGE_SIZE as u64;
            let current_page_virt = Page::containing_address(start_virt_addr + page_offset);
            let current_frame_phys = PhysFrame::containing_address(physical_address + page_offset);

            match self.map_kernel_page_internal(current_page_virt, current_frame_phys, flags) {
                Ok(flush) => flush.flush(),
                Err(e) => {
                    log::error!("map_physical_memory failed for VA {:?} to PA {:?}: {:?}", current_page_virt, current_frame_phys, e);
                    // TODO: Attempt to unmap pages successfully mapped so far in this call.
                    return Err(match e {
                        MapToError::FrameAllocationFailed => MemoryError::OutOfMemory,
                        _ => MemoryError::InvalidMapping,
                    });
                }
            }
        }
        Ok(start_virt_addr)
    }

    /// Unmaps a virtual memory region. Does not free the underlying physical frames.
    pub fn unmap_region_internal(&mut self, virtual_address: VirtAddr, size: usize) -> Result<(), MemoryError> { // Renamed
        if size == 0 { return Ok(()); }
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) { return Err(MemoryError::InvalidState); }

        let start_page = Page::<Size4KiB>::containing_address(virtual_address);
        let end_page = Page::<Size4KiB>::containing_address(virtual_address + size as u64 - 1u64);
        let page_range = Page::range_inclusive(start_page, end_page);

        for page in page_range {
            match self.mapper_mut().unmap(page) {
                Ok((_frame, flush)) => flush.flush(), // Frame is returned but we don't free it here
                Err(UnmapError::PageNotMapped) => log::trace!("unmap_region: Page {:?} was not mapped.", page),
                Err(e) => {
                    log::error!("unmap_region: Failed to unmap page {:?}: {:?}", page, e);
                    return Err(MemoryError::InvalidMapping);
                }
            }
        }
        Ok(())
    }

    /// Helper to create the initial `OffsetPageTable`.
    unsafe fn create_page_tables(phys_mem_offset: VirtAddr) -> Result<OffsetPageTable<'static>, &'static str> {
        // `current_page_table` needs the offset directly, as PMM might not be fully usable yet for virt_to_phys.
        let level_4_table_ptr = Self::get_active_level_4_table_mut_ptr(phys_mem_offset);
        Ok(OffsetPageTable::new(&mut *level_4_table_ptr, phys_mem_offset))
    }

    /// Helper to get a mutable pointer to the active L4 page table.
    unsafe fn get_active_level_4_table_mut_ptr(phys_mem_offset: VirtAddr) -> *mut PageTable {
        let (level_4_frame, _) = Cr3::read();
        let phys_addr = level_4_frame.start_address();
        let virt_addr = phys_mem_offset + phys_addr.as_u64();
        virt_addr.as_mut_ptr()
    }
    // REMOVE: allocate and deallocate methods. Heap allocator should be used.
}

// --- Public Interface Functions ---
// These are the primary functions other modules should use for memory operations.

pub fn map_page_for_kernel(
    page: Page<Size4KiB>,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
) -> Result<MapperFlush<Size4KiB>, MapToError<Size4KiB>> {
    MEMORY_MANAGER.lock().map_kernel_page_internal(page, frame, flags)
}

pub fn map_physical_memory(
    physical_address: PhysAddr,
    size: usize,
    flags: PageTableFlags,
) -> Result<VirtAddr, MemoryError> {
    MEMORY_MANAGER.lock().map_physical_memory_internal(physical_address, size, flags)
}

pub fn unmap_region(virtual_address: VirtAddr, size: usize) -> Result<(), MemoryError> {
    MEMORY_MANAGER.lock().unmap_region_internal(virtual_address, size)
}

/// Provides access to the physical memory offset stored during core initialization.
pub fn get_physical_memory_offset() -> VirtAddr {
    VirtAddr::new(PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed))
}

/// Retrieves current memory statistics.
pub fn memory_info() -> MemoryInfo {
    if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) {
        return MemoryInfo { total_ram:0, free_ram:0, used_ram:0, reserved_ram:0, kernel_size:0, page_size: PAGE_SIZE};
    }
    let pmm = physical::get_physical_memory_manager(); // Safe to call after PMM init
    MemoryInfo {
        total_ram: pmm.total_memory(),
        free_ram: pmm.free_memory(),
        used_ram: pmm.used_memory(),
        reserved_ram: pmm.kernel_size(), // Assuming reserved is mainly kernel
        kernel_size: pmm.kernel_size(),
        page_size: PAGE_SIZE,
    }
}

// REMOVED current_page_table from here. If needed, it should use get_physical_memory_offset().
// However, direct access to L4 table should be minimized outside this module.
