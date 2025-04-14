//! Kernel heap allocator
//! 
//! Provides heap memory allocation for the kernel

use core::sync::atomic::{AtomicBool, Ordering};
use core::ptr::NonNull;
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use lazy_static::lazy_static;

#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use linked_list_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
use x86_64::VirtAddr;

use crate::kernel::memory::memory_manager::{MemoryProtection, MemoryType};

/// Default initial heap size
const DEFAULT_HEAP_SIZE: usize = 1024 * 1024 * 4; // 4MB

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
            // Allocate memory for the heap
            let heap_start = match super::virtual_mem::allocate(
                size,
                MemoryProtection {
                    read: true,
                    write: true,
                    execute: false,
                    user: false,
                    cache: super::CacheType::WriteBack,
                },
                MemoryType::Normal
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

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator::new();