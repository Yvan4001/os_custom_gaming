//! DMA (Direct Memory Access) Management

use core::sync::atomic::{AtomicBool, Ordering};
use alloc::vec::Vec;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{PageTableFlags, Size4KiB, PhysFrame, Page}; // Added Page, PhysFrame
// FrameAllocator as X64FrameAllocator, Mapper, Translate // Not directly used here if MM handles it

use crate::kernel::memory::{
    physical::{self, PAGE_SIZE}, // Use physical::PAGE_SIZE
    memory_manager::{self, MemoryProtection, CacheType, MemoryType},
    MemoryError,
};

// ... (PCI config read functions as before) ...
unsafe fn pci_read_config_u8(bus: u8, device: u8, function: u8, offset: u8) -> u8 { /* ... */
    let address = 0x80000000 | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | (offset as u32);
    let mut addr_port = x86_64::instructions::port::Port::new(0xCF8); addr_port.write(address);
    let mut data_port = x86_64::instructions::port::Port::new(0xCFC + (offset & 3) as u16); data_port.read()
}
unsafe fn pci_read_config_u16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let low = pci_read_config_u8(bus, device, function, offset); let high = pci_read_config_u8(bus, device, function, offset + 1);
    ((high as u16) << 8) | (low as u16)
}


#[derive(Debug)] // Added Debug
pub struct DmaBuffer {
    pub virt_addr: VirtAddr,
    pub phys_addr: PhysAddr,
    pub size: usize,
    pub coherent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaAddressLimit { None, Limit16M, Limit4G, Custom(u64) }
impl DmaAddressLimit {
    fn as_physical_limit_option(&self) -> Option<u64> { // Renamed to be clearer
        match self {
            DmaAddressLimit::None => None,
            DmaAddressLimit::Limit16M => Some(0x100_0000 - 1), // Max address in 0-16MB range
            DmaAddressLimit::Limit4G => Some(0x1_0000_0000 - 1),// Max address in 0-4GB range
            DmaAddressLimit::Custom(addr) => Some(*addr),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DmaAllocOptions {
    pub coherent: bool,
    pub align: usize,
    pub limit: DmaAddressLimit,
}
impl Default for DmaAllocOptions { /* ... as before ... */
    fn default() -> Self { Self { coherent: true, align: PAGE_SIZE, limit: DmaAddressLimit::None } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaType { Coherent, NonCoherent }

#[derive(Debug)]
pub struct DmaRegion {
    pub phys_addr: PhysAddr,
    pub virt_addr: VirtAddr,
    pub size: usize,
    pub dma_type: DmaType,
}

// --- IOMMU related structs (definitions only, implementation is complex) ---
#[derive(Debug)] pub struct DmarTable { /* ... */ physical_address: PhysAddr, length: u32, drhd_units: Vec<DrhdUnit>, }
#[derive(Debug)] pub struct DrhdUnit { /* ... */ physical_register_base: PhysAddr, virtual_register_base: Option<VirtAddr>, segment: u16, flags: u8, scope: Vec<DeviceScope>,}
#[derive(Debug)] pub struct DeviceScope { /* ... */ r#type: u8, bus: u8, device: u8, function: u8, }
#[derive(Debug, Clone)] struct PciDmaDevice { /* ... */ bus: u8, device: u8, function: u8, vendor_id: u16, device_id: u16, }


// --- Global DMA State ---
use spin::RwLock; // Ensure spin::RwLock is used
lazy_static::lazy_static! {
    static ref DMA_MANAGER_STATE: RwLock<DmaManagerState> = RwLock::new(DmaManagerState::new());
}

#[derive(Debug)] // Added Debug
struct DmaManagerState {
    initialized: bool,
    // ... other fields from dma_rs_v2 (legacy_dma_available, pci_dma_devices, etc.)
    isa_region: Option<DmaRegion>,
    dmar_table: Option<DmarTable>, // For IOMMU
    // ... buffer pools ...
    small_buffer_pool: Vec<DmaBuffer>,
}
impl DmaManagerState { fn new() -> Self { /* ... initialize fields ... */
    DmaManagerState { initialized: false, isa_region: None, dmar_table: None, small_buffer_pool: Vec::new(), /* etc */ }
} }


/// DMA Manager (Zero-Sized Type for namespacing static methods)
pub struct DmaManager;

impl DmaManager {
    pub fn init_subsystem() -> Result<(), &'static str> { // Renamed from init to avoid conflict with module init
        let mut state = DMA_MANAGER_STATE.write();
        if state.initialized {
            log::warn!("DMA Manager already initialized.");
            return Ok(());
        }
        log::info!("Initializing DMA Subsystem...");
        // Self::detect_dma_controllers(&mut state)?;
        // Self::setup_dma_memory_pools(&mut state)?;
        // Self::initialize_iommu(&mut state)?; // Complex, placeholder
        state.initialized = true;
        log::info!("DMA Subsystem initialized (basic setup).");
        Ok(())
    }

    // Internal helper to allocate and map a DMA region
    fn allocate_dma_region_internal(size: usize, options: DmaAllocOptions) -> Result<DmaRegion, MemoryError> {
        if size == 0 { return Err(MemoryError::InvalidRange); }

        let phys_addr = physical::allocate_contiguous_dma(
            size,
            options.align,
            options.limit.as_physical_limit_option(), // Pass Option<u64>
        ).ok_or(MemoryError::NoMemory)?;

        let mut page_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        if options.coherent {
            page_flags |= PageTableFlags::NO_CACHE; // Or WRITE_THROUGH
        }

        // Use the global mapping service from memory_manager
        let virt_addr = memory_manager::map_physical_memory(phys_addr, size, page_flags)?;

        Ok(DmaRegion {
            phys_addr,
            virt_addr,
            size,
            dma_type: if options.coherent { DmaType::Coherent } else { DmaType::NonCoherent },
        })
    }

    // Internal helper to allocate a DmaBuffer
    fn allocate_dma_buffer_internal(size: usize, options: DmaAllocOptions) -> Result<DmaBuffer, MemoryError> {
        let region = Self::allocate_dma_region_internal(size, options)?;
        Ok(DmaBuffer {
            virt_addr: region.virt_addr,
            phys_addr: region.phys_addr,
            size: region.size,
            coherent: options.coherent,
        })
    }

    // Public API to allocate a DMA buffer
    pub fn allocate_buffer(size: usize, options: DmaAllocOptions) -> Result<DmaBuffer, MemoryError> {
        let state = DMA_MANAGER_STATE.read();
        if !state.initialized { return Err(MemoryError::InvalidState); }
        // TODO: Try to get from pool first
        Self::allocate_dma_buffer_internal(size, options)
    }

    // Public API to free a DMA buffer
    pub fn free_buffer(buffer: DmaBuffer) -> Result<(), MemoryError> {
        let state = DMA_MANAGER_STATE.read();
        if !state.initialized { return Err(MemoryError::InvalidState); }

        memory_manager::unmap_region(buffer.virt_addr, buffer.size)?;
        physical::get_physical_memory_manager().free_phys_addrs(
            buffer.phys_addr,
            (buffer.size + PAGE_SIZE - 1) / PAGE_SIZE,
        );
        Ok(())
    }

    // --- IOMMU Placeholder ---
    // fn initialize_iommu(state: &mut DmaManagerState) -> Result<(), &'static str> {
    //     log::debug!("IOMMU initialization (placeholder)...");
    //     // Find DMAR table, map it, parse DRHDs, map DRHD MMIO, configure IOMMU.
    //     // This is very complex.
    //     Ok(())
    // }
    // ... other DMA methods like sync_for_device/cpu ...
}

/// Public init function for the DMA module, called by `MemoryManager::init_services`.
pub fn init() -> Result<(), &'static str> {
    DmaManager::init_subsystem()
}
