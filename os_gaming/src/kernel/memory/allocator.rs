//! Kernel heap allocator
//! 
//! Provides heap memory allocation for the kernel

use core::sync::atomic::{AtomicBool, Ordering};
use core::ptr::NonNull;
use core::alloc::{GlobalAlloc, Layout};
use spin::{Mutex, MutexGuard};
use lazy_static::lazy_static;
use bootloader::BootInfo;


#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use linked_list_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
use x86_64::VirtAddr;
use x86_64::PhysAddr;
use x86_64::structures::paging::{Mapper, Page, PhysFrame};
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::mapper::OffsetPageTable;
use x86_64::structures::paging::PageTable;
use crate::kernel::memory::memory_manager::CacheType;

use crate::kernel::memory::memory_manager::{MemoryProtection, MemoryType};
use crate::kernel::memory::physical::get_physical_memory_manager;
static mut PHYSICAL_MEMORY_OFFSET: Option<u64> = None;

// Default initial heap size
const DEFAULT_HEAP_SIZE: usize = 1024 * 1024 * 4; // 4MB // Example address for the mapper
const PAGE_TABLE_ADDR: usize = 0x2000_0000;


/// Heap allocator for kernel memory
pub struct KernelHeapAllocator {
    initialized: AtomicBool,
    #[cfg(not(feature = "std"))]
    allocator: LockedHeap,
    #[cfg(feature = "std")]
    allocations: Mutex<BTreeMap<usize, (usize, Layout)>>,
    #[cfg(feature = "std")]
    next_addr: Mutex<usize>,
}

pub struct BootFrameAllocator {
    next_free_frame: usize,
}

pub struct MemoryOffsetInfo {
    pub physical_memory_offset: u64,
}

impl BootFrameAllocator {
    pub fn new() -> Self {
        BootFrameAllocator {
            next_free_frame: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        use crate::kernel::memory::physical::get_physical_memory_manager;

        get_physical_memory_manager()
            .allocate_frame()
            .map(PhysFrame::containing_address)
    }
}




impl KernelHeapAllocator {
    /// Create a new kernel heap allocator
    pub const fn new() -> Self {
        #[cfg(not(feature = "std"))]
        {
            Self {
                initialized: AtomicBool::new(false),
                allocator: LockedHeap::empty(),
            }
        }
        
        #[cfg(feature = "std")]
        {
            Self {
                initialized: AtomicBool::new(false),
                allocations: Mutex::new(BTreeMap::new()),
                next_addr: Mutex::new(0x1000_0000), // Start at 16MB
            }
        }
    }
    
    /// Initialize the heap allocator
    pub fn init(&self, size: usize) -> Result<(), &'static str> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        #[cfg(not(feature = "std"))]
        {
            // Obtenir une référence mutable au mapper global
            let mut mapper_guard = MAPPER.lock(); // Renommé pour clarté, c'est le MutexGuard

            // Allocate memory for the heap, en passant le mapper
            let heap_start = match super::r#virtual::allocate( // Changed from virtual_mem
                                                               size,
                                                               MemoryProtection {
                                                                   read: true,
                                                                   write: true,
                                                                   execute: false,
                                                                   user: false,
                                                                   cache_type: CacheType::WriteBack,
                                                               },
                                                               MemoryType::Normal,
                                                               &mut *mapper_guard, // Passer une référence mutable au contenu du MutexGuard
            ) {
                Ok(addr) => addr,
                Err(_) => return Err("Failed to allocate heap memory"),
            };

            // Initialize the allocator
            unsafe {
                self.allocator.lock().init(
                    heap_start.as_mut_ptr(),
                    size
                );
            }
        }
        
        #[cfg(feature = "std")]
        {
            // In std mode, we just use the system allocator
            // Nothing to initialize
        }
        
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }
    
    /// Check if the allocator is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for KernelHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.initialized.load(Ordering::SeqCst) {
            return core::ptr::null_mut();
        }
        
        #[cfg(not(feature = "std"))]
        {
            self.allocator.alloc(layout)
        }
        
        #[cfg(feature = "std")]
        {
            // In std mode, we'll just simulate allocation
            let mut next_addr = self.next_addr.lock();
            
            // Align address
            let align = layout.align();
            let addr = (*next_addr + align - 1) & !(align - 1);
            
            // Store the allocation
            let size = layout.size();
            self.allocations.lock().insert(addr, (size, layout));
            
            // Update next address
            *next_addr = addr + size;
            
            addr as *mut u8
        }
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !self.initialized.load(Ordering::SeqCst) {
            return;
        }
        
        #[cfg(not(feature = "std"))]
        {
            self.allocator.dealloc(ptr, layout);
        }
        
        #[cfg(feature = "std")]
        {
            // In std mode, just remove the allocation record
            let addr = ptr as usize;
            self.allocations.lock().remove(&addr);
        }
    }
}

lazy_static! {
    static ref KERNEL_HEAP: KernelHeapAllocator = KernelHeapAllocator::new();
    static ref FRAME_ALLOCATOR: Mutex<KernelHeapAllocator> = Mutex::new(KernelHeapAllocator::new());
    static ref MEMORY_OFFSET_INFO: spin::Mutex<Option<MemoryOffsetInfo>> = spin::Mutex::new(None);
    pub static ref MAPPER: Mutex<OffsetPageTable<'static>> = {
        // Utiliser l'offset global précédemment stocké
        let phys_mem_offset = get_physical_memory_offset();

        // Récupérer la table de pages active
        let level_4_table = unsafe { active_level_4_table(phys_mem_offset) };

        Mutex::new(unsafe { OffsetPageTable::new(level_4_table, phys_mem_offset) })
    };
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::{registers::control::Cr3, structures::paging::page_table::FrameError};

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}


/// Initialize the kernel heap
pub fn init(size: usize) -> Result<(), &'static str> {
    let heap_size = if size == 0 { DEFAULT_HEAP_SIZE } else { size };
    KERNEL_HEAP.init(heap_size)
}

/// Get the kernel heap allocator
pub fn get_kernel_heap() -> &'static KernelHeapAllocator {
    &KERNEL_HEAP
}

pub fn set_memory_offset_info(physical_memory_offset: u64) {
    unsafe {
        PHYSICAL_MEMORY_OFFSET = Some(physical_memory_offset);
    }
}

// Utilisez cette fonction pour obtenir le décalage lors de l'initialisation du mappeur
pub fn get_physical_memory_offset() -> VirtAddr {
    unsafe {
        VirtAddr::new(PHYSICAL_MEMORY_OFFSET.expect("L'offset de mémoire physique n'a pas été initialisé"))
    }
}


#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_5555_5555_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

/// Initializes the kernel heap. Must only be called once.
pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    let mut mapper = MAPPER.lock();
    let mut frame_allocator = BootFrameAllocator::new();

    for page in page_range {
        // Vérifier si la page est déjà mappée avant de tenter de la mapper
        if let Ok(_) = mapper.translate_page(page) {
            continue;
        }

        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)?
                .flush();
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}


