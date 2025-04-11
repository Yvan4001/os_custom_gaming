pub mod keyboard;
pub mod vga;
pub mod mouse;
pub mod sound;
pub mod storage;
pub mod network;
pub mod display;
pub mod usb;
pub mod gamepad;
pub mod power;
pub mod filesystem;
pub mod timer;
pub mod acpi;

use spin::Mutex;
use lazy_static::lazy_static;

/// Container for all driver managers
pub struct DriverManager {
    pub network_manager: network::NetworkManager,
    pub storage_manager: storage::StorageManager,
    pub usb_manager: usb::UsbManager,
    pub input_manager: InputManager,
    pub sound_system: sound::SoundDriver,
}

/// Input device abstraction
pub struct InputManager {
    pub keyboard: keyboard::KeyboardState,
    pub mouse: mouse::MouseState,
    pub gamepads: gamepad::GamepadManager,
}

// Global access to driver manager
lazy_static! {
    pub static ref DRIVER_MANAGER: Mutex<Option<DriverManager>> = Mutex::new(None);
}

/// Initialize all drivers
pub fn init() -> Result<(), &'static str> {
    // Initialize display first for debugging output
    display::init()?;
    
    // Initialize hardware timer/clock
    timer::init()?;
    
    // Initialize ACPI for power management
    acpi::init()?;
    
    // Initialize network subsystem
    let net_manager = network::init()?;
    
    // Initialize storage devices
    let storage_manager = storage::init()?;
    
    // Initialize USB bus
    let usb_manager = usb::init()?;
    
    // Initialize input devices
    let keyboard_state = keyboard::init()?;
    let mouse_state = mouse::init()?;
    let gamepad_manager = gamepad::init()?;
    
    let input_manager = InputManager {
        keyboard: keyboard_state,
        mouse: mouse_state,
        gamepads: gamepad_manager,
    };
    
    // Initialize sound system
    let sound_system = sound::init()?;
    
    // Initialize filesystem
    filesystem::init(&storage_manager)?;
    
    // Initialize power management
    power::init()?;
    
    // Create driver manager instance
    let manager = DriverManager {
        network_manager: net_manager,
        storage_manager,
        usb_manager,
        input_manager,
        sound_system,
    };
    
    // Store global reference
    *DRIVER_MANAGER.lock() = Some(manager);
    
    #[cfg(debug_assertions)]
    println!("All drivers initialized successfully");
    
    Ok(())
}

/// Get the driver manager
pub fn get_driver_manager() -> &'static Mutex<Option<DriverManager>> {
    &DRIVER_MANAGER
}