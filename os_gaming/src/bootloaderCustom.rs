extern crate alloc;
use alloc::vec::Vec;
use alloc::vec; // This is redundant if alloc::vec::Vec is already imported
use alloc::string::String;
// It's good practice to use VirtAddr for offsets that define virtual memory regions
use x86_64::VirtAddr; // Assuming you might want to use this for the offset eventually
use core::ops::Range;

// BootInfo is what the actual bootloader passes to the kernel.
// This custom config is for your kernel's desired settings or internal representation.
// use bootloader::BootInfo; // Not directly used in this file's struct definition

/// Represents desired configurations for the boot environment or early kernel.
/// Note: For bootloader v0.9.31 used with `bootimage`, these settings are not
/// automatically passed to or configured in the actual bootloader by defining this struct.
/// Actual configuration for bootloader v0.9.31 happens via Cargo.toml [package.metadata.bootimage]
/// and an optional JSON config file.
#[derive(Debug, Clone)]
pub struct BootloaderConfig {
    /// The desired virtual address where physical memory should be mapped.
    /// The actual bootloader (v0.9.x or v0.11.x with 'map_physical_memory' feature)
    /// will provide this in BootInfo.physical_memory_offset.
    pub physical_memory_offset: Option<u64>, // Stays as u64 to match BootInfo field

    /// Desired size for the kernel's initial stack.
    pub kernel_stack_size: usize,

    /// Whether the bootloader should identity map the first 1MiB (or similar) of physical memory.
    /// This is a feature some bootloaders offer.
    pub identity_map_base_memory: bool, // Renamed for clarity

    /// Physical memory regions the kernel wishes were excluded by the bootloader from the "usable" map.
    /// For bootloader v0.9.31, this would need to be translated into its JSON config.
    /// For bootloader v0.11.x, this can be configured via .cargo/config.toml or build.rs.
    pub excluded_memory_regions: Vec<Range<u64>>,

    // --- New "Features" / Configuration Options ---

    /// Preferred framebuffer width, if the kernel has a preference.
    pub preferred_framebuffer_width: Option<u32>,

    /// Preferred framebuffer height.
    pub preferred_framebuffer_height: Option<u32>,

    /// Preferred framebuffer bits per pixel (e.g., 32).
    pub preferred_framebuffer_bpp: Option<u8>,

    /// A command line string the kernel might want to parse.
    /// Actual kernel command line is usually passed by the bootloader in BootInfo.
    pub kernel_command_line: Option<String>,

    /// Whether the kernel requires ACPI tables to be found and made available.
    pub acpi_required: bool,

    /// Whether the kernel intends to utilize Symmetric Multi-Processing (SMP)
    /// and thus expects the bootloader to initialize other cores if capable.
    pub smp_enabled_preference: bool,

    /// Desired logging level for the kernel itself (can be set later too).
    pub kernel_log_level: Option<String>, // e.g., "INFO", "DEBUG", "TRACE"
}

/// Provides a default set of desired boot configurations.
/// These are the kernel's *preferences* or *assumptions*. They don't directly
/// configure the bootloader v0.9.31 when called from kernel_main.
pub fn get_bootloader_config() -> BootloaderConfig {
    BootloaderConfig {
        // Existing:
        physical_memory_offset: Some(0xFFFF_8000_0000_0000), // Standard higher-half offset
        kernel_stack_size: 400 * 1024, // 400 KiB
        identity_map_base_memory: true, // Often useful for accessing BIOS areas or early VGA
        excluded_memory_regions: vec![
            0x400000..0x401000, // Your specific exclusion for the problematic PF
            0x0..0x1000, // Typically, first page is often reserved/unusable
            0xA0000..0xC0000, // VGA memory area
            0xE0000..0xF0000, // BIOS area
            0xF0000..0x100000, // Extended BIOS area
            0x100000..0x200000, // Early kernel area

        ],

        // New "Features":
        preferred_framebuffer_width: Some(1920),
        preferred_framebuffer_height: Some(1080),
        preferred_framebuffer_bpp: Some(32),
        kernel_command_line: Some(String::from("fluxgrid.debug=1 fluxgrid.novesa=true")), // Example cmdline
        acpi_required: true,
        smp_enabled_preference: true, // Kernel desires SMP if available
        kernel_log_level: Some(String::from("INFO")),
    }
}

impl BootloaderConfig {
    pub fn new() -> BootloaderConfig {
        get_bootloader_config()
    }
}
