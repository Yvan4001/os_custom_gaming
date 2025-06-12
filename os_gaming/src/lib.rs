#![no_std]
#![feature(abi_x86_interrupt)]
#![allow(warnings)]

extern crate alloc;
use alloc::string::ToString;
use log::{info, error, debug, trace, warn};
use core::panic::PanicInfo;
use alloc::string::String;

use x86_64::PhysAddr;
use multiboot2::MemoryAreaType;
use multiboot2::BootInformation;
use multiboot2::BootInformationHeader;
use alloc::vec::Vec;


pub mod boot {
    pub mod info;
    pub mod setup;
}
use crate::boot::info::CustomBootInfo;
use crate::boot::info::MemoryRegion;
use crate::boot::info::MemoryRegionType;

pub mod config;
pub mod kernel;
pub mod gui;
pub mod system;
pub mod logger;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "FluxGridOs Gaming System";

#[derive(Debug, Clone)] pub struct Config { /* ... as defined in your lib.rs ... */
    pub display_mode: DisplayMode, pub performance_profile: PerformanceProfile, pub fullscreen: bool,
    pub height: u32, pub width: u32, pub refresh_rate: u32,
    pub theme: String, pub language: String, pub exit_on_escape: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)] pub enum DisplayMode { Windowed, Borderless, Fullscreen }
#[derive(Debug, Clone, Copy, PartialEq)] pub enum PerformanceProfile { PowerSaver, Balanced, Performance, Custom }
impl Default for Config { /* ... as defined in your lib.rs ... */
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Windowed, performance_profile: PerformanceProfile::Balanced,
            fullscreen: false, height: 1080, width: 1920, refresh_rate: 60,
            theme: "default".to_string(), language: "en".to_string(), exit_on_escape: true,
        }
    }
}


#[cfg(not(feature = "std"))]
pub fn kernel_entry_from_lib(magic: u64, multiboot_info_address: u64) -> ! {
    logger::init().expect("Logger initialization failed");
    info!("Logger initialized.");
    
    // We already checked the magic number and non-null address in `_start`.
    // Now we can safely load and parse the MBI.
    
    // This offset MUST match the one your paging setup will use.
    const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_8000_0000_0000;
    let boot_info_mbi = unsafe {
        // Assume GRUB identity maps low memory where MBI resides.
        let mbi_ptr = multiboot_info_address as *const BootInformationHeader;
        BootInformation::load(mbi_ptr).expect("Failed to load Multiboot2 info")
    };
    info!("Multiboot2 Information structure loaded and parsed.");

    // Populate your custom boot info struct
    let mut custom_boot_info = CustomBootInfo {
        physical_memory_offset: Some(PHYSICAL_MEMORY_OFFSET),
        memory_map_regions: Vec::new(),
        ..CustomBootInfo::default()
    };
    
    if let Some(cmdline_tag) = boot_info_mbi.command_line_tag() {
        if let Ok(cmd) = cmdline_tag.cmdline() {
            // This requires the heap allocator to be initialized first
            // to create an owned String. We'll skip storing it for now.
            log::info!("Kernel command line: {}", cmd);
        }
    }
    
    if let Some(memory_map_tag) = boot_info_mbi.memory_map_tag() {
        for area in memory_map_tag.memory_areas() {
            let region_type = match area.typ() {
                _ => MemoryRegionType::Usable, // 1 means available/usable
                _ => MemoryRegionType::Reserved,
            };
            custom_boot_info.memory_map_regions.push(MemoryRegion {
                range: PhysAddr::new(area.start_address())..PhysAddr::new(area.end_address()),
                region_type,
            });
        }
    }
    info!("Memory map parsed. {} regions found.", custom_boot_info.memory_map_regions.len());

    crate::boot::info::set_global_boot_info(custom_boot_info);
    let boot_info_ref = crate::boot::info::get_global_boot_info().expect("Global boot info not set");

    log::info!("Initializing Kernel Subsystems...");
    match init_kernel_internal(boot_info_ref) {
        Ok(_) => log::info!("Kernel subsystems initialized successfully."),
        Err(e) => { log::error!("Kernel initialization failed: {}", e); hcf(); }
    }

    log::info!("Initializing Drivers...");
    match init_driver_internal() {
        Ok(_) => log::info!("Drivers initialized successfully."),
        Err(e) => { log::error!("Driver initialization failed: {}", e); hcf(); }
    }

    log::info!("Initializing GUI...");
    match init_gui_internal() {
        Ok(_) => log::info!("GUI initialized successfully."),
        Err(e) => { log::error!("GUI initialization failed: {}", e); hcf(); }
    }

    log::info!("FluxGridOS startup complete. Halting CPU.");
    loop { x86_64::instructions::hlt(); }
}

fn init_kernel_internal(boot_info: &'static CustomBootInfo) -> Result<(), &'static str> {
    crate::kernel::memory::init(boot_info)?;
    Ok(())
}

fn init_driver_internal() -> Result<(), &'static str> {
    crate::kernel::drivers::init()?;
    Ok(())
}

fn init_gui_internal() -> Result<(), &'static str> {
    let os_config = Config::default();
    crate::gui::init_gui(os_config);
    Ok(())
}
#[cfg(not(feature = "std"))]
pub fn hcf() -> ! {
    log::error!("SYSTEM HALTED!");
    loop { unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack, preserves_flags)); } }
}