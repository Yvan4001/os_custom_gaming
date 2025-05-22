use core::ops::Range;
use x86_64::{PhysAddr, VirtAddr};
use alloc::vec::Vec;
use spin::Once;

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
    pub command_line: Option<&'static str>,
}

static GLOBAL_BOOT_INFO: Once<CustomBootInfo> = Once::new();

pub fn set_global_boot_info(info: CustomBootInfo) {
    GLOBAL_BOOT_INFO.call_once(|| info);
}

pub fn get_global_boot_info() -> Option<&'static CustomBootInfo> {
    GLOBAL_BOOT_INFO.get()
}