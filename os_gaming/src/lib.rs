#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]
#![feature(core_intrinsics)]
#![feature(asm_experimental_arch)]
#![feature(asm_const)]
#![feature(global_asm)]
#![feature(abi_x86_interrupt)]
// Set up proper imports for different environments
#[cfg(feature = "std")]
pub use std::boxed::Box;
#[cfg(feature = "std")]
pub use std::string::String;
#[cfg(feature = "std")]
pub use std::vec::Vec;

#[cfg(not(feature = "std"))]
pub use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
pub use alloc::string::String;
#[cfg(not(feature = "std"))]
pub use alloc::vec::Vec;
use core::arch::asm;
use core::cmp::PartialEq;
use core::result::Result::Ok;
use core::result::Result;
use core::marker::Copy;


unsafe extern "C" fn __stack_chk_fail() {
    asm!("sti", options(nomem, nostack, preserves_flags));
}

#[cfg(feature = "std")]
use toml;

#[macro_use]
extern crate core;

// Import necessary components for no_std
#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

// Public modules
pub mod config;
#[cfg(feature = "std")]
pub mod gui;
pub mod kernel;
pub mod system;

// System constants
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "OS Gaming System";

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

// Initialization function for the system
pub fn init() -> Result<Config, &'static str> {
    #[cfg(feature = "std")]
    log::info!("Initializing OS Gaming v{}", VERSION);

    let config = Config::default();

    #[cfg(feature = "std")]
    kernel::boot::init(&config)?;

    #[cfg(not(feature = "std"))]
    kernel::boot::init_bare_metal(&config)?;

    Ok(config)
}

#[cfg(not(feature = "std"))]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize your OS
    let _ = init();

    // Main OS loop - never returns
    loop {}
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
