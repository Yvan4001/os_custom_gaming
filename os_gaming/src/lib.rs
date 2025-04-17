#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(abi_x86_interrupt))]
#![feature(abi_x86_interrupt)]

// Public modules
#[cfg(feature = "std")]
pub mod gui;
pub mod kernel;
pub mod system;
pub mod config;

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
    
    Ok(config)
}