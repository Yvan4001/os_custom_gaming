extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

/// Gamepad types we can support
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadType {
    Unknown,
    XboxController,
    PlayStation,
    NintendoSwitch,
    Generic,
}

/// Input state for a gamepad
#[derive(Debug, Clone, Copy)]
pub struct GamepadState {
    pub id: usize,
    pub connected: bool,
    pub buttons: u32,       // Bit flags for button states
    pub left_stick_x: i16,  // -32768 to 32767
    pub left_stick_y: i16,  // -32768 to 32767
    pub right_stick_x: i16, // -32768 to 32767
    pub right_stick_y: i16, // -32768 to 32767
    pub left_trigger: u8,   // 0 to 255
    pub right_trigger: u8,  // 0 to 255
}

/// Button definitions
pub const BTN_A: u32 = 0x00000001;
pub const BTN_B: u32 = 0x00000002;
pub const BTN_X: u32 = 0x00000004;
pub const BTN_Y: u32 = 0x00000008;
pub const BTN_LEFT_SHOULDER: u32 = 0x00000010;
pub const BTN_RIGHT_SHOULDER: u32 = 0x00000020;
pub const BTN_BACK: u32 = 0x00000040;
pub const BTN_START: u32 = 0x00000080;
pub const BTN_LEFT_THUMB: u32 = 0x00000100;
pub const BTN_RIGHT_THUMB: u32 = 0x00000200;
pub const BTN_DPAD_UP: u32 = 0x00000400;
pub const BTN_DPAD_DOWN: u32 = 0x00000800;
pub const BTN_DPAD_LEFT: u32 = 0x00001000;
pub const BTN_DPAD_RIGHT: u32 = 0x00002000;
pub const BTN_GUIDE: u32 = 0x00004000;

/// Represents a gamepad/controller device
pub struct GamepadDevice {
    id: usize,
    name: String,
    gamepad_type: GamepadType,
    state: GamepadState,
    connected: AtomicBool,
}

/// Manages all gamepad devices
pub struct GamepadManager {
    devices: Vec<GamepadDevice>,
    next_id: usize,
}

// Global gamepad manager
lazy_static! {
    static ref GAMEPAD_MANAGER: Mutex<GamepadManager> = Mutex::new(GamepadManager::new());
}

impl GamepadDevice {
    /// Create a new gamepad device
    pub fn new(id: usize, name: String, gamepad_type: GamepadType) -> Self {
        Self {
            id,
            name,
            gamepad_type,
            state: GamepadState {
                id,
                connected: true,
                buttons: 0,
                left_stick_x: 0,
                left_stick_y: 0,
                right_stick_x: 0,
                right_stick_y: 0,
                left_trigger: 0,
                right_trigger: 0,
            },
            connected: AtomicBool::new(true),
        }
    }
    
    /// Update the gamepad state
    pub fn update_state(&mut self, new_state: GamepadState) {
        self.state = new_state;
    }
    
    /// Get the current gamepad state
    pub fn get_state(&self) -> GamepadState {
        self.state
    }
    
    /// Check if the gamepad is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
    
    /// Mark the gamepad as disconnected
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
    }
    
    /// Get the gamepad name
    pub fn get_name(&self) -> &str {
        &self.name
    }
    
    /// Get the gamepad type
    pub fn get_type(&self) -> GamepadType {
        self.gamepad_type
    }
    
    /// Get the gamepad ID
    pub fn get_id(&self) -> usize {
        self.id
    }
}

impl GamepadManager {
    /// Create a new gamepad manager
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_id: 0,
        }
    }
    
    /// Add a gamepad device
    pub fn add_device(&mut self, name: String, gamepad_type: GamepadType) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        
        let device = GamepadDevice::new(id, name, gamepad_type);
        self.devices.push(device);
        
        id
    }
    
    /// Get a gamepad by ID
    pub fn get_device(&self, id: usize) -> Option<&GamepadDevice> {
        self.devices.iter().find(|dev| dev.get_id() == id)
    }
    
    /// Get a mutable reference to a gamepad by ID
    pub fn get_device_mut(&mut self, id: usize) -> Option<&mut GamepadDevice> {
        self.devices.iter_mut().find(|dev| dev.get_id() == id)
    }
    
    /// Remove a gamepad
    pub fn remove_device(&mut self, id: usize) -> bool {
        if let Some(pos) = self.devices.iter().position(|dev| dev.get_id() == id) {
            self.devices.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Get the number of connected gamepads
    pub fn get_connected_count(&self) -> usize {
        self.devices.iter().filter(|dev| dev.is_connected()).count()
    }
    
    /// Get all devices
    pub fn get_devices(&self) -> &[GamepadDevice] {
        &self.devices
    }
}

/// Initialize the gamepad subsystem
pub fn init() -> Result<GamepadManager, &'static str> {
    let mut manager = GamepadManager::new();
    
    // In a real driver, we would scan for USB HID devices and/or
    // other inputs that could be gamepads
    
    #[cfg(feature = "std")]
    {
        // For testing, create some virtual controllers
        manager.add_device(
            "Xbox Controller".to_string(),
            GamepadType::XboxController
        );
        
        manager.add_device(
            "DualShock 4".to_string(),
            GamepadType::PlayStation
        );
        
        log::info!("Gamepad subsystem initialized with {} controllers", 
            manager.get_devices().len());
    }
    
    // Store in global manager
    *GAMEPAD_MANAGER.lock() = manager.clone();
    
    Ok(manager)
}

/// Get the global gamepad manager
pub fn get_manager() -> &'static Mutex<GamepadManager> {
    &GAMEPAD_MANAGER
}

/// Poll for gamepad state updates
pub fn poll() {
    let mut manager = GAMEPAD_MANAGER.lock();
    
    // In a real driver, this would check hardware for updates
    // For now, we just update our virtual controllers with some test data
    
    #[cfg(feature = "std")]
    {
        if let Some(xbox) = manager.get_device_mut(0) {
            if xbox.is_connected() {
                // Simulate some input changes
                let mut state = xbox.get_state();
                
                // Simulate pressing the A button half the time
                // Use a static counter instead of relying on timer ticks
                static mut TICK_COUNTER: u64 = 0;
                let tick_value = unsafe {
                    TICK_COUNTER = TICK_COUNTER.wrapping_add(1);
                    TICK_COUNTER
                };
                
                if tick_value % 2 == 0 {
                    state.buttons |= BTN_A;
                } else {
                    state.buttons &= !BTN_A;
                }
                
                // Simulate stick movement
                // Using a simple counter instead of ticks from timer
                static mut COUNTER: u64 = 0;
                let t = unsafe {
                    COUNTER = COUNTER.wrapping_add(1);
                    (COUNTER as f32) / 10.0
                };
                state.left_stick_x = (t.sin() * 32767.0) as i16;
                state.left_stick_y = (t.cos() * 32767.0) as i16;
                
                xbox.update_state(state);
            }
        }
    }
}

// Add Clone implementation for GamepadManager
impl Clone for GamepadManager {
    fn clone(&self) -> Self {
        Self {
            devices: self.devices.clone(),
            next_id: self.next_id,
        }
    }
}

// Add Clone implementation for GamepadDevice
impl Clone for GamepadDevice {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            gamepad_type: self.gamepad_type,
            state: self.state,
            connected: AtomicBool::new(self.connected.load(Ordering::SeqCst)),
        }
    }
}