//! Virtual memory management
//! 
//! Handles page tables, mappings, and address space management

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::{VirtAddr, PhysAddr};
pub(crate) use x86_64::structures::paging::{
    PageTable, PageTableFlags, PhysFrame, Size4KiB, Size2MiB, Size1GiB, 
    Mapper, OffsetPageTable, Page, FrameAllocator, MappedPageTable,
    page_table::FrameError
};
use alloc::vec::Vec;
use super::physical::{self, PAGE_SIZE};
use crate::kernel::memory::memory_manager::MemoryError;
use crate::kernel::memory::memory_manager::{MemoryProtection, MemoryType, CacheType};
use crate::kernel::memory::memory_manager::current_page_table;
use crate::kernel::memory::allocator::KernelHeapAllocator;

const DEFAULT_HEAP_SIZE: usize = 1024 * 1024 * 4; // 4MB

/// Virtual memory manager
pub struct VirtualMemoryManager {
    /// Next available virtual address for allocation
    next_virtual_address: AtomicUsize,
    /// Lower half limit (for user space)
    lower_half_limit: VirtAddr,
    /// Kernel space base address
    kernel_base: VirtAddr,
}

impl VirtualMemoryManager {
    /// Create a new virtual memory manager
    pub const fn new() -> Self {
        Self {
            next_virtual_address: AtomicUsize::new(0x1000_0000), // Start at 16MB
            lower_half_limit: VirtAddr::new(0x0000_7FFF_FFFF_FFFF), // 47-bit user space
            kernel_base: VirtAddr::new(0xFFFF_8000_0000_0000), // Higher half
        }
    }
    
    /// Initialize the virtual memory manager
    pub fn init(&self) -> Result<(), &'static str> {
        #[cfg(not(feature = "std"))]
        {
            // Initialize the kernel page tables
            // This is highly dependent on your bootloader and initial state
        }
        
        Ok(())
    }
    
    /// Allocate virtual memory space
    pub fn allocate_virtual_space(&self, size: usize, alignment: usize) -> Result<VirtAddr, MemoryError> {
        // Round size up to page size
        let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let size = pages * PAGE_SIZE;
        
        // Align the address
        let mut addr = self.next_virtual_address.load(Ordering::SeqCst);
        addr = (addr + alignment - 1) & !(alignment - 1);
        
        // Allocate the space
        let new_addr = addr + size;
        self.next_virtual_address.store(new_addr, Ordering::SeqCst);
        
        Ok(VirtAddr::new(addr as u64))
    }
    
    /// Map physical memory to a specific virtual address
    pub fn map_memory(&self,
                     virt_addr: VirtAddr,
                     phys_addr: PhysAddr,
                     size: usize,
                      mapper: &mut OffsetPageTable,
                     protection: MemoryProtection,
                     mem_type: MemoryType) -> Result<(), MemoryError> {
        #[cfg(feature = "std")]
        {
            // In std mode, we just simulate mapping
            Ok(())
        }

        #[cfg(not(feature = "std"))]
        {
            // Round size up to page size
            let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
            let size = pages * PAGE_SIZE;

            let _mapper = crate::kernel::memory::allocator::MAPPER.lock();

            // Convert protection to PageTableFlags
            let mut flags = PageTableFlags::PRESENT;

            if protection.write {
                flags |= PageTableFlags::WRITABLE;
            }
            if !protection.execute {
                flags |= PageTableFlags::NO_EXECUTE;
            }
            if protection.user {
                flags |= PageTableFlags::USER_ACCESSIBLE;
            }

            // Set cache type
            match protection.cache_type {
                CacheType::Uncacheable => {
                    flags |= PageTableFlags::NO_CACHE;
                }
                CacheType::WriteThrough => {
                    flags |= PageTableFlags::WRITE_THROUGH;
                }
                _ => {} // Default caching
            }

            // Map the pages
            let page_table = current_page_table();

            for i in 0..pages {
                let page_virt = VirtAddr::new(virt_addr.as_u64() + (i * PAGE_SIZE) as u64);
                let page_phys = PhysAddr::new(phys_addr.as_u64() + (i * PAGE_SIZE) as u64);

                let page = Page::<Size4KiB>::containing_address(page_virt);
                let frame = PhysFrame::containing_address(page_phys);

                unsafe {
                    // Create an offset page table using the current page table
                    // Map the page
                    mapper.map_to(
                        page,
                        frame,
                        flags,
                        &mut FrameAllocImpl
                    )
                    .map_err(|_| MemoryError::AllocationFailed)?
                    .flush();
                }
            }

            Ok(())
        }
    }
    
    /// Unmap virtual memory
    pub fn unmap_memory(&self, virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
        #[cfg(feature = "std")]
        {
            // In std mode, we just simulate unmapping
            Ok(())
        }
        
        #[cfg(not(feature = "std"))]
        {
            // Round size up to page size
            let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
            
            // Unmap the pages
            let page_table = current_page_table();
            
            for i in 0..pages {
                let page_virt = VirtAddr::new(virt_addr.as_u64() + (i * PAGE_SIZE) as u64);
                let page = Page::<Size4KiB>::containing_address(page_virt);
                
                unsafe {
                    // Create an offset page table using the current page table
                    let mut mapper = OffsetPageTable::new(
                        page_table,
                        VirtAddr::new(0xFFFF800000000000) // Kernel direct mapping offset
                    );
                    
                    // Unmap the page
                    if let Ok((_frame, flush)) = mapper.unmap(page) {
                        flush.flush();
                    }
                }
            }
            
            Ok(())
        }
    }
    
    /// Check if a virtual address is mapped
    pub fn is_mapped(&self, virt_addr: VirtAddr) -> bool {
        #[cfg(feature = "std")]
        {
            // In std mode, we just simulate
            true
        }
        
        #[cfg(not(feature = "std"))]
        {
            // Check if the address is mapped in the page tables
            if let Some(_) = physical::virt_to_phys(virt_addr) {
                true
            } else {
                false
            }
        }
    }
    
    /// Change protection for a memory region
    pub fn protect(&self, virt_addr: VirtAddr, size: usize, protection: MemoryProtection, mapper: &mut OffsetPageTable) -> Result<(), MemoryError> {
        #[cfg(feature = "std")]
        {
            // In std mode, we just simulate
            Ok(())
        }
        
        #[cfg(not(feature = "std"))]
        {
            // Round size up to page size
            let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
            
            // Convert protection to PageTableFlags
            let mut flags = PageTableFlags::PRESENT;
            
            if protection.write {
                flags |= PageTableFlags::WRITABLE;
            }
            if !protection.execute {
                flags |= PageTableFlags::NO_EXECUTE;
            }
            if protection.user {
                flags |= PageTableFlags::USER_ACCESSIBLE;
            }
            
            // Set cache type
            match protection.cache_type {
                CacheType::Uncacheable => {
                    flags |= PageTableFlags::NO_CACHE;
                }
                CacheType::WriteThrough => {
                    flags |= PageTableFlags::WRITE_THROUGH;
                }
                CacheType::WriteBack => {
                    flags |= PageTableFlags::WRITABLE;
                }
                _ => {} // Default caching
            }
            
            // Update the page table entries
            let page_table = current_page_table();
            
            for i in 0..pages {
                let page_virt = VirtAddr::new(virt_addr.as_u64() + (i * PAGE_SIZE) as u64);
                
                // Get the current mapping
                if let Some(phys_addr) = physical::virt_to_phys(page_virt) {
                    // Unmap and remap with new protection
                    self.unmap_memory(page_virt, PAGE_SIZE)?;
                    self.map_memory(page_virt, phys_addr, PAGE_SIZE, mapper, protection, MemoryType::Normal)?;
                }
            }
            
            Ok(())
        }
    }
    
    /// Allocate and map physical memory
    pub fn allocate_memory(&self, size: usize, protection: MemoryProtection, mem_type: MemoryType, mapper: &mut OffsetPageTable) -> Result<VirtAddr, MemoryError> {
        // Allocate virtual space
        let virt_addr = self.allocate_virtual_space(size, PAGE_SIZE)?;
        
        // Round size up to page size
        let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let size = pages * PAGE_SIZE;
        
        #[cfg(not(feature = "std"))]
        {
            // Allocate physical memory
            let phys_addr = physical::get_physical_memory_manager()
                .allocate_frames(pages)
                .ok_or(MemoryError::OutOfMemory)?;
            
            // Map the memory
            self.map_memory(virt_addr, phys_addr, size, mapper, protection, mem_type)?;
        }
        
        Ok(virt_addr)
    }
    
    /// Free allocated memory
    pub fn free_memory(&self, virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
        // Round size up to page size
        let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let size = pages * PAGE_SIZE;
        
        #[cfg(not(feature = "std"))]
        {
            // Get the physical addresses before unmapping
            let mut phys_addrs = Vec::with_capacity(pages);
            
            for i in 0..pages {
                let page_virt = VirtAddr::new(virt_addr.as_u64() + (i * PAGE_SIZE) as u64);
                
                if let Some(phys_addr) = physical::virt_to_phys(page_virt) {
                    phys_addrs.push(phys_addr);
                }
            }
            
            // Unmap the virtual memory
            self.unmap_memory(virt_addr, size)?;
            
            // Free the physical memory
            for phys_addr in phys_addrs {
                physical::get_physical_memory_manager().free_frame(phys_addr);
            }
        }
        
        #[cfg(feature = "std")]
        {
            // In std mode, we just simulate
        }
        
        Ok(())
    }
}

/// A frame allocator implementation for the page table mapping
#[cfg(not(feature = "std"))]
struct FrameAllocImpl;

#[cfg(not(feature = "std"))]
unsafe impl FrameAllocator<Size4KiB> for FrameAllocImpl {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let pmm = physical::get_physical_memory_manager();
        pmm.allocate_frame().map(PhysFrame::containing_address)
    }
}

lazy_static! {
    static ref VIRTUAL_MEMORY_MANAGER: VirtualMemoryManager = VirtualMemoryManager::new();
    static ref KERNEL_HEAP: KernelHeapAllocator = KernelHeapAllocator::new();
}

/// Initialize virtual memory management
pub fn init(size: usize) -> Result<(), &'static str> {
    log::warn!("Using deprecated allocator::init. Consider using init_heap for the global allocator.");
    let heap_size = if size == 0 { DEFAULT_HEAP_SIZE } else { size };
    KERNEL_HEAP.init(heap_size)
}

/// Get a reference to the virtual memory manager
pub fn get_virtual_memory_manager() -> &'static VirtualMemoryManager {
    &VIRTUAL_MEMORY_MANAGER
}

/// Get the kernel heap allocator (Associated with KERNEL_HEAP struct - likely remove)
pub fn get_kernel_heap() -> &'static KernelHeapAllocator {
    log::warn!("Using deprecated allocator::get_kernel_heap.");
   &KERNEL_HEAP
}

/// Allocate virtual memory
pub fn allocate(size: usize, protection: MemoryProtection, mem_type: MemoryType, mapper: &mut OffsetPageTable) -> Result<VirtAddr, MemoryError> {
    get_virtual_memory_manager().allocate_memory(size, protection, mem_type, mapper)
}

/// Free allocated memory
pub fn free(addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
    get_virtual_memory_manager().free_memory(addr, size)
}

/// Map physical memory to virtual memory
pub fn map(virt_addr: VirtAddr, phys_addr: PhysAddr, size: usize, protection: MemoryProtection, mem_type: MemoryType, mapper: &mut OffsetPageTable) -> Result<(), MemoryError> {
    get_virtual_memory_manager().map_memory(virt_addr, phys_addr, size, mapper , protection, mem_type)
}

/// Unmap virtual memory
pub fn unmap(virt_addr: VirtAddr, size: usize) -> Result<(), MemoryError> {
    get_virtual_memory_manager().unmap_memory(virt_addr, size)
}

pub fn map_physical_memory(virt_addr: VirtAddr, phys_addr: PhysAddr, size: usize, protection: MemoryProtection, mem_type: MemoryType, mapper: &mut OffsetPageTable) -> Result<VirtAddr, MemoryError> {
    get_virtual_memory_manager().map_memory(virt_addr, phys_addr, size, mapper, protection, mem_type)?;
    Ok(virt_addr)
}
/*
let virt_addr = r#virtual::map_physical_region(
        PhysAddr::new(phys_addr as u64), // Convert usize phys_addr to PhysAddr
        size,
        protection,
        MemoryType::DMA, // Mark memory as DMA type
        &mut *mapper,    // Pass the page table mapper
    )
 */
pub fn map_physical_region(phys_addr: PhysAddr, size: usize, protection: MemoryProtection, mem_type: MemoryType, mapper: &mut OffsetPageTable) -> Result<VirtAddr, MemoryError> {
    let virt_addr = get_virtual_memory_manager().allocate_memory(size, protection, mem_type, mapper)?;
    get_virtual_memory_manager().map_memory(virt_addr, phys_addr, size, mapper, protection, mem_type)?;
    Ok(virt_addr)
}

/// Change memory protection
pub fn protect(virt_addr: VirtAddr, size: usize, protection: MemoryProtection, mapper: &mut OffsetPageTable) -> Result<(), MemoryError> {
    get_virtual_memory_manager().protect(virt_addr, size, protection, mapper)
}