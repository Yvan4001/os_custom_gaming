#![no_std]
#![feature(abi_x86_interrupt)]
#![allow(warnings)]

extern crate alloc;
use alloc::string::ToString;
use log::{info, error, debug, trace, warn};
use core::panic::PanicInfo;
use alloc::string::String;

pub mod boot {
    pub mod info;
}
use crate::boot::info::CustomBootInfo;

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
pub fn kernel_entry_from_lib() -> ! {
    logger::init().expect("Logger initialization failed");
    log::info!("FluxGridOS Initializing (Manual Multiboot2)... Logger Active.");
    log::info!("FluxGridOS v{} Starting...", VERSION);

    let boot_info = match crate::boot::info::get_global_boot_info() {
        Some(bi) => bi,
        None => {
            log::error!("FATAL: Global boot info not set by _start!");
            hcf();
        }
    };

    log::debug!("Global CustomBootInfo retrieved. PhysMemOffset: {:?}", boot_info.physical_memory_offset);
    log::debug!("Memory map regions count: {}", boot_info.memory_map_regions.len());
    for (i, region) in boot_info.memory_map_regions.iter().enumerate() {
        log::trace!(
            "MemRegion[{}]: PA {:#x} - {:#x} (size {:#x}) type {:?}",
            i, region.start_address().as_u64(), region.end_address().as_u64(),
            region.size(), region.region_type
        );
         if region.start_address().as_u64() <= 0x400000 && region.end_address().as_u64() > 0x400000 {
            log::debug!("CustomMemMap Region containing 0x400000: {:#x}-{:#x} {:?}",
                region.start_address().as_u64(), region.end_address().as_u64(), region.region_type);
        }
    }
    log::info!("Initializing Kernel Subsystems...");
    match init_kernel_internal(boot_info) {
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