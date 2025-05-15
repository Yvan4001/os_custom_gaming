#![cfg_attr(not(feature = "std"), no_std)] // Garder pour no_std
// #![cfg_attr(not(feature = "std"), no_main)] // Supprimer, entry_point! s'en charge
#![feature(abi_x86_interrupt)]
#![no_std]
#![cfg_attr(test, no_main)]
#![allow(warnings)]

// Imports conditionnels pour std
#[cfg(feature = "std")]
pub use std::{boxed::Box, string::String, vec::Vec};
#[cfg(feature = "std")]
use toml; // Garder si utilisé en std

// Imports pour no_std (alloc requis)
extern crate alloc;
#[cfg(not(feature = "std"))]
pub use alloc::{boxed::Box, string::{String, ToString}, vec::Vec, sync::Arc}; // Combiner les imports alloc

// Imports Core communs
use core::panic::PanicInfo;
use core::arch::asm;
use core::default::Default;
use core::result::Result::{self, Ok, Err}; // Importer Result et ses variantes
use core::clone::Clone;
use core::marker::{Send, Sync, Copy};
use core::cmp::PartialEq;
use core::ops::FnOnce;
use core::option::Option::{self, Some, None}; // Importer Option et ses variantes
use core::mem::drop; // Garder si utilisé explicitement

// Imports des crates externes
use bootloader::{entry_point, BootInfo};
use log::{info, error}; // Garder les imports log

// Déclaration des modules locaux (supprimer la version conditionnelle de gui si non nécessaire)
pub mod config;
pub mod kernel;
pub mod gui;
pub mod system;
pub mod logger;
pub mod bootloaderCustom;

// System constants
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "FluxGridOs Gaming System";

// Configuration structure
#[derive(Debug, Clone)]
pub struct Config {
    pub display_mode: DisplayMode,
    pub performance_profile: PerformanceProfile,
    pub fullscreen: bool,
    pub height: u32,
    pub width: u32,
    pub refresh_rate: u32,
    pub theme: String,
    pub language: String,
    pub exit_on_escape: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Windowed,
    Borderless,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerformanceProfile {
    PowerSaver,
    Balanced,
    Performance,
    Custom,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Windowed,
            performance_profile: PerformanceProfile::Balanced,
            fullscreen: false,
            height: 1080,
            width: 1920,
            refresh_rate: 60,
            theme: "default".to_string(),
            language: "en".to_string(),
            exit_on_escape: true,
        }
    }
}

#[cfg(not(feature = "std"))]
pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize logger
    logger::init().expect("Logger initialization failed");
    info!("Starting OS Gaming...");

    // Initialize kernel
    info!("Init Kernel...");
    match init_kernel(boot_info) {
        Ok(_) => info!("Kernel successfully initialized"),
        Err(e) => {
            error!("Error when Kernel initialize: {:?}", e);
            hcf();
        }
    }

    // Initialize GUI
    info!("Init GUI...");
    match init_gui() {
        Ok(_) => info!("GUI successfully initialized"),
        Err(e) => {
            error!("Error when GUI initialize: {:?}", e);
            hcf();
        }
    }

    loop {
        x86_64::instructions::hlt();
    }
}


fn init_kernel(boot_info: &'static BootInfo) -> Result<(), &'static str> {
    kernel::boot::init(boot_info)?;
    Ok(())
}


fn init_gui() -> Result<(), &'static str> {
    gui::init_gui(Default::default());
    Ok(())
}

// Fonction pour arrêter le CPU (Halt and Catch Fire)
#[cfg(not(feature = "std"))]
pub fn hcf() -> ! {
    loop {
        x86_64::instructions::hlt(); // Instruction pour arrêter le CPU jusqu'à la prochaine interruption
    }
}



#[cfg(not(feature = "bootloader-custom-config"))]
entry_point!(kernel_main);