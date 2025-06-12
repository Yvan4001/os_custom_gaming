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
};
use core::ptr::NonNull;
use alloc::string::String;

use crate::kernel::memory::physical::{self, PAGE_SIZE};
use crate::kernel::memory::{allocator, dma, r#virtual};
use crate::boot::info::CustomBootInfo as BootInfo;
use crate::kernel::memory::MemoryRegion;

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
    PhysicalOffsetMissing,
    HeapInitFailed,
    DmaInitFailed,
    FrameAllocationFailed,
    MappingError(MapToError<Size4KiB>),
    KernelAddressMissing,
}

impl From<MapToError<Size4KiB>> for MemoryInitError {
    fn from(error: MapToError<Size4KiB>) -> Self {
        MemoryInitError::MappingError(error)
    }
}

// MODIFIED: Added From<&'static str> for MemoryInitError
impl From<&'static str> for MemoryInitError {
    fn from(s: &'static str) -> Self {
        // Log the original string error for debugging, then return a specific variant.
        // Since create_page_tables is the primary source of &'static str errors here,
        // mapping to PageTableCreationFailed is reasonable.
        log::error!("MemoryInitError from &str: {}. Mapping to PageTableCreationFailed.", s);
        MemoryInitError::PageTableCreationFailed
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
            MemoryInitError::PhysicalOffsetMissing => "MM: Physical memory offset missing",
            MemoryInitError::FrameAllocationFailed => "MM: Frame allocation failed during mapping",
            MemoryInitError::MappingError(_) => "MM: Page mapping error",
            MemoryInitError::KernelAddressMissing => "MM: Kernel physical addresses missing/invalid",
        }
    }
}

// --- Memory Flags & Info ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryProtectionFlags { // This is the primary struct for protection
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
        if self.read || self.write || self.execute {
            flags |= PageTableFlags::PRESENT;
        }
        if self.write { flags |= PageTableFlags::WRITABLE; }
        if !self.execute { flags |= PageTableFlags::NO_EXECUTE; }
        if self.user { flags |= PageTableFlags::USER_ACCESSIBLE; }

        match self.cache {
            CacheType::Uncacheable => flags |= PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH,
            CacheType::WriteCombining => flags |= PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH, // Simplification, PAT needed for true WC
            CacheType::WriteThrough => flags |= PageTableFlags::WRITE_THROUGH,
            CacheType::WriteProtected => flags |= PageTableFlags::WRITE_THROUGH, // Fallback
            CacheType::WriteBack => { /* Default */ }
        }
        flags
    }
}

impl Default for MemoryProtectionFlags {
    fn default() -> Self {
        MemoryProtectionFlags {
            read: true,
            write: true,
            execute: false,
            user: false,
            cache: CacheType::WriteBack,
            memory_type: MemoryType::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    pub total_ram: usize,
    pub free_ram: usize,
    pub used_ram: usize,
    pub reserved_ram: usize,
    pub kernel_size: usize,
    pub page_size: usize,
}


// --- Global State ---
lazy_static! {
    pub static ref MEMORY_MANAGER: Mutex<MemoryManager> = Mutex::new(MemoryManager::new_uninit());
}
pub static CORE_MM_INITIALIZED: AtomicBool = AtomicBool::new(false);
pub static PHYSICAL_MEMORY_OFFSET: AtomicU64 = AtomicU64::new(0);
extern "C" {
    pub static __kernel_physical_start: u8;
    pub static __kernel_physical_end: u8;
}

pub struct MemoryManager {
    pub mapper: Option<OffsetPageTable<'static>>,
}

impl MemoryManager {
    fn new_uninit() -> Self { MemoryManager { mapper: None } }

    pub fn init_core(boot_info: &'static BootInfo) -> Result<(), MemoryInitError> {
        if CORE_MM_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
            log::warn!("MemoryManager::init_core called more than once.");
            return Ok(());
        }
        log::info!("Initializing Core Memory Manager (PMM & Mapper)...");

        let kernel_start_phys = PhysAddr::new(unsafe { &__kernel_physical_start as *const _ as u64 });
        let kernel_end_phys = PhysAddr::new(unsafe { &__kernel_physical_end as *const _ as u64 });

        if kernel_start_phys.is_null() || kernel_end_phys.is_null() || kernel_start_phys >= kernel_end_phys {
            CORE_MM_INITIALIZED.store(false, Ordering::SeqCst);
            return Err(MemoryInitError::KernelAddressMissing);
        }
        log::debug!("Kernel physical bounds: {:#x} - {:#x}", kernel_start_phys, kernel_end_phys);

        physical::init_frame_allocator(boot_info.memory_map_regions.iter(), kernel_start_phys, kernel_end_phys)
            .map_err(|e_str| MemoryInitError::PhysicalMemoryInitFailed(String::from(e_str)))?;
        log::info!("Physical Memory Manager (PMM) initialized.");

        let phys_mem_offset = boot_info.physical_memory_offset
            .ok_or(MemoryInitError::PhysicalOffsetMissing)?;
        let phys_mem_offset_val = VirtAddr::new(phys_mem_offset);
        PHYSICAL_MEMORY_OFFSET.store(phys_mem_offset_val.as_u64(), Ordering::SeqCst);
        log::debug!("Physical memory offset stored: {:#x}", phys_mem_offset_val.as_u64());

        let mut mm_guard = MEMORY_MANAGER.lock();
        // The `?` operator will now work because we implemented From<&'static str> for MemoryInitError
        let page_tables = unsafe { Self::create_page_tables(phys_mem_offset_val) }?;
        mm_guard.mapper = Some(page_tables);
        drop(mm_guard);

        log::info!("Kernel Page Table Mapper initialized. Core MM setup complete.");
        Ok(())
    }

    pub fn init_services() -> Result<(), MemoryInitError> {
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) {
            return Err(MemoryInitError::PhysicalMemoryInitFailed("Core MM not ready".into()));
        }
        log::info!("Initializing Memory Services (Heap, DMA)...");
        allocator::init_heap().map_err(|_e_map| MemoryInitError::HeapInitFailed)?;
        log::info!("Kernel Heap initialized.");
        dma::init().map_err(|_e_str| MemoryInitError::DmaInitFailed)?;
        log::info!("DMA Subsystem initialized.");
        log::info!("All memory services initialized.");
        Ok(())
    }

    pub fn init_heap_only() -> Result<(), MemoryInitError> {
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) {
            return Err(MemoryInitError::PhysicalMemoryInitFailed("Core MM not ready".into()));
        }
        
        log::info!("Initializing Kernel Heap (minimal)...");
        allocator::init_heap().map_err(|_e_map| MemoryInitError::HeapInitFailed)?;
        log::info!("Kernel Heap initialized.");
        
        Ok(())
    }

    fn mapper_mut(&mut self) -> &mut OffsetPageTable<'static> {
        self.mapper.as_mut().expect("MemoryManager: Mapper not initialized.")
    }

    pub fn map_kernel_page_internal(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<MapperFlush<Size4KiB>, MapToError<Size4KiB>> {
    
        // CRITICAL LOGGING POINT 1: Entry to your mapping function
        log::debug!(
            "MM_MAP_INTERNAL_ENTRY: Request to map VP {:#x} -> PF {:#x} with Flags {:?}",
            page.start_address().as_u64(),
            frame.start_address().as_u64(),
            flags
        );
    
        // Specifically watch for the problematic mapping parameters
        if page.start_address().as_u64() == 0x1000 && frame.start_address().as_u64() == 0x400000 {
            log::warn!("MM_MAP_INTERNAL_ENTRY: !!! INTERCEPTED PROBLEMATIC PARAMS: VP 0x1000 -> PF 0x400000 !!!");
            // For extreme debugging, you could add a stack trace or deliberate panic here
            // to see the call path from *your* code immediately.
            // Example (requires a panic handler that can print stack or info):
            // panic!("Deliberate panic: map_kernel_page_internal called with VP 0x1000 -> PF 0x400000");
        }
    
        let pmm = physical::get_physical_memory_manager();
    
        // CRITICAL LOGGING POINT 2: Before calling x86_64::Mapper::translate
        log::trace!(
            "MM_MAP_INTERNAL_TRANSLATE: Checking existing mapping for VP {:#x}",
            page.start_address().as_u64()
        );
        match self.mapper_mut().translate(page.start_address()) {
            TranslateResult::Mapped { frame: mapped_raw_frame, offset: _, flags: existing_flags } => {
                if let MappedFrame::Size4KiB(mapped_frame) = mapped_raw_frame {
                    if mapped_frame == frame && existing_flags == flags {
                        log::trace!("MM_MAP_INTERNAL_TRANSLATE: VP {:#x} already identically mapped to PF {:#x}. Skipping map_to.",
                            page.start_address().as_u64(), frame.start_address().as_u64());
                        return Ok(MapperFlush::new(page)); // Already identically mapped
                    } else {
                        log::error!("MM_MAP_INTERNAL_TRANSLATE: VP {:#x} already mapped but differently. Requested PF {:#x}, Existing PF {:#x}. Flags Req: {:?}, Ex: {:?}",
                            page.start_address().as_u64(), frame.start_address().as_u64(), mapped_frame.start_address().as_u64(), flags, existing_flags);
                        return Err(MapToError::PageAlreadyMapped(mapped_frame)); // Mapped differently
                    }
                } else {
                    log::error!("MM_MAP_INTERNAL_TRANSLATE: VP {:#x} is part of a huge page. Cannot map as 4KiB.", page.start_address().as_u64());
                    return Err(MapToError::ParentEntryHugePage); // Mapped as huge page
                }
            }
            TranslateResult::NotMapped => {
                log::trace!("MM_MAP_INTERNAL_TRANSLATE: VP {:#x} is not mapped. Proceeding.", page.start_address().as_u64());
            }
            TranslateResult::InvalidFrameAddress(addr) => {
                log::error!("MM_MAP_INTERNAL_TRANSLATE: InvalidFA {:?} for page {:?}", addr, page.start_address());
                return Err(MapToError::ParentEntryHugePage); // Or a more specific error
            }
        }
    
        // CRITICAL LOGGING POINT 3: Right before calling x86_64::Mapper::map_to
        // This is where the x86_64 crate will perform the operation that might panic.
        log::debug!(
            "MM_MAP_INTERNAL_MAP_TO: Calling x86_64 mapper.map_to for VP {:#x} -> PF {:#x}",
            page.start_address().as_u64(),
            frame.start_address().as_u64()
        );
        unsafe { self.mapper_mut().map_to(page, frame, flags, pmm) }
    }
    

    pub fn map_physical_memory_internal(
        &mut self,
        physical_address: PhysAddr,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<VirtAddr, MemoryError> {
        if size == 0 { return Err(MemoryError::InvalidRange); }
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) { return Err(MemoryError::InvalidState); }
        
        const MMIO_BASE: u64 = 0xFFFF_E000_0000_0000;
        let start_virt_addr = VirtAddr::new(MMIO_BASE + (physical_address.as_u64() & 0x0000_FFFF_FFFF_FFFF));

        let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        for i in 0..num_pages {
            let page_offset = i as u64 * PAGE_SIZE as u64;
            let current_page_virt = Page::containing_address(start_virt_addr + page_offset);
            let current_frame_phys = PhysFrame::containing_address(physical_address + page_offset);
            match self.map_kernel_page_internal(current_page_virt, current_frame_phys, flags) {
                Ok(flush) => flush.flush(),
                Err(e) => {
                    log::error!("map_physical_memory_internal failed for VA {:?} to PA {:?}: {:?}", current_page_virt, current_frame_phys, e);
                    return Err(match e {
                        MapToError::FrameAllocationFailed => MemoryError::OutOfMemory,
                        _ => MemoryError::InvalidMapping,
                    });
                }
            }
        }
        Ok(start_virt_addr)
    }

    pub fn unmap_region_internal(&mut self, virtual_address: VirtAddr, size: usize) -> Result<(), MemoryError> {
        if size == 0 { return Ok(()); }
        if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) { return Err(MemoryError::InvalidState); }

        let start_page = Page::<Size4KiB>::containing_address(virtual_address);
        let end_page = Page::<Size4KiB>::containing_address(virtual_address + size as u64 - 1u64);
        for page in Page::range_inclusive(start_page, end_page) {
            match self.mapper_mut().unmap(page) {
                Ok((_frame, flush)) => flush.flush(),
                Err(UnmapError::PageNotMapped) => log::trace!("Unmap: Page {:?} not mapped.", page),
                Err(e) => {
                    log::error!("unmap_region_internal: Failed to unmap page {:?}: {:?}", page, e);
                    return Err(MemoryError::InvalidMapping);
                }
            }
        }
        Ok(())
    }
    
    pub unsafe fn create_page_tables(phys_mem_offset: VirtAddr) -> Result<OffsetPageTable<'static>, &'static str> {
        let level_4_table_ptr = Self::get_active_level_4_table_mut_ptr(phys_mem_offset);
        Ok(OffsetPageTable::new(&mut *level_4_table_ptr, phys_mem_offset))
    }

    pub unsafe fn get_active_level_4_table_mut_ptr(phys_mem_offset: VirtAddr) -> *mut PageTable {
        let (level_4_frame, _) = Cr3::read();
        (phys_mem_offset + level_4_frame.start_address().as_u64()).as_mut_ptr()
    }
}

// --- Public Interface Functions ---
pub fn map_page_for_kernel(
    page: Page<Size4KiB>,
    frame: PhysFrame<Size4KiB>,
    flags: PageTableFlags,
) -> Result<MapperFlush<Size4KiB>, MapToError<Size4KiB>> {
    log::debug!("MM_PUB_MAP_PAGE: Request for VP {:#x} -> PF {:#x}", page.start_address().as_u64(), frame.start_address().as_u64());
    MEMORY_MANAGER.lock().map_kernel_page_internal(page, frame, flags)
}
pub fn map_physical_memory(
    physical_address: PhysAddr,
    size: usize,
    flags: PageTableFlags,
) -> Result<VirtAddr, MemoryError> {
    log::debug!("MM_PUB_MAP_PHYS: Request for PA {:#x}, size {:#x}", physical_address.as_u64(), size);
    MEMORY_MANAGER.lock().map_physical_memory_internal(physical_address, size, flags)
}
pub fn unmap_region(virtual_address: VirtAddr, size: usize) -> Result<(), MemoryError> {
    MEMORY_MANAGER.lock().unmap_region_internal(virtual_address, size)
}
pub fn get_physical_memory_offset() -> VirtAddr {
    VirtAddr::new(PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed))
}
pub fn memory_info() -> MemoryInfo {
    if !CORE_MM_INITIALIZED.load(Ordering::SeqCst) {
        return MemoryInfo { total_ram:0, free_ram:0, used_ram:0, reserved_ram:0, kernel_size:0, page_size: PAGE_SIZE};
    }
    let pmm = physical::get_physical_memory_manager();
    MemoryInfo {
        total_ram: pmm.total_memory(),
        free_ram: pmm.free_memory(),
        used_ram: pmm.used_memory(),
        reserved_ram: pmm.kernel_size(),
        kernel_size: pmm.kernel_size(),
        page_size: PAGE_SIZE,
    }
}