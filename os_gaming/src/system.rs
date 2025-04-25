//! System initialization and management
//!
//! This module coordinates the initialization of all system components
//! and provides the interface between kernel services and the GUI.
extern crate alloc;
use core::arch::asm;

use alloc::string::String;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::config::{load_system_config, SystemConfig};
use crate::gui::{self, FontManager, Renderer, Theme, WindowLayoutConfig, WindowManager};
use crate::kernel::drivers::filesystem as fs;
use crate::kernel::drivers::filesystem::{FilesystemManager};
use crate::kernel::{self, drivers, interrupts, memory};
use bincode::{Decode, Encode};
use alloc::boxed::Box;

/// System state flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Initializing,
    Running,
    Suspended,
    Shutdown,
    Error,
}

/// System configuration
pub struct System {
    /// Current system state
    state: Mutex<SystemState>,

    /// System configuration
    config: Mutex<SystemConfig>,

    /// Display resolution
    resolution: (u32, u32),

    /// System uptime in milliseconds
    uptime: Mutex<u64>,

    /// Window manager
    window_manager: Option<Mutex<WindowManager>>,

    /// Event handlers
    event_handlers: Mutex<Vec<fn(SystemEvent)>>,

    /// Whether audio is enabled
    audio_enabled: bool,

    /// Whether networking is enabled
    network_enabled: bool,
}

// Safety: We're manually asserting that System can be shared between threads
// This is safe because we're using Mutex for all mutable access and ensuring proper synchronization
unsafe impl Send for System {}
unsafe impl Sync for System {}

/// System event types
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Input event (keyboard, mouse, gamepad)
    Input(InputEvent),

    /// Window management event
    Window(WindowEvent),

    /// System state change
    StateChange(SystemState),

    /// Low memory warning
    LowMemory,

    /// Power management event
    Power(PowerEvent),

    /// Custom event with arbitrary data
    Custom(u32, Vec<u8>),
}

/// Input event types
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Keyboard event
    Key {
        key: u16,
        pressed: bool,
        modifiers: u8,
    },

    /// Mouse event
    Mouse {
        x: i32,
        y: i32,
        buttons: u8,
        scroll_delta: i8,
    },

    /// Gamepad event
    Gamepad {
        id: u8,
        buttons: u32,
        axes: [i16; 6],
    },

    /// Touch event
    Touch {
        id: u8,
        x: i32,
        y: i32,
        pressure: u8,
    },
}

/// Window event types
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// Window was created
    Created(u32),

    /// Window was destroyed
    Destroyed(u32),

    /// Window was resized
    Resized(u32, u32, u32),

    /// Window was moved
    Moved(u32, i32, i32),

    /// Window focus changed
    FocusChanged(u32, bool),

    /// Window visibility changed
    VisibilityChanged(u32, bool),
}

/// Power management events
#[derive(Debug, Clone)]
pub enum PowerEvent {
    /// Battery level change
    BatteryLevel(u8),

    /// Power source changed
    PowerSourceChanged(PowerSource),

    /// System is about to suspend
    Suspend,

    /// System resumed from suspend
    Resume,

    /// Low battery warning
    LowBattery,
}

/// Power source types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    Battery,
    AC,
    Unknown,
}

impl System {
    /// Create a new system instance
    pub fn new() -> Self {
        Self {
            state: Mutex::new(SystemState::Initializing),
            config: Mutex::new(SystemConfig::default()),
            resolution: (1280, 720), // Default resolution
            uptime: Mutex::new(0),
            window_manager: None,
            event_handlers: Mutex::new(Vec::new()),
            audio_enabled: false,
            network_enabled: false,
        }
    }

    /// Initialize the system
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Set initializing state
        *self.state.lock() = SystemState::Initializing;

        // Load system configuration
        match load_system_config() {
            Ok(config) => *self.config.lock() = config,
            Err(e) => log::warn!("Failed to load system config: {}", e),
        }

        // Initialize kernel services
        log::info!("Initializing kernel services...");

        // 1. Initialize memory management first
        log::info!("Initializing memory management...");
        memory::init().map_err(|e| {
            log::error!("Memory initialization failed: {}", e);
            "Memory initialization failed"
        })?;

        // 2. Initialize interrupt handling
        log::info!("Initializing interrupt handling...");
        interrupts::init();

        // 3. Initialize drivers
        log::info!("Initializing device drivers...");
        drivers::init();
        // 4. Initialize filesystem
        log::info!("Initializing filesystem...");
        FilesystemManager::init();

        // Initialize GUI subsystems
        log::info!("Initializing GUI subsystems...");

        // 1. Check for a valid graphics mode
        let config = self.config.lock();
        let (width, height) = config.display.resolution.unwrap_or((1280, 720));
        drop(config);

        // 2. Initialize the renderer
        log::info!("Initializing renderer at {}x{}", width, height);
        let renderer = gui::init_renderer(width, height).map_err(|e| {
            log::error!("Renderer initialization failed: {}", e);
            "Renderer initialization failed"
        })?;

        // 3. Initialize font system
        log::info!("Initializing font system...");
        gui::init_fonts().map_err(|e| {
            log::error!("Font initialization failed: {}", e);
            "Font initialization failed"
        })?;

        // 4. Initialize window manager
        log::info!("Initializing window manager...");
        let wm = gui::init_window_manager(renderer).map_err(|e| {
            log::error!("Window manager initialization failed: {}", e);
            "Window manager initialization failed"
        })?;

        self.window_manager = Some(Mutex::new(wm));
        self.resolution = (width, height);

        // Initialize audio if enabled
        if self.config.lock().audio.enabled {
            log::info!("Initializing audio subsystem...");
            if drivers::sound::init().is_ok() {
                self.audio_enabled = true;
            } else {
                log::warn!("Audio initialization failed, continuing without audio");
            }
        }

        // Initialize networking if enabled
        if self.config.lock().network.enabled {
            log::info!("Initializing network subsystem...");
            if drivers::network::init().is_ok() {
                self.network_enabled = true;
            } else {
                log::warn!("Network initialization failed, continuing without networking");
            }
        }

        // Register input handlers
        self.register_default_handlers();

        // System is now running
        log::info!("System initialization complete!");
        *self.state.lock() = SystemState::Running;

        Ok(())
    }

    /// Run the system main loop
    pub fn run(&mut self) -> ! {
        log::info!("Entering system main loop");

        // Start the system timer
        kernel::drivers::timer::start_system_timer();

        loop {
            // Process pending events
            self.process_events();

            // Update window manager
            if let Some(wm) = &self.window_manager {
                wm.lock().update();
            }

            // Check system state
            match *self.state.lock() {
                SystemState::Running => {
                    // Normal operation
                    self.update_uptime();
                }
                SystemState::Suspended => {
                    // Low-power mode
                    kernel::drivers::power::enter_sleep_mode();
                }
                SystemState::Shutdown => {
                    // Perform shutdown sequence
                    self.shutdown();
                }
                SystemState::Error => {
                    // System error - show error screen
                    self.show_error_screen();
                    // After error handling, go back to running state
                    *self.state.lock() = SystemState::Running;
                }
                _ => {}
            }
        }
    }

    /// Process system events
    fn process_events(&self) {
        // Process input events
        let mut input_events = Vec::new();

        // Collect keyboard events
        if let Ok(events) = drivers::keyboard::get_pending_events() {
            for event in events {
                input_events.push(SystemEvent::Input(InputEvent::Key {
                    key: event.key_code,
                    pressed: event.alt_pressed || event.ctrl_pressed || event.shift_pressed,
                    modifiers: event.modifiers,
                }));
            }
        }

        // Collect mouse events
        if let Ok(events) = drivers::mouse::get_pending_events() {
            for event in events {
                input_events.push(SystemEvent::Input(InputEvent::Mouse {
                    x: event.x,
                    y: event.y,
                    buttons: event.buttons,
                    scroll_delta: event.scroll_wheel,
                }));
            }
        } else {
            log::error!("Failed to get mouse events");
        }

        // Collect gamepad events
        if let Ok(events) = drivers::gamepad::get_pending_events() {
            for event in events {
                input_events.push(SystemEvent::Input(InputEvent::Gamepad {
                    id: event.id,
                    buttons: event.buttons,
                    axes: event.axes,
                }));
            }
        }

        // Dispatch all input events
        for event in input_events {
            self.dispatch_event(event);
        }
    }

    /// Dispatch a system event to all registered handlers
    fn dispatch_event(&self, event: SystemEvent) {
        // Get a copy of the event handlers to avoid holding the lock during callbacks
        let handlers = self.event_handlers.lock().clone();

        // Pass the event to all handlers
        for handler in handlers {
            handler(event.clone());
        }

        // Special handling for window manager events
        if let SystemEvent::Input(input) = &event {
            if let Some(wm) = &self.window_manager {
                let mut wm = wm.lock();
                match input {
                    InputEvent::Key {
                        key,
                        pressed,
                        modifiers,
                    } => {
                        wm.handle_key_event(*key, *pressed, *modifiers);
                    }
                    InputEvent::Mouse {
                        x,
                        y,
                        buttons,
                        scroll_delta,
                    } => {
                        wm.handle_mouse_event(*x, *y, *buttons, *scroll_delta);
                    }
                    InputEvent::Touch { id, x, y, pressure } => {
                        wm.handle_touch_event(*id, *x, *y, *pressure);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Register an event handler
    pub fn register_event_handler(&self, handler: fn(SystemEvent)) {
        self.event_handlers.lock().push(handler);
    }

    /// Register default event handlers
    fn register_default_handlers(&self) {
        // Register power management handler
        self.register_event_handler(|event| {
            if let SystemEvent::Power(power_event) = event {
                match power_event {
                    PowerEvent::LowBattery => {
                        log::warn!("Low battery warning!");
                        // Show low battery notification
                    }
                    PowerEvent::PowerSourceChanged(source) => {
                        match source {
                            PowerSource::Battery => {
                                log::info!("Switched to battery power");
                                // Enable power saving mode
                            }
                            PowerSource::AC => {
                                log::info!("Switched to AC power");
                                // Disable power saving mode
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Update system uptime
    fn update_uptime(&self) {
        let current_time = kernel::drivers::timer::get_system_time_ms();
        let mut uptime = self.uptime.lock();
        *uptime = current_time;
    }

    /// Get current system uptime in milliseconds
    pub fn get_uptime(&self) -> u64 {
        *self.uptime.lock()
    }

    /// Get current system state
    pub fn get_state(&self) -> SystemState {
        *self.state.lock()
    }

    /// Set system state
    pub fn set_state(&self, new_state: SystemState) {
        let mut state = self.state.lock();
        *state = new_state;

        // Dispatch state change event
        self.dispatch_event(SystemEvent::StateChange(new_state));
    }

    /// Shutdown the system
    fn shutdown(&self) -> ! {
        log::info!("System shutting down...");

        // Save any unsaved data
        self.save_system_state();

        // Close all open windows
        if let Some(wm) = &self.window_manager {
            wm.lock().close_all_windows();
        }

        // Shutdown subsystems in reverse order
        log::info!("Shutting down subsystems...");

        if self.network_enabled {
            drivers::network::shutdown();
        }

        if self.audio_enabled {
            drivers::sound::shutdown();
        }

        // Shutdown filesystem
        fs::shutdown();

        // Perform platform-specific shutdown
        log::info!("Goodbye!");

        // This function must not return, so it should call a diverging function
        kernel::drivers::power::shutdown();

        // In case shutdown returns (which it shouldn't)
        loop {
            // Halt the CPU
            core::hint::spin_loop();
        }
    }

    /// Save system state before shutdown
    fn save_system_state(&self) {
        // Save window positions and states
        if let Some(wm) = &self.window_manager {
            let windows_layout = wm.lock().save_layout();

            // Convert Rect to serializable format
            let serializable_windows: Vec<(u32, String, (i32, i32, u32, u32))> = windows_layout
                .into_iter()
                .map(|(id, title, rect)| (id, title, (rect.x, rect.y, rect.width, rect.height)))
                .collect();

            // Now serialize the converted data
            let encoded =
                match bincode::encode_to_vec(serializable_windows, bincode::config::standard()) {
                    Ok(data) => data,
                    Err(e) => {
                        log::error!("Failed to serialize system state: {}", e);
                        return;
                    }
                };

            let fs_manager = FilesystemManager::new();
            // Write to file using the filesystem API
            match fs_manager.open_file("/etc/os_gaming/system_state.bin", false) {
                Ok(mut file) => {
                    if let Err(_) = file.write(&encoded, &fs_manager) {
                        log::error!("Failed to write system state file");
                    } else {
                        log::info!("System state saved ({} bytes)", encoded.len());
                    }
                }
                Err(_) => log::error!("Failed to open system state file for writing"),
            }
        }
    }

    /// Show error screen
    fn show_error_screen(&self) {
        if let Some(wm) = &self.window_manager {
            let mut wm = wm.lock();

            // Create error dialog
            let window_id = wm
                .create_window("System Error", 400, 300, true)
                .unwrap_or(0);
            if window_id > 0 {
                // Populate with error details
                // This would use your GUI widgets to create a proper error screen
                wm.show_window(window_id);
            }
        }
    }

    /// Launch an application
    pub fn launch_application(&self, app_name: &str) -> Result<u32, &'static str> {
        log::info!("Launching application: {}", app_name);

        // Find the application
        let app_info = match self.find_application(app_name) {
            Some(info) => info,
            None => return Err("Application not found"),
        };

        // Load the application
        let app = match self.load_application(&app_info) {
            Ok(app) => app,
            Err(_) => return Err("Failed to load application"),
        };

        // Create a window for the application
        let window_id = if let Some(wm) = &self.window_manager {
            let mut wm = wm.lock();
            wm.create_window(app_name, 800, 600, true).unwrap_or(0)
        } else {
            return Err("Window manager not initialized");
        };

        if window_id == 0 {
            return Err("Failed to create application window");
        }

        // Start the application
        app.run(window_id);

        // Show the window
        if let Some(wm) = &self.window_manager {
            wm.lock().show_window(window_id);
        }

        Ok(window_id)
    }

    /// Find application by name
    fn find_application(&self, app_name: &str) -> Option<AppInfo> {
        // This would search the filesystem for the application

        // Mock implementation
        if app_name == "settings" {
            Some(AppInfo {
                name: "Settings".into(),
                path: "/apps/settings".into(),
                version: "1.0".into(),
            })
        } else if app_name == "game" {
            Some(AppInfo {
                name: "Game".into(),
                path: "/apps/game".into(),
                version: "1.0".into(),
            })
        } else {
            None
        }
    }

    /// Load an application
    fn load_application(&self, app_info: &AppInfo) -> Result<Box<dyn Application>, &'static str> {
        // This would load the application binary or script
    
        // Mock implementation
        match app_info.name.as_str() {
            "Settings" => Ok(Box::new(MockApplication::new(app_info.name.clone()))),
            "Game" => Ok(Box::new(MockApplication::new(app_info.name.clone()))),
            _ => Err("Unsupported application"),
        }
    }
}

/// Application information
#[derive(Debug, Clone)]
struct AppInfo {
    name: String,
    path: String,
    version: String,
}

/// Application trait
trait Application {
    fn run(&self, window_id: u32);
    fn terminate(&self);
}

/// Mock application implementation
struct MockApplication {
    name: String,
}

impl MockApplication {
    fn new(name: String) -> Self {
        Self { name }
    }
}

impl Application for MockApplication {
    fn run(&self, window_id: u32) {
        log::info!("Application {} running in window {}", self.name, window_id);
    }

    fn terminate(&self) {
        log::info!("Application {} terminated", self.name);
    }
}

// Global system instance
lazy_static! {
    /// Global system instance
    static ref SYSTEM: Mutex<System> = Mutex::new(System::new());
}

/// Get the global system instance
pub fn get_system() -> &'static Mutex<System> {
    &SYSTEM
}

/// Initialize the system
pub fn init() -> Result<(), &'static str> {
    let mut system = SYSTEM.lock();
    system.init()
}

/// Start the system main loop
pub fn run() -> ! {
    let mut system = SYSTEM.lock();
    system.run()
}

pub fn get_system_info() -> SystemConfig {
    SYSTEM.lock().config.lock().clone()
}

pub fn apply_profile(profile: SystemConfig) {
    let mut system = SYSTEM.lock();
    system.config.lock().apply_profile(profile);
}

pub fn optimize_performance() {
    let mut system = SYSTEM.lock();
    system.config.lock().optimize_performance();
}
