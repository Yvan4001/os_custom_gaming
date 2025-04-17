pub mod keyboard;
pub mod vga;
pub mod hdmi;
pub mod mouse;
pub mod sound;
pub mod storage;
pub mod network;
pub mod display;
pub mod usb;
pub mod gamepad;
pub mod filesystem;
pub mod timer;
pub mod power;
pub mod displayport;
pub mod gpu;

use gamepad::GamepadManager;
use keyboard::KeyboardState;
use display::DisplayInfo;
use power::PowerManager;
use displayport::DisplayPortDriver;
use hdmi::HdmiDriver;
use vga::Writer;
use sound::SoundDriver;
use mouse::MouseState;
use spin::Mutex;
use lazy_static::lazy_static;

/// Container for all driver managers
pub struct DriverManager {
    pub network_manager: network::NetworkManager,
    pub storage_manager: storage::StorageManager,
    pub usb_manager: usb::UsbManager,
    pub input_manager: InputManager,
    pub sound_system: sound::SoundDriver,
    pub power_manager: power::PowerManager,
    pub filesystem_manager: filesystem::FilesystemManager,
    pub timer_manager: timer::TimerManager,
    pub displayport_manager: displayport::DisplayPortDriver,
    pub hdmi_manager: hdmi::HdmiDriver,
    pub vga_manager: vga::Writer,
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
    
    // Initialize network subsystem
    let net_manager = network::init()?;
    
    // Initialize storage devices
    let storage_manager = storage::init()?;
    
    // Initialize USB bus
    let usb_manager = usb::init()?;
    
    // Initialize input devices
    keyboard::init();
    mouse::init();
    let gamepad_manager = gamepad::init()?;
    let mouse_state = mouse::MouseState::new();
    let keyboard_state = KeyboardState::new();
    
    let input_manager = InputManager {
        keyboard: keyboard_state.into(),
        mouse: mouse_state.into(),
        gamepads: gamepad_manager.clone(),
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
        power_manager : power::PowerManager::new(),
        filesystem_manager: filesystem::FilesystemManager::new(),
        timer_manager: timer::TimerManager::new(),
        displayport_manager: DisplayPortDriver::new(),
        hdmi_manager: HdmiDriver::new(),
        vga_manager: Writer::new(),
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