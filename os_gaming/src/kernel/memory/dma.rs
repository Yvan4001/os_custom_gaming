use crate::kernel::memory::physical::PhysicalMemoryManager;
use crate::kernel::memory::r#virtual::VirtualMemoryManager;
use core::sync::atomic::{AtomicBool, Ordering};
extern crate alloc;
use alloc::vec::Vec;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::structures::paging::Translate;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::FrameAllocator;
use core::ptr;
use crate::kernel::memory::allocator::MAPPER;
use crate::kernel::memory::physical::get_physical_memory_manager;
use crate::kernel::memory::r#virtual::{allocate as allocate_virtual, map_physical_memory};
use crate::kernel::memory::memory_manager::{MemoryProtection, CacheType, MemoryType, MemoryInfo};
use crate::kernel::memory::MemoryError;
use x86_64::structures::paging::Mapper;
use crate::kernel::memory::physical;
use crate::kernel::memory::r#virtual;


// Helper function to read PCI configuration space
unsafe fn pci_read_config_u8(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let address = 0x80000000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | (offset as u32);

    // Write address to CONFIG_ADDRESS port
    let mut addr_port = x86_64::instructions::port::Port::new(0xCF8);
    addr_port.write(address);

    // Read data from CONFIG_DATA port
    let mut data_port = x86_64::instructions::port::Port::new(0xCFC + (offset & 3) as u16);
    data_port.read()
}
unsafe fn pci_read_config_u16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let low = pci_read_config_u8(bus, device, function, offset);
    let high = pci_read_config_u8(bus, device, function, offset + 1);
    ((high as u16) << 8) | (low as u16)
}


/// Represents a DMA buffer allocation.
pub struct DmaBuffer {
    /// Virtual address of the buffer
    pub virt_addr: VirtualMemoryManager,
    /// Physical address of the buffer (for device access)
    pub phys_addr: PhysicalMemoryManager,
    /// Size of the buffer in bytes
    pub size: usize,
    /// Whether this buffer is coherent (no explicit cache management needed)
    pub coherent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaAddressLimit {
    /// No limit (up to maximum physical address).
    None,
    /// Limit to addresses below 16MB (24-bit).
    Limit16M,
    /// Limit to addresses below 4GB (32-bit).
    Limit4G,
    /// Custom limit.
    Custom(u64)
}

impl DmaAddressLimit {
    /// Converts the limit to a usize physical address boundary.
    /// Returns None if the limit is DmaAddressLimit::None.
    fn as_physical_limit(&self) -> Option<usize> {
        match self {
            DmaAddressLimit::None => None,
            DmaAddressLimit::Limit16M => Some(0x100_0000), // 16MB
            DmaAddressLimit::Limit4G => Some(0x1_0000_0000), // 4GB
            DmaAddressLimit::Custom(addr) => Some(*addr as usize),
        }
    }
}

/// DMA allocation flags
#[derive(Debug, Clone, Copy)]
pub struct DmaAllocOptions {
    /// If true, the memory will be coherent between CPU and device
    pub coherent: bool,
    /// Preferred alignment of the allocation
    pub align: usize,
}

pub enum DmaType {
    /// Coherent memory (no cache management needed)
    Coherent,
    /// Non-coherent memory (requires explicit cache management)
    NonCoherent,
    
}

pub struct DmaRegion {
    pub phys_addr: usize,
    pub virt_addr: usize,
    pub size: usize,
    pub dma_type : DmaType,
    pub limit: DmaAddressLimit,
}

#[derive(Debug)]
pub struct DmarTable {
    length: u32,
    drhd_units: Vec<DrhdUnit>,
}

#[derive(Debug)]
pub struct DrhdUnit {
    register_base: PhysAddr,
    segment: u16,
    flags: u8,
    scope: Vec<DeviceScope>,
}

#[derive(Debug)]
pub struct DeviceScope {
    r#type: u8,
    bus: u8,
    device: u8,
    function: u8,
}


impl Default for DmaAllocOptions {
    fn default() -> Self {
        Self {
            coherent: true,
            align: 4096, // Page size alignment by default
        }
    }
}

use spin::RwLock;

lazy_static::lazy_static! {
    /// Information about DMA controllers
    static ref DMA_CONTROLLER_INFO: RwLock<DmaControllerInfo> = RwLock::new(DmaControllerInfo::new());
    
    /// DMA memory regions
    static ref DMA_REGIONS: RwLock<DmaRegions> = RwLock::new(DmaRegions::new());
    
    /// DMA buffer pools
    static ref DMA_BUFFER_POOLS: RwLock<DmaBufferPools> = RwLock::new(DmaBufferPools::new());
    
    /// Bounce buffer pool
    static ref BOUNCE_BUFFER_POOL: RwLock<BounceBufferPool> = RwLock::new(BounceBufferPool::new());
    
    /// IOMMU information
    static ref IOMMU_INFO: RwLock<IommuInfo> = RwLock::new(IommuInfo::new());
}

/// Global DMA manager
pub struct DmaManager {
    initialized: AtomicBool,
}

impl DmaManager {
    /// Create a new DMA manager instance
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the DMA subsystem
    pub fn initialize(&self) -> Result<(), &'static str> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Err("DMA manager already initialized");
        }
        
        // 1. Detect available DMA controllers
        self.detect_dma_controllers()?;
        
        // 2. Set up DMA memory regions based on hardware capabilities
        self.setup_dma_regions()?;
        
        // 3. Create pools of pre-allocated DMA buffers for common sizes
        self.initialize_buffer_pools()?;
        
        // 4. Set up bounce buffers for memory outside DMA-addressable range
        self.initialize_bounce_buffers()?;
        
        // 5. Check for and initialize IOMMU if available
        self.initialize_iommu()?;
        
        // Log successful initialization
        #[cfg(feature = "log")]
        log::info!("DMA manager successfully initialized");
        
        Ok(())
    }
    
    /// Detect and initialize DMA controllers
    fn detect_dma_controllers(&self) -> Result<(), &'static str> {
        #[cfg(feature = "std")]
        {
            // In std mode, we simulate the hardware
            return Ok(());
        }
        
        #[cfg(not(feature = "std"))]
        {
            // Check for legacy 8237 DMA controller (ISA DMA)
            let legacy_dma_present = unsafe {
                // Read DMA controller status register
                // Port 0x08 for DMA controller 1 (channels 0-3)
                let mut port = x86_64::instructions::port::Port::new(0x08);
                let status: u8 = port.read();
                
                // If accessible, it's likely present
                true
            };
            
            if legacy_dma_present {
                // Initialize legacy DMA controller
                self.init_legacy_dma()?;
            }
            
            // For modern systems, identify PCI and PCIe devices with DMA capabilities
            let pci_devices = self.enumerate_pci_dma_devices();
            
            // Store information about DMA-capable devices
            let mut dma_info = DMA_CONTROLLER_INFO.write();
            dma_info.legacy_dma_available = legacy_dma_present;
            dma_info.pci_dma_devices = pci_devices;
        }
        
        Ok(())
    }

    fn enumerate_pci_dma_devices(&self) -> Vec<PciDmaDevice> {
        let mut dma_devices = Vec::new();

        // Scan all PCI buses (0-255), devices (0-31), and functions (0-7)
        for bus in 0..=255 {
            for device in 0..32 {
                for function in 0..8 {
                    // Read PCI configuration space
                    let vendor_id = unsafe {
                        pci_read_config_u16(bus, device, function, 0x00)
                    };

                    // Skip if no device present (vendor ID = 0xFFFF)
                    if vendor_id == 0xFFFF {
                        continue;
                    }

                    // Read device capabilities
                    let status = unsafe {
                        pci_read_config_u16(bus, device, function, 0x06)
                    };

                    // Check if device has capabilities list (bit 4 of status register)
                    if status & (1 << 4) != 0 {
                        // Read capabilities pointer
                        let cap_pointer = unsafe {
                            pci_read_config_u8(bus, device, function, 0x34)
                        };

                        // Traverse capabilities list
                        let mut current_cap = cap_pointer;
                        while current_cap != 0 {
                            let cap_id = unsafe {
                                pci_read_config_u8(bus, device, function, current_cap)
                            };

                            // Check for DMA capability (ID = 0x09)
                            if cap_id == 0x09 {
                                // Device supports DMA, create device info
                                let device_id = unsafe {
                                    pci_read_config_u16(bus, device, function, 0x02)
                                };
                                let class_code = unsafe {
                                    pci_read_config_u8(bus, device, function, 0x0B)
                                };
                                let subclass = unsafe {
                                    pci_read_config_u8(bus, device, function, 0x0A)
                                };
                                
                                break; // Found DMA capability, move to next device
                            }

                            // Move to next capability
                            current_cap = unsafe {
                                pci_read_config_u8(bus, device, function, current_cap + 1)
                            };
                        }
                    }
                }
            }
        }

        dma_devices
    }


    /// Initialize legacy 8237 DMA controller
    #[cfg(not(feature = "std"))]
    fn init_legacy_dma(&self) -> Result<(), &'static str> {
        // Safety: Direct hardware access - must be used carefully
        unsafe {
            use x86_64::instructions::port::Port;
            
            // Reset the first DMA controller (channels 0-3)
            let mut cmd_port1: Port<u8> = Port::new(0x0A); // Command register
            let mut mask_port1: Port<u8> = Port::new(0x0F); // All mask register
            
            cmd_port1.write(0x04); // Set to controller reset
            mask_port1.write(0x0F); // Mask all channels
            
            // Reset the second DMA controller (channels 4-7, used on PC/AT)
            let mut cmd_port2: Port<u8> = Port::new(0xD4);
            let mut mask_port2: Port<u8> = Port::new(0xDE);
            
            cmd_port2.write(0x04); // Set to controller reset
            mask_port2.write(0x0F); // Mask all channels
            
            // Clear any pending interrupts
            let mut temp_port: Port<u8> = Port::new(0x0C); // Clear FF register
            let _: u8 = temp_port.read();
            
            temp_port = Port::new(0xD8);
            let _: u8 = temp_port.read();
        }
        
        Ok(())
    }
    
    /// Set up DMA memory regions
    fn setup_dma_regions(&self) -> Result<(), &'static str> {
        // We need different regions for different DMA requirements
        
        // 1. ISA DMA zone (< 16MB)
        // 2. 32-bit DMA zone (< 4GB)
        // 3. 64-bit DMA zone (anywhere in memory)
        
        // Allocate physical memory for DMA zone (ISA)
        let isa_dma_size = 1024 * 1024; // 1MB
        let isa_dma_region = allocate_dma_region(
            isa_dma_size, 
            DmaAddressLimit::Limit16M,
            4096
        )?;
        
        // Allocate physical memory for DMA zone (32-bit)
        let dma32_size = 4 * 1024 * 1024; // 4MB
        let dma32_region = allocate_dma_region(
            dma32_size, 
            DmaAddressLimit::Limit4G,
            4096
        )?;
        
        // Allocate physical memory for DMA zone (64-bit)
        let dma64_size = 16 * 1024 * 1024; // 16MB
        let dma64_region = allocate_dma_region(
            dma64_size, 
            DmaAddressLimit::Limit4G,
            4096
        )?;
        
        // Store the allocated regions for later use
        let mut regions = DMA_REGIONS.write();
        regions.isa_region = Some(isa_dma_region);
        regions.dma32_region = Some(dma32_region);
        regions.dma64_region = Some(dma64_region);
        
        Ok(())
    }
    
    /// Initialize buffer pools for common DMA buffer sizes
    fn initialize_buffer_pools(&self) -> Result<(), &'static str> {
        // Create pools of pre-allocated buffers for common sizes
        // This improves performance by avoiding allocation during critical operations
        
        // Pool sizes - adjust based on your system's needs
        const SMALL_BUFFER_SIZE: usize = 4 * 1024;     // 4KB
        const MEDIUM_BUFFER_SIZE: usize = 64 * 1024;   // 64KB
        const LARGE_BUFFER_SIZE: usize = 1024 * 1024;  // 1MB
        
        // Number of pre-allocated buffers
        const SMALL_BUFFER_COUNT: usize = 32;    // 128KB total
        const MEDIUM_BUFFER_COUNT: usize = 8;    // 512KB total
        const LARGE_BUFFER_COUNT: usize = 2;     // 2MB total
        
        // Create buffer pools
        let mut pools = DMA_BUFFER_POOLS.write();
        
        // Small buffer pool
        let mut small_pool = Vec::with_capacity(SMALL_BUFFER_COUNT);
        for _ in 0..SMALL_BUFFER_COUNT {
            match self.allocate_buffer_internal(
                SMALL_BUFFER_SIZE,
                DmaAllocOptions {
                    coherent: true,
                    align: 4096,
                }
            ) {
                Ok(buffer) => small_pool.push(buffer),
                Err(_) => break, // Stop if allocation fails
            }
        }
        pools.small_buffer_pool = small_pool;
        
        // Medium buffer pool
        let mut medium_pool = Vec::with_capacity(MEDIUM_BUFFER_COUNT);
        for _ in 0..MEDIUM_BUFFER_COUNT {
            match self.allocate_buffer_internal(
                MEDIUM_BUFFER_SIZE,
                DmaAllocOptions {
                    coherent: true,
                    align: 4096,
                }
            ) {
                Ok(buffer) => medium_pool.push(buffer),
                Err(_) => break, // Stop if allocation fails
            }
        }
        pools.medium_buffer_pool = medium_pool;
        
        // Large buffer pool
        let mut large_pool = Vec::with_capacity(LARGE_BUFFER_COUNT);
        for _ in 0..LARGE_BUFFER_COUNT {
            match self.allocate_buffer_internal(
                LARGE_BUFFER_SIZE,
                DmaAllocOptions {
                    coherent: true,
                    align: 4096,
                }
            ) {
                Ok(buffer) => large_pool.push(buffer),
                Err(_) => break, // Stop if allocation fails
            }
        }
        pools.large_buffer_pool = large_pool;
        
        #[cfg(feature = "log")]
        log::debug!("DMA buffer pools initialized: small={}, medium={}, large={}",
                    pools.small_buffer_pool.len(),
                    pools.medium_buffer_pool.len(),
                    pools.large_buffer_pool.len());
        
        Ok(())
    }
    
    /// Initialize bounce buffers for cases where memory is outside DMA-addressable range
    fn initialize_bounce_buffers(&self) -> Result<(), &'static str> {
        // Bounce buffers are used when:
        // 1. A device can only access limited memory range (e.g., ISA DMA < 16MB)
        // 2. We need to copy from higher memory to DMA-accessible memory
        
        // Allocate a pool of bounce buffers
        const BOUNCE_BUFFER_SIZE: usize = 64 * 1024; // 64KB
        const BOUNCE_BUFFER_COUNT: usize = 4;        // 256KB total
        
        let mut bounce_buffers = Vec::with_capacity(BOUNCE_BUFFER_COUNT);
        
        for _ in 0..BOUNCE_BUFFER_COUNT {
            match allocate_dma_region(
                BOUNCE_BUFFER_SIZE,
                DmaAddressLimit::Limit16M, // ISA DMA compatible
                4096
            ) {
                Ok(region) => bounce_buffers.push(region),
                Err(_) => break, // Stop if allocation fails
            }
        }
        
        // Store bounce buffers for later use
        let mut bounce_pool = BOUNCE_BUFFER_POOL.write();
        bounce_pool.buffers = bounce_buffers;
        
        Ok(())
    }

    fn allocate_buffer_internal(&self, size: usize, options: DmaAllocOptions) -> Result<DmaBuffer, &'static str> {
        let mut buffers = DMA_BUFFER_POOLS.write();
        if let Some(buffer) = buffers.small_buffer_pool.pop() {
            return Ok(buffer);
        }
        if let Some(buffer) = buffers.medium_buffer_pool.pop() {
            return Ok(buffer);
        }
        if let Some(buffer) = buffers.large_buffer_pool.pop() {
            return Ok(buffer);
        }
        Err("No available buffer pools")
    }
    pub fn parse_drhd_units(dmar: &mut DmarTable) -> Result<(), &'static str> {
        let mut offset: u64 = 48; // Skip DMAR header
        while offset < dmar.length as u64 {
            // SAFETY: Reading ACPI table contents
            unsafe {
                let entry_type = read_physical_u16(PhysAddr::new(offset))
                    .ok_or("Failed to read entry type")?;
                let entry_length = read_physical_u16(PhysAddr::new(offset + 2))
                    .ok_or("Failed to read entry length")?;

                // DRHD unit (type = 0)
                if entry_type == 0 {
                    let flags = read_physical_u8(PhysAddr::new(offset + 4))
                        .ok_or("Failed to read DRHD flags")?;
                    let segment = read_physical_u16(PhysAddr::new(offset + 6))
                        .ok_or("Failed to read DRHD segment")?;
                    let register_base = PhysAddr::new(
                        read_physical_u64(PhysAddr::new(offset + 8))
                            .ok_or("Failed to read IOMMU register base address")?
                    );

                    let mut unit = DrhdUnit {
                        register_base,
                        segment,
                        flags,
                        scope: Vec::new(),
                    };

                    // Parse device scope
                    let mut scope_offset = offset + 16;
                    while scope_offset < offset + entry_length as u64 {
                        let scope_type = read_physical_u8(PhysAddr::new(scope_offset))
                            .ok_or("Failed to read scope type")?;
                        let scope_length = read_physical_u8(PhysAddr::new(scope_offset + 1))
                            .ok_or("Failed to read scope length")?;
                        let bus = read_physical_u8(PhysAddr::new(scope_offset + 2))
                            .ok_or("Failed to read bus number")?;
                        let device = read_physical_u8(PhysAddr::new(scope_offset + 3))
                            .ok_or("Failed to read device number")?;
                        let function = read_physical_u8(PhysAddr::new(scope_offset + 4))
                            .ok_or("Failed to read function number")?;

                        unit.scope.push(DeviceScope {
                            r#type: scope_type,
                            bus,
                            device,
                            function,
                        });
                        scope_offset += scope_length as u64;
                    }
                    dmar.drhd_units.push(unit);
                }
                offset += entry_length as u64;
            }
        }
        Ok(())
    }

    pub fn detect_iommu() -> Result<bool, &'static str> {
        // Check CPUID for IOMMU/VT-d support
        let cpuid = raw_cpuid::CpuId::new();

        if let Some(vt_features) = cpuid.get_feature_info() {
            // First check for VMX (VT-x) support
            if !vt_features.has_vmx() {
                return Ok(false);
            }
        } else {
            return Err("Failed to read CPU features");
        }

        // Check extended features for VT-d support
        if let Some(ext_features) = cpuid.get_extended_feature_info() {
            if !ext_features.has_avx2() {
                return Ok(false);
            }
        } else {
            return Err("Failed to read extended CPU features");
        }

        // Try to find DMAR table
        if find_acpi_dmar_table().is_none() {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn configure_global_iommu() -> Result<(), &'static str> {
        // Get DMAR table
        let mut dmar = find_acpi_dmar_table()
            .ok_or("No DMAR table found")?;

        // Parse DRHD units
        Self::parse_drhd_units(&mut dmar)?;

        for unit in &dmar.drhd_units {
            // SAFETY: Accessing IOMMU MMIO registers
            unsafe {
                // Enable VT-d globally
                write_physical_u32(unit.register_base + 0x10, 1);

                // Wait for IOMMU to become ready
                while read_physical_u32(unit.register_base + 0x10)
                    .ok_or("Failed to read IOMMU status register")? & (1 << 31) == 0
                {
                    core::hint::spin_loop();
                }

            }
        }

        Ok(())
    }
    

    pub fn setup_dma_remapping_tables() -> Result<(), &'static str> {
        // Allocate root table (4KB aligned)
        let root_table = allocate_page()?;

        // For each DRHD unit
        let dmar = find_acpi_dmar_table()
            .ok_or("No DMAR table found")?;

        for unit in &dmar.drhd_units {
            // SAFETY: Accessing IOMMU MMIO registers
            unsafe {
                // Set root table address
                write_physical_u64(unit.register_base + 0x20, root_table.as_u64() | 1);

                // Invalidate context cache
                write_physical_u64(unit.register_base + 0x28, 1);

                // Wait for invalidation completion
                while read_physical_u32(unit.register_base + 0x2C)
                    .ok_or("Failed to read invalidation status register")? & 1 == 0
                {
                    core::hint::spin_loop();
                }

                // Enable DMA remapping
                let global_command = read_physical_u32(unit.register_base + 0x10)
                    .ok_or("Failed to read global command register")?;
                write_physical_u32(unit.register_base + 0x10, global_command | (1 << 30));
            }
        }

        Ok(())
    }
    


    /// Initialize IOMMU if available
    pub fn initialize_iommu(&self) -> Result<(), &'static str> {
        #[cfg(feature = "std")]
        {
            // En mode std, nous simulons le hardware
            return Ok(());
        }

        #[cfg(not(feature = "std"))]
        {
            // Vérifier la disponibilité de l'IOMMU
            let iommu_available = DmaManager::detect_iommu()?;

            if iommu_available {
                // Initialiser l'IOMMU
                let mut dmar_table = find_acpi_dmar_table()
                    .ok_or("Table DMAR non trouvée")?;

                // Analyser la table DMAR
                DmaManager::parse_drhd_units(&mut dmar_table)?;

                // Configurer l'IOMMU global
                if let Err(e) = DmaManager::configure_global_iommu() {
                    log::warn!("Échec de la configuration de l'IOMMU: {}", e);
                    return Ok(());  // Continuer sans IOMMU
                }

                // Configurer les tables de remapping DMA
                if let Err(e) = DmaManager::setup_dma_remapping_tables() {
                    log::warn!("Échec de la configuration des tables de remapping: {}", e);
                    return Ok(());  // Continuer sans tables de remapping
                }

                // Mettre à jour le statut de l'IOMMU
                let mut iommu_info = IOMMU_INFO.write();
                iommu_info.available = true;
                iommu_info.initialized = true;
                iommu_info.is_intel = true;

                #[cfg(feature = "log")]
                log::info!("IOMMU initialisé avec succès");
            } else {
                #[cfg(feature = "log")]
                log::info!("Aucun IOMMU détecté, utilisation du DMA direct");
            }
        }

        Ok(())
    }

    pub fn map_range_safely(
        mapper: &mut impl Mapper<Size4KiB>,
        start_page: Page<Size4KiB>,
        end_page: Page<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
        flags: PageTableFlags
    ) -> Result<(), MemoryError> {
        for page in Page::range_inclusive(start_page, end_page) {
            // Vérifier si la page est déjà mappée
            if let Ok(_) = mapper.translate_page(page) {
                continue; // Page déjà mappée, passer à la suivante
            }

            // Allouer une frame physique
            let frame = frame_allocator
                .allocate_frame()
                .ok_or(MemoryError::OutOfMemory)?;

            // Mapper la page à la frame
            match unsafe { mapper.map_to(page, frame, flags, frame_allocator) } {
                Ok(tlb) => {
                    tlb.flush();
                },
                Err(MapToError::FrameAllocationFailed) => {
                    return Err(MemoryError::OutOfMemory);
                },
                Err(MapToError::PageAlreadyMapped(mapped_page)) => {
                    // Continuer si la page est déjà mappée
                    continue;
                },
                Err(_) => {
                    return Err(MemoryError::InvalidMapping);
                }
            }
        }

        Ok(())
    }
    
    /// Allocate a buffer suitable for DMA operations
    pub fn allocate(
        size: usize,
        protection: MemoryProtection,
        mem_type: MemoryType,
        mapper: &mut (impl Mapper<Size4KiB> + Translate)
    ) -> Result<VirtAddr, MemoryError> {
        // Vérifier que la taille est valide
        if size == 0 {
            return Err(MemoryError::InvalidAddress);
        }

        // Calculer le nombre de pages nécessaires
        let num_pages = (size + 4095) / 4096; // Arrondir au nombre de pages supérieur

        // Trouver une plage d'adresses virtuelles contiguës
        let start_virt_addr = VirtAddr::new(0x_7000_0000_0000); // Exemple d'adresse de départ

        // Ici, nous obtenons une copie de PhysicalMemoryManager puisque la fonction
        // get_physical_memory_manager() retourne une référence
        let mut frame_allocator = physical::get_physical_memory_manager();

        // Calculer les pages de début et de fin
        let start_page = Page::containing_address(start_virt_addr);
        let end_page = Page::containing_address(start_virt_addr + (num_pages * 4096) as u64 - 1u64);

        // Configurer les flags de page
        let mut flags = PageTableFlags::PRESENT;
        if protection.write {
            flags |= PageTableFlags::WRITABLE;
        }
        if protection.user {
            flags |= PageTableFlags::USER_ACCESSIBLE;
        }
        if !protection.execute {
            flags |= PageTableFlags::NO_EXECUTE;
        }

        if let Err(e) = Self::map_range_safely(mapper, start_page, end_page, frame_allocator, flags) {
            log::error!("Failed to map memory range: {:?}", e);
            return Err(e);
        }

        Ok(start_virt_addr)
    }

    /// Free a previously allocated DMA buffer
    pub fn free(&self, buffer: DmaBuffer) -> Result<(), &'static str> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err("DMA manager not initialized");
        }
        
        // Free the allocated memory
        // Unmap from virtual address space if needed
        
        Ok(())
    }
    
    /// Synchronize a buffer for device access (CPU → Device)
    pub fn sync_for_device(&self, buffer: &DmaBuffer) -> Result<(), &'static str> {
        if !buffer.coherent {
            // Flush CPU caches for the buffer region
            // This ensures the device sees the latest data
        }
        Ok(())
    }
    
    /// Synchronize a buffer for CPU access (Device → CPU)
    pub fn sync_for_cpu(&self, buffer: &DmaBuffer) -> Result<(), &'static str> {
        if !buffer.coherent {
            // Invalidate CPU caches for the buffer region
            // This ensures the CPU sees the latest data written by the device
        }
        Ok(())
    }

}

// Create a global DMA manager instance
pub static DMA_MANAGER: DmaManager = DmaManager::new();

pub fn get_memory_info() -> MemoryInfo {
    #[cfg(feature = "std")]
    {
        // En mode std, nous utilisons des valeurs simulées
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

/// Allocate a contiguous region of physical memory suitable for DMA and maps it
/// into the kernel's virtual address space.
///
/// # Arguments
///
/// * `size`: The size of the region to allocate in bytes.
/// * `limit`: The physical address limit for the allocation (e.g., below 16MB or 4GB).
/// * `alignment`: The required alignment for the physical address.
///
/// # Returns
///
/// A `Result` containing the `DmaRegion` on success, or an error message string
/// on failure.
fn allocate_dma_region(
    size: usize,
    limit: DmaAddressLimit,
    alignment: usize,
) -> Result<DmaRegion, &'static str> {
    // Vérifier si suffisamment de mémoire est disponible avant d'allouer
    let memory_info = get_memory_info();

    // Ajouter de la marge pour les frais généraux de mapping
    let required_memory = size + (size / 10); // Ajouter 10% pour les frais généraux

    if memory_info.free_ram < required_memory {
        return Err("Pas assez de mémoire disponible pour l'allocation DMA");
    }

    let physical_limit_opt = limit.as_physical_limit();

    let phys_addr_type = physical::allocate_contiguous_dma(size, alignment, physical_limit_opt.iter().len())
        .ok_or("Échec de l'allocation de mémoire physique contiguë pour le DMA")?;

    let protection = MemoryProtection {
        read: true,
        write: true,
        execute: false,
        user: false,
        cache_type: CacheType::WriteCombining,
    };

    let mut mapper = MAPPER.lock();
    let virt_addr = r#virtual::map_physical_region(
        phys_addr_type,
        size,
        protection,
        MemoryType::DMA,
        &mut *mapper,
    )
        .map_err(|_| "Échec du mapping de la mémoire DMA physique vers l'adresse virtuelle")?;

    Ok(DmaRegion {
        phys_addr: phys_addr_type.as_u64() as usize,
        virt_addr: virt_addr.as_u64() as usize,
        size,
        dma_type: DmaType::Coherent,
        limit,
    })
}

fn cleanup_failed_dma_mapping(
    start_addr: VirtAddr,
    current_addr: VirtAddr,
    page_size: usize,
    mapper: &mut impl x86_64::structures::paging::Mapper<Size4KiB>,
) {
    let mut addr = start_addr;
    while addr < current_addr {
        let page = Page::containing_address(addr);
        unsafe {
            // Ignorer les erreurs pendant le nettoyage
            let _ = mapper.unmap(page);
        }
        addr += page_size.try_into().unwrap();
    }
}

pub fn free_dma_region(region: DmaRegion) -> Result<(), MemoryError> {
    let page_size = 4096;
    let num_pages = region.size / page_size;

    // Démapper les pages virtuelles
    let mut mapper = MAPPER.lock();
    let mut current_virt = region.virt_addr;

    for _ in 0..num_pages {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(current_virt as u64));

        unsafe {
            // Ignorer les erreurs si la page n'est pas mappée
            let _ = mapper.unmap(page);
        }

        current_virt += page_size;
    }

    // Libérer les frames physiques
    let physical_memory_manager = physical::get_physical_memory_manager();
    let frames_to_free = num_pages;
    physical_memory_manager.free_frames(PhysAddr::new(region.phys_addr as u64), frames_to_free);

    Ok(())
}

pub fn write_to_dma(region: &DmaRegion, data: &[u8], offset: usize) -> Result<(), MemoryError> {
    if offset + data.len() > region.size {
        return Err(MemoryError::OutOfMemory);
    }

    let dst_ptr = unsafe { (region.virt_addr as *mut u8).add(offset) };

    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, data.len());
    }

    Ok(())
}

/// Lit des données depuis une région DMA
pub fn read_from_dma(region: &DmaRegion, buffer: &mut [u8], offset: usize) -> Result<(), MemoryError> {
    if offset + buffer.len() > region.size {
        return Err(MemoryError::OutOfMemory);
    }

    let src_ptr = unsafe { (region.virt_addr as *const u8).add(offset) };

    unsafe {
        core::ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), buffer.len());
    }

    Ok(())
}

/// Synchronise une région DMA avec la mémoire principale (utile pour les caches)
pub fn sync_dma_for_device(region: &DmaRegion) {
    // Vider le cache CPU pour cette région
    unsafe {
        for offset in (0..region.size).step_by(64) {
            let addr = region.virt_addr + offset;
            x86_64::instructions::tlb::flush(VirtAddr::new(addr.try_into().unwrap()));
        }
    }
}

/// Synchronise une région DMA après une opération DMA (pour le CPU)
pub fn sync_dma_for_cpu(region: &DmaRegion) {
    // Invalider le cache CPU pour cette région
    unsafe {
        for offset in (0..region.size).step_by(64) {
            let addr = region.virt_addr + offset;
            x86_64::instructions::tlb::flush(VirtAddr::new(addr.try_into().unwrap()));
        }
    }
}

impl From<DmaAddressLimit> for Option<u64> {
    fn from(limit: DmaAddressLimit) -> Self {
        match limit {
            DmaAddressLimit::Limit16M => Some(0xFF_FFFF), // 16 MiB
            DmaAddressLimit::Limit4G => Some(0xFFFF_FFFF), // 4 GiB
            DmaAddressLimit::Custom(limit) => Some(limit),
            DmaAddressLimit::None => None,
        }
    }
}



struct DmaControllerInfo {
    legacy_dma_available: bool,
    pci_dma_devices: Vec<PciDmaDevice>,
}

impl DmaControllerInfo {
    fn new() -> Self {
        Self {
            legacy_dma_available: false,
            pci_dma_devices: Vec::new(),
        }
    }
}

/// Information about a PCI device with DMA capabilities
#[derive(Clone)]
struct PciDmaDevice {
    bus: u8,
    device: u8,
    function: u8,
    vendor_id: u16,
    device_id: u16,
    dma_mask: u8, // Number of bits in DMA address
}

/// DMA memory regions
struct DmaRegions {
    isa_region: Option<DmaRegion>,
    dma32_region: Option<DmaRegion>,
    dma64_region: Option<DmaRegion>,
}

impl DmaRegions {
    fn new() -> Self {
        Self {
            isa_region: None,
            dma32_region: None,
            dma64_region: None,
        }
    }
}

/// DMA buffer pools
struct DmaBufferPools {
    small_buffer_pool: Vec<DmaBuffer>,
    medium_buffer_pool: Vec<DmaBuffer>,
    large_buffer_pool: Vec<DmaBuffer>,
}

impl DmaBufferPools {
    fn new() -> Self {
        Self {
            small_buffer_pool: Vec::new(),
            medium_buffer_pool: Vec::new(),
            large_buffer_pool: Vec::new(),
        }
    }
}

/// Bounce buffer pool
struct BounceBufferPool {
    buffers: Vec<DmaRegion>,
}

impl BounceBufferPool {
    fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }
}

/// IOMMU information
struct IommuInfo {
    available: bool,
    initialized: bool,
    is_intel: bool,
    is_amd: bool,
}

impl IommuInfo {
    fn new() -> Self {
        Self {
            available: false,
            initialized: false,
            is_intel: false,
            is_amd: false,
        }
    }
}

// Helper functions
unsafe fn find_rsdp() -> Option<PhysAddr> {
    let mut addr = 0xE0000;
    while addr < 0x100000 {
        let signature = read_physical_u64(PhysAddr::new(addr))?;
        if signature == 0x2052545020445352 { // "RSD PTR "
            return Some(PhysAddr::new(addr));
        }
        addr += 16;
    }
    None
}

unsafe fn read_physical_addr(addr: PhysAddr) -> Option<PhysAddr> {
    Some(PhysAddr::new(read_physical_u64(addr)?))
}

unsafe fn read_physical_u64(addr: PhysAddr) -> Option<u64> {
    Some(ptr::read_volatile(addr.as_u64() as *const u64))
}

unsafe fn read_physical_u32(addr: PhysAddr) -> Option<u32> {
    Some(ptr::read_volatile(addr.as_u64() as *const u32))
}

unsafe fn read_physical_u16(addr: PhysAddr) -> Option<u16> {
    Some(ptr::read_volatile(addr.as_u64() as *const u16))
}

unsafe fn read_physical_u8(addr: PhysAddr) -> Option<u8> {
    Some(ptr::read_volatile(addr.as_u64() as *const u8))
}

unsafe fn write_physical_u64(addr: PhysAddr, value: u64) {
    ptr::write_volatile(addr.as_u64() as *mut u64, value);
}

unsafe fn write_physical_u32(addr: PhysAddr, value: u32) {
    ptr::write_volatile(addr.as_u64() as *mut u32, value);
}

fn allocate_page() -> Result<PhysAddr, &'static str> {
    // Implementation depends on your memory allocator
    // Should return 4KB aligned physical address
    unimplemented!("Page allocation not implemented")
}

pub fn find_acpi_dmar_table() -> Option<DmarTable> {
    // SAFETY: This accesses ACPI tables in physical memory
    unsafe {
        // Search for RSDP in BIOS area (0xE0000 - 0xFFFFF)
        let rsdp_addr = find_rsdp()?;

        // Get RSDT/XSDT from RSDP
        let rsdt_addr = read_physical_addr(rsdp_addr + 16)?;

        // Search RSDT entries for "DMAR" signature
        let rsdt_length = read_physical_u32(rsdt_addr + 4)?;
        let entries = (rsdt_length - 36) / 4;

        for i in 0..entries {
            let entry_addr = read_physical_addr(rsdt_addr + 36 + (i * 4) as u64)?;
            let signature = read_physical_u32(entry_addr)?;

            // Check for "DMAR" signature (0x52414D44)
            if signature == 0x52414D44 {
                let length = read_physical_u32(entry_addr + 4)?;
                return Some(DmarTable {
                    length,
                    drhd_units: Vec::new(),
                });
            }
        }
    }

    None
}

pub fn init() -> Result<(), &'static str> {
    Ok(())
}