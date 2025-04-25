//! System configuration
//!
//! This module provides configuration management for the operating system,
//! including loading/saving settings, profile management, and defaults.
#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::kernel::drivers::filesystem::{self, FileOpenMode};
use alloc::format;
use bincode;
use core::fmt;
use bincode::{Decode, Encode};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use spin::Mutex;
/// Main system configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct SystemConfig {
    /// Display settings
    pub display: DisplayConfig,

    /// Audio settings
    pub audio: AudioConfig,

    /// Network settings
    pub network: NetworkConfig,

    /// Input settings
    pub input: InputConfig,

    /// GPU settings
    pub gpu: GpuConfig,

    /// Performance settings
    pub performance: PerformanceConfig,

    /// Power management settings
    pub power: PowerConfig,

    /// Storage settings
    pub storage: StorageConfig,

    /// Window layout configuration
    pub window_layout: Option<WindowLayoutConfig>,

    /// Current profile
    pub active_profile: String,

    /// Custom user settings
    pub user_settings: UserSettings,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct DisplayConfig {
    /// Resolution (width, height)
    pub resolution: Option<(u32, u32)>,

    /// Refresh rate in Hz
    pub refresh_rate: u32,

    /// Color depth in bits per pixel
    pub color_depth: u8,

    /// Whether to use hardware acceleration
    pub hardware_acceleration: bool,

    /// Whether to use VSync
    pub vsync: bool,

    /// Scaling factor for UI elements
    pub ui_scale: f32,

    /// Gamma correction
    pub gamma: f32,

    /// Maximum framerate (0 for unlimited)
    pub max_framerate: u32,

    /// Whether to allow tearing for lower latency
    pub allow_tearing: bool,

    /// Whether to use fullscreen mode
    pub fullscreen: bool,
}

/// Audio configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct AudioConfig {
    /// Whether audio is enabled
    pub enabled: bool,

    /// Master volume (0-100)
    pub master_volume: u8,

    /// Sound effects volume (0-100)
    pub sfx_volume: u8,

    /// Music volume (0-100)
    pub music_volume: u8,

    /// Voice volume (0-100)
    pub voice_volume: u8,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Audio buffer size in frames
    pub buffer_size: u32,

    /// Whether to use hardware acceleration
    pub hardware_acceleration: bool,

    /// Whether surround sound is enabled
    pub surround: bool,

    /// Audio backend to use
    pub backend: String,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct NetworkConfig {
    /// Whether networking is enabled
    pub enabled: bool,

    /// Preferred interface
    pub preferred_interface: String,

    /// Whether to use DHCP
    pub use_dhcp: bool,

    /// Static IP address
    pub static_ip: Option<String>,

    /// Subnet mask
    pub subnet_mask: Option<String>,

    /// Default gateway
    pub gateway: Option<String>,

    /// DNS servers
    pub dns_servers: Vec<String>,

    /// Maximum bandwidth limit in KB/s (0 for unlimited)
    pub bandwidth_limit: u32,

    /// Connection timeout in seconds
    pub connection_timeout: u16,

    /// Whether to allow background network activity
    pub allow_background: bool,
}

/// Input configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct InputConfig {
    /// Keyboard layout
    pub keyboard_layout: String,

    /// Mouse sensitivity (1-10)
    pub mouse_sensitivity: u8,

    /// Mouse acceleration
    pub mouse_acceleration: f32,

    /// Whether to invert mouse Y axis
    pub invert_mouse_y: bool,

    /// Key repeat delay in ms
    pub key_repeat_delay: u16,

    /// Key repeat rate in ms
    pub key_repeat_rate: u16,

    /// Controller deadzone (0.0-1.0)
    pub controller_deadzone: f32,

    /// Controller vibration strength (0-100)
    pub controller_vibration: u8,

    /// Whether to swap A/B buttons on controller
    pub swap_ab_buttons: bool,

    /// Input device priority
    pub device_priority: Vec<String>,
}

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct GpuConfig {
    /// Preferred GPU (useful for multi-GPU systems)
    pub preferred_gpu: String,

    /// Texture quality (0=Low, 1=Medium, 2=High, 3=Ultra)
    pub texture_quality: u8,

    /// Shadow quality (0=Off, 1=Low, 2=Medium, 3=High)
    pub shadow_quality: u8,

    /// Anti-aliasing level (0=Off, 1=FXAA, 2=MSAA2x, 3=MSAA4x, 4=MSAA8x)
    pub antialiasing: u8,

    /// Anisotropic filtering level (0, 2, 4, 8, 16)
    pub anisotropic_filtering: u8,

    /// Whether to use tessellation
    pub tessellation: bool,

    /// Whether to use ray tracing
    pub ray_tracing: bool,

    /// VRAM usage limit in MB (0 for unlimited)
    pub vram_limit: u32,

    /// Shader quality (0=Low, 1=Medium, 2=High)
    pub shader_quality: u8,

    /// Whether to use compute shaders
    pub compute_shaders: bool,

    /// Whether to use asynchronous compute
    pub async_compute: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct PerformanceConfig {
    /// Process priority (0=Low, 1=Normal, 2=High)
    pub process_priority: u8,

    /// Whether to use all available CPU cores
    pub use_all_cores: bool,

    /// Maximum CPU usage percentage (0-100, 0 for unlimited)
    pub max_cpu_usage: u8,

    /// Thread pool size (0 for automatic)
    pub thread_pool_size: u16,

    /// IO buffer size in KB
    pub io_buffer_size: u32,

    /// Whether to preload assets
    pub preload_assets: bool,

    /// Memory pool size in MB
    pub memory_pool_size: u32,

    /// Whether to use memory compression
    pub memory_compression: bool,

    /// Whether to optimize for latency or throughput
    pub optimize_for_latency: bool,

    /// Whether to enable aggressive optimizations
    pub aggressive_optimization: bool,
}

/// Power management configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct PowerConfig {
    /// Power profile (0=Balanced, 1=Performance, 2=Power Saving)
    pub power_profile: u8,

    /// Whether to reduce performance on battery
    pub reduce_on_battery: bool,

    /// Screen timeout in seconds (0 for never)
    pub screen_timeout: u16,

    /// System sleep timeout in minutes (0 for never)
    pub sleep_timeout: u16,

    /// CPU governor (e.g., "ondemand", "performance", "powersave")
    #[serde(default)]
    pub cpu_governor: String,

    /// GPU power state (0=Auto, 1=Full, 2=Reduced, 3=Minimum)
    pub gpu_power_state: u8,

    /// Whether to enable dynamic frequency scaling
    pub dynamic_frequency: bool,

    /// Low battery threshold percentage
    pub low_battery_threshold: u8,

    /// Critical battery threshold percentage
    pub critical_battery_threshold: u8,

    /// Whether to show battery percentage
    pub show_battery_percentage: bool,
}


/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct StorageConfig {
    /// Cache size in MB
    pub cache_size: u32,

    /// Whether to use disk cache
    pub use_disk_cache: bool,

    /// Whether to compress temporary files
    pub compress_temp_files: bool,

    /// Auto-save interval in minutes (0 to disable)
    pub autosave_interval: u16,

    /// Maximum log file size in MB
    pub max_log_size: u32,

    /// How many days to keep logs
    pub log_retention_days: u8,

    /// Whether to use memory-mapped files where possible
    pub use_memory_mapped_files: bool,

    /// Whether to verify file integrity on load
    pub verify_file_integrity: bool,

    /// IO scheduler (e.g., "cfq", "noop", "deadline")
    pub io_scheduler: String,

    /// Whether to sync data immediately
    pub sync_immediately: bool,
}

/// Window layout configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct WindowLayoutConfig {
    /// Stored window positions and sizes
    pub windows: Vec<WindowPosition>,

    /// Default window theme
    pub default_theme: String,

    /// Whether to remember window positions
    pub remember_positions: bool,

    /// Whether to use window animations
    pub use_animations: bool,

    /// Window border thickness
    pub border_thickness: u8,

    /// Window corner radius
    pub corner_radius: u8,

    /// Whether to allow transparent windows
    pub allow_transparency: bool,

    /// Default window opacity (0-255)
    pub default_opacity: u8,
}

/// Individual window position
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct WindowPosition {
    /// Window ID or name
    pub id: String,

    /// Position (x, y)
    pub position: (i32, i32),

    /// Size (width, height)
    pub size: (u32, u32),

    /// Whether window is minimized
    pub minimized: bool,

    /// Whether window is maximized
    pub maximized: bool,

    /// Window z-order (lower numbers are more in front)
    pub z_order: u16,
}

/// User-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct UserSettings {
    /// Username
    pub username: String,

    /// User language
    pub language: String,

    /// Color scheme (light/dark/custom)
    pub color_scheme: String,

    /// Accessibility options
    pub accessibility: AccessibilityConfig,

    /// Custom key bindings
    pub key_bindings: Vec<KeyBinding>,

    /// Recently used applications
    pub recent_apps: Vec<String>,

    /// User notifications settings
    pub notifications: NotificationSettings,
}

/// Accessibility configuration
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct AccessibilityConfig {
    /// Whether high contrast mode is enabled
    pub high_contrast: bool,

    /// Text scaling factor
    pub text_scale: f32,

    /// Whether screen reader is enabled
    pub screen_reader: bool,

    /// Whether color blindness correction is enabled
    pub color_blindness_correction: u8, // 0=Off, 1=Protanopia, 2=Deuteranopia, 3=Tritanopia

    /// Whether to reduce animations
    pub reduce_animations: bool,

    /// Whether to enable keyboard navigation
    pub keyboard_navigation: bool,
}

/// Custom key binding
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct KeyBinding {
    /// Action name
    pub action: String,

    /// Key code
    pub key_code: u16,

    /// Modifier keys
    pub modifiers: u8,
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(crate = "serde")]
pub struct NotificationSettings {
    /// Whether notifications are enabled
    pub enabled: bool,

    /// Whether to show notifications in fullscreen mode
    pub show_in_fullscreen: bool,

    /// Notification display duration in seconds
    pub duration: u8,

    /// Whether to play sound on notification
    pub play_sound: bool,

    /// Maximum number of notifications to show at once
    pub max_visible: u8,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            audio: AudioConfig::default(),
            network: NetworkConfig::default(),
            input: InputConfig::default(),
            gpu: GpuConfig::default(),
            performance: PerformanceConfig::default(),
            power: PowerConfig::default(),
            storage: StorageConfig::default(),
            window_layout: Some(WindowLayoutConfig::default()),
            active_profile: "Balanced".into(),
            user_settings: UserSettings::default(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            resolution: Some((1920, 1080)),
            refresh_rate: 60,
            color_depth: 32,
            hardware_acceleration: true,
            vsync: true,
            ui_scale: 1.0,
            gamma: 1.0,
            max_framerate: 144,
            allow_tearing: false,
            fullscreen: false,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 80,
            sfx_volume: 80,
            music_volume: 70,
            voice_volume: 85,
            sample_rate: 48000,
            buffer_size: 1024,
            hardware_acceleration: true,
            surround: false,
            backend: "default".into(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preferred_interface: "eth0".into(),
            use_dhcp: true,
            static_ip: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: vec!["8.8.8.8".into(), "1.1.1.1".into()],
            bandwidth_limit: 0,
            connection_timeout: 30,
            allow_background: true,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            keyboard_layout: "us".into(),
            mouse_sensitivity: 5,
            mouse_acceleration: 1.0,
            invert_mouse_y: false,
            key_repeat_delay: 500,
            key_repeat_rate: 30,
            controller_deadzone: 0.1,
            controller_vibration: 80,
            swap_ab_buttons: false,
            device_priority: vec!["keyboard".into(), "controller".into(), "mouse".into()],
        }
    }
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            preferred_gpu: "auto".into(),
            texture_quality: 2,
            shadow_quality: 2,
            antialiasing: 2,
            anisotropic_filtering: 8,
            tessellation: true,
            ray_tracing: false,
            vram_limit: 0,
            shader_quality: 2,
            compute_shaders: true,
            async_compute: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            process_priority: 1,
            use_all_cores: true,
            max_cpu_usage: 0,
            thread_pool_size: 0,
            io_buffer_size: 1024,
            preload_assets: true,
            memory_pool_size: 512,
            memory_compression: true,
            optimize_for_latency: true,
            aggressive_optimization: false,
        }
    }
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            power_profile: 0,
            reduce_on_battery: true,
            screen_timeout: 300,
            sleep_timeout: 30,
            cpu_governor: "ondemand".into(),
            gpu_power_state: 0,
            dynamic_frequency: true,
            low_battery_threshold: 20,
            critical_battery_threshold: 10,
            show_battery_percentage: true,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cache_size: 512,
            use_disk_cache: true,
            compress_temp_files: false,
            autosave_interval: 5,
            max_log_size: 10,
            log_retention_days: 7,
            use_memory_mapped_files: true,
            verify_file_integrity: false,
            io_scheduler: "cfq".into(),
            sync_immediately: false,
        }
    }
}

impl Default for WindowLayoutConfig {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            default_theme: "default".into(),
            remember_positions: true,
            use_animations: true,
            border_thickness: 1,
            corner_radius: 3,
            allow_transparency: true,
            default_opacity: 255,
        }
    }
}

impl WindowLayoutConfig {
    pub fn from_gui_layout() -> Self {
        Self {
            windows: Vec::new(),
            default_theme: "default".into(),
            remember_positions: true,
            use_animations: true,
            border_thickness: 1,
            corner_radius: 3,
            allow_transparency: true,
            default_opacity: 255,
        }
    }
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            username: "user".into(),
            language: "en_US".into(),
            color_scheme: "dark".into(),
            accessibility: AccessibilityConfig::default(),
            key_bindings: Vec::new(),
            recent_apps: Vec::new(),
            notifications: NotificationSettings::default(),
        }
    }
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            high_contrast: false,
            text_scale: 1.0,
            screen_reader: false,
            color_blindness_correction: 0,
            reduce_animations: false,
            keyboard_navigation: true,
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_in_fullscreen: false,
            duration: 5,
            play_sound: true,
            max_visible: 3,
        }
    }
}

impl SystemConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    pub fn load() -> Result<Self, ConfigError> {
        load_system_config()
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), ConfigError> {
        save_system_config(self)
    }

    /// Apply a profile
    pub fn apply_profile(&mut self, profile: SystemConfig) {
        // Save the profile name
        let profile_name = profile.active_profile.clone();

        // Copy over all settings
        *self = profile;

        // Update the active profile
        self.active_profile = profile_name.clone();

        log::info!("Applied profile: {}", profile_name);
    }

    /// Optimize for performance
    pub fn optimize_performance(&mut self) {
        // Display settings
        self.display.vsync = false;
        self.display.max_framerate = 0; // Unlimited
        self.display.allow_tearing = true;

        // GPU settings
        self.gpu.texture_quality = 1; // Medium
        self.gpu.shadow_quality = 1; // Medium
        self.gpu.antialiasing = 1; // FXAA (Low)
        self.gpu.ray_tracing = false;
        self.gpu.shader_quality = 1; // Medium

        // Performance settings
        self.performance.process_priority = 2; // High
        self.performance.use_all_cores = true;
        self.performance.preload_assets = true;
        self.performance.optimize_for_latency = true;
        self.performance.aggressive_optimization = true;

        // Power settings
        self.power.power_profile = 1; // Performance
        self.power.cpu_governor = "performance".into();
        self.power.gpu_power_state = 1; // Full power
        self.power.dynamic_frequency = false;

        log::info!("Applied performance optimizations");
    }

    /// Optimize for power saving
    pub fn optimize_power(&mut self) {
        // Display settings
        self.display.vsync = true;
        self.display.max_framerate = 60;
        self.display.allow_tearing = false;

        // GPU settings
        self.gpu.texture_quality = 0; // Low
        self.gpu.shadow_quality = 0; // Off
        self.gpu.antialiasing = 0; // Off
        self.gpu.ray_tracing = false;
        self.gpu.shader_quality = 0; // Low

        // Performance settings
        self.performance.process_priority = 0; // Low
        self.performance.max_cpu_usage = 70; // Limit to 70%
        self.performance.preload_assets = false;
        self.performance.optimize_for_latency = false;

        // Power settings
        self.power.power_profile = 2; // Power saving
        self.power.cpu_governor = "powersave".into();
        self.power.gpu_power_state = 2; // Reduced
        self.power.screen_timeout = 60; // 1 minute
        self.power.sleep_timeout = 5; // 5 minutes

        log::info!("Applied power-saving optimizations");
    }

    /// Create a balanced profile
    pub fn create_balanced_profile() -> Self {
        let mut config = Self::default();
        config.active_profile = "Balanced".into();
        config
    }

    /// Create a performance profile
    pub fn create_performance_profile() -> Self {
        let mut config = Self::default();
        config.active_profile = "Performance".into();
        config.optimize_performance();
        config
    }

    /// Create a power-saving profile
    pub fn create_power_saving_profile() -> Self {
        let mut config = Self::default();
        config.active_profile = "Power Saving".into();
        config.optimize_power();
        config
    }

    /// Get all available profiles
    pub fn get_available_profiles() -> Vec<String> {
        vec![
            "Balanced".into(),
            "Performance".into(),
            "Power Saving".into(),
        ]
    }
}

/// Config error types
#[derive(Debug, Clone)]
pub enum ConfigError {
    IoError(&'static str),
    ParseError(&'static str),
    InvalidValue(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "I/O Error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid Value: {}", msg),
        }
    }
}

/// Load system configuration from file
pub fn load_system_config() -> Result<SystemConfig, ConfigError> {
    // Try to open the config file (using the binary extension)
    let config_path = "/etc/os_gaming/config.bin"; // Use matching path from save function

    // Create a new filesystem manager
    let fs_manager = filesystem::FilesystemManager::new();

    match fs_manager.open_file(config_path, true) {
        // Use enum for clarity if available
        Ok(mut file) => {
            // Create a buffer to hold file content
            let mut buffer = Vec::new();
            let mut read_buf = [0u8; 4096]; // Temporary buffer for each read call

            // Read the file in chunks until we've got it all
            loop {
                // Assuming file.read takes a mutable slice and returns Result<usize, _>
                match file.read(&mut read_buf, &fs_manager, 0) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            // End of file
                            break;
                        }
                        // Append the read bytes to our main buffer
                        buffer.extend_from_slice(&read_buf[..bytes_read]);
                    }
                    Err(e) => {
                        // Log the specific error if possible, e.g., log::error!("Error reading config file {}: {:?}", config_path, e);
                        return Err(ConfigError::IoError("Failed to read config file"));
                    }
                }
            }

            let config: Result<SystemConfig, _> = bincode::decode_from_slice(
                &buffer,
                bincode::config::standard()
            ).map(|(decoded, _)| decoded);


            match config {
                Ok(decoded_config) => {
                    log::info!(
            "System configuration loaded successfully from {}",
            config_path
        );
                    Ok(decoded_config)
                }
                Err(e) => {
                    // Log the specific error if possible
                    Err(ConfigError::ParseError(
                        "Invalid config file format (bincode)"
                    ))
                }
            }
        }
        Err(e) => {
            // File doesn't exist or couldn't be opened, create default config
            // Log the specific error if possible, e.g., log::info!("Config file {} not found or error opening: {:?}. Creating default.", config_path, e);
            log::info!("Config file not found, creating default configuration");
            let config = SystemConfig::default();
            // Try to save it using the bincode save function
            if let Err(save_err) = save_system_config(&config) {
                // Log the specific save error if possible, e.g., log::warn!("Failed to save default configuration: {:?}", save_err);
                log::warn!("Failed to save default configuration");
            }
            Ok(config)
        }
    }
}

/// Save system configuration to file
pub fn save_system_config(config: &SystemConfig) -> Result<(), ConfigError> {
    // Serialize config using bincode
    let bytes = match bincode::encode_to_vec(config, bincode::config::standard()) {
        Ok(bytes) => bytes,
        Err(_) => return Err(ConfigError::ParseError("Failed to serialize config using bincode")),
    };

    // Create directory if it doesn't exist
    let mut fs_manager = filesystem::FilesystemManager::new();

    // Create directory
    let dir_path = "/etc/os_gaming";
    // Use if let Err(_) = ... to avoid unused result warning if create_directory returns Result<(), _>
    if let Err(e) = fs_manager.create_directory(dir_path) {
        // Log the specific error if possible, e.g., log::warn!("Failed to create config directory {}: {:?}", dir_path, e);
        log::warn!("Failed to create config directory {}", dir_path);
        // Decide if this should be a hard error or just a warning
        // return Err(ConfigError::IoError("Failed to create config directory"));
    }

    // Write to file (consider changing extension from .toml to .bin or .cfg)
    let config_path = "/etc/os_gaming/config.bin"; // Changed extension
    match fs_manager.open_file(config_path, true) {
        // Use enum for clarity if available
        Ok(mut file) => {
            let mut position = 0;

            while position < bytes.len() {
                // Pass the slice directly
                match file.write(&bytes[position..], &fs_manager) {
                    Ok(bytes_written) => {
                        if bytes_written == 0 {
                            // Avoid infinite loop if write returns 0 without error
                            log::error!("File write returned 0 bytes written, potential stall.");
                            return Err(ConfigError::IoError(
                                "Failed to write config file (wrote 0 bytes)",
                            ));
                        }
                        position += bytes_written;
                    }
                    Err(e) => {
                        // Log the specific error if possible, e.g., log::error!("Error writing config file: {:?}", e);
                        return Err(ConfigError::IoError("Failed to write config file"));
                    }
                }
            }

            log::info!("System configuration saved to {}", config_path);
            Ok(())
        }
        Err(e) => {
            // Log the specific error if possible, e.g., log::error!("Error opening config file {}: {:?}", config_path, e);
            Err(ConfigError::IoError(
                "Failed to open config file for writing",
            ))
        }
    }
}

/// Global configuration
lazy_static! {
    static ref CONFIG: Mutex<SystemConfig> = Mutex::new(load_system_config().unwrap_or_else(|e| {
        log::warn!("Failed to load config: {}, using defaults", e);
        SystemConfig::default()
    }));
}

/// Get global configuration
pub fn get_config() -> &'static Mutex<SystemConfig> {
    &CONFIG
}
