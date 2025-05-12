extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use bootloader::BootInfo;
use core::ops::Range;

// Create a custom configuration structure compatible with bootloader 0.9.31
#[derive(Debug, Clone)]
pub struct BootloaderConfig {
    pub physical_memory_offset: Option<u64>,
    pub kernel_stack_size: usize,
    pub identity_map_base: bool,
    pub excluded_memory_regions: Vec<Range<u64>>,
}

pub fn get_bootloader_config() -> BootloaderConfig {
    BootloaderConfig {
        physical_memory_offset: Some(0xFFFF_8000_0000_0000),
        kernel_stack_size: 100 * 1024, // 100 KiB
        identity_map_base: true,
        excluded_memory_regions: vec![0x400000..0x401000],
    }
}