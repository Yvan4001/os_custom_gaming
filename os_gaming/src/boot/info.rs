// src/boot/info.rs

use core::ops::Range;
use x86_64::{PhysAddr, VirtAddr};
use alloc::vec::Vec;
use spin::Once;
use alloc::string::String;

// Data structures for boot information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryRegionType {
    Usable = 1, Reserved = 2, AcpiReclaimable = 3, AcpiNvs = 4, BadMemory = 5,
    Kernel = 6, Bootloader = 7, Framebuffer = 8, Mmio = 9, Unknown = 0xFFFFFFFF,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct MemoryRegion {
    pub range: Range<PhysAddr>,
    pub region_type: MemoryRegionType,
}
impl MemoryRegion {
    pub fn start_address(&self) -> PhysAddr { self.range.start }
    pub fn end_address(&self) -> PhysAddr { self.range.end }
    pub fn size(&self) -> u64 { self.range.end.as_u64().saturating_sub(self.range.start.as_u64()) }
}

#[derive(Debug, Default)]
pub struct CustomBootInfo {
    pub memory_map_regions: Vec<MemoryRegion>,
    pub physical_memory_offset: Option<u64>,
    pub framebuffer_addr: Option<u64>,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_pitch: u32,
    pub framebuffer_bpp: u8,
    pub rsdp_addr: Option<u64>,
    pub command_line: Option<String>, // Using an owned String
}

#[derive(Debug)]
#[repr(C)]
pub struct EarlyBootInfo {
    pub magic: u32,
    pub mbi_addr: u32,
    pub memory_map_size: u32,
    pub memory_map_addr: u32,
    pub physical_memory_offset: u64,
    pub framebuffer_addr: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_pitch: u32,
    pub framebuffer_bpp: u8,
    pub rsdp_addr: u64,
    pub command_line_ptr: u64,
    pub region_count: usize,
    pub memory_regions: [MemoryRegion; 128], // Fixed size for simplicity
    pub end_tag_type: u32,
    pub end_tag_flags: u32,
}

// --- Global Storage for Parsed Boot Info ---
// This uses spin::Once to ensure it's written exactly once.
static GLOBAL_BOOT_INFO: Once<CustomBootInfo> = Once::new();

/// Called by `_start` in `main.rs` to store the parsed info.
/// Panics if called more than once.
pub fn set_global_boot_info(info: CustomBootInfo) {
    GLOBAL_BOOT_INFO.call_once(|| info);
}

/// Called by `kernel_entry_from_lib` and other kernel parts.
/// Returns `None` if `set_global_boot_info` has not been called.
pub fn get_global_boot_info() -> Option<&'static CustomBootInfo> {
    GLOBAL_BOOT_INFO.get()
}
