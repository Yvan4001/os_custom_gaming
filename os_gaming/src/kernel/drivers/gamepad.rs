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

#[cfg(not(feature = "std"))]
const USB_IRQ: u8 = 43;      // USB controller interrupt

/// Constants for gamepad-specific interrupt sources
#[cfg(not(feature = "std"))]
const GAMEPAD_USB_ENDPOINT: u8 = 0x81;

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

/// Handle a gamepad interrupt
pub fn handle_interrupt() {
    #[cfg(feature = "std")]
    {
        // In std mode, just simulate by polling
        poll();
        return;
    }
    
    #[cfg(not(feature = "std"))]
    {
        // Get the gamepad manager instance
        let mut manager = GAMEPAD_MANAGER.lock();
        
        // First, determine which device triggered the interrupt
        // In a real implementation, this would check hardware controller registers
        
        // Track previous states to detect changes
        let mut previous_states: Vec<(usize, GamepadState)> = manager.devices
            .iter()
            .filter(|d| d.is_connected())
            .map(|d| (d.get_id(), d.get_state()))
            .collect();
        
        // Check for USB controller interrupts (common source for modern gamepads)
        if is_usb_interrupt_pending() {
            process_usb_gamepad_interrupts(&mut manager);
        }
        
        // Check for device-specific interrupts and update states
        for device in manager.devices.iter_mut().filter(|d| d.is_connected()) {
            let old_state = previous_states
                .iter()
                .find(|(id, _)| *id == device.get_id())
                .map(|(_, state)| *state);
            
            // Get new state after interrupt processing
            let new_state = device.get_state();
            
            // Compare states to detect changes if we have an old state
            if let Some(old_state) = old_state {
                // Check for button changes
                if old_state.buttons != new_state.buttons {
                    process_button_changes(device.get_id(), old_state.buttons, new_state.buttons);
                }
                
                // Check for stick movement
                if old_state.left_stick_x != new_state.left_stick_x || 
                   old_state.left_stick_y != new_state.left_stick_y {
                    process_stick_movement(
                        device.get_id(),
                        0, // Left stick ID
                        new_state.left_stick_x,
                        new_state.left_stick_y
                    );
                }
                
                if old_state.right_stick_x != new_state.right_stick_x || 
                   old_state.right_stick_y != new_state.right_stick_y {
                    process_stick_movement(
                        device.get_id(),
                        1, // Right stick ID
                        new_state.right_stick_x,
                        new_state.right_stick_y
                    );
                }
                
                // Check for trigger changes
                if old_state.left_trigger != new_state.left_trigger {
                    process_trigger_movement(device.get_id(), 0, new_state.left_trigger);
                }
                
                if old_state.right_trigger != new_state.right_trigger {
                    process_trigger_movement(device.get_id(), 1, new_state.right_trigger);
                }
            }
        }
        
        // Acknowledge the interrupt
        unsafe {
            crate::kernel::interrupts::irq::end_of_interrupt(USB_IRQ);
        }
    }
}
#[cfg(not(feature = "std"))]
fn is_usb_interrupt_pending() -> bool {
    // In a real implementation, check the USB controller's interrupt status register
    // This would typically involve reading hardware registers
    
    unsafe {
        // Example implementation for UHCI controller:
        const UHCI_USBSTS: u16 = 0x2;  // Status register offset
        let base_port = 0xC000;         // Example base I/O port
        let status = x86_64::instructions::port::Port::<u16>::new(base_port + UHCI_USBSTS).read();
        (status & 0x1) != 0;         // Check interrupt flag bit
        
        // Or for memory-mapped EHCI/XHCI controllers:
        // let mmio_base = 0xFEDC0000 as *const u32;  // Memory-mapped base
        // let status = core::ptr::read_volatile(mmio_base.add(0x4)); // Status register
        // (status & 0x1) != 0  // Check interrupt flag
        
        // For now, just assume there's an interrupt pending
        true
    }
}

#[cfg(not(feature = "std"))]
fn process_usb_gamepad_interrupts(manager: &mut GamepadManager) {
    // Process all connected USB gamepads
    for device in manager.devices.iter_mut().filter(|d| d.is_connected()) {
        match device.get_type() {
            GamepadType::XboxController => process_xbox_controller(device),
            GamepadType::PlayStation => process_playstation_controller(device),
            GamepadType::NintendoSwitch => process_switch_controller(device),
            _ => process_generic_controller(device),
        }
    }
}

#[cfg(not(feature = "std"))]
fn process_xbox_controller(device: &mut GamepadDevice) {
    // For Xbox controller, read USB endpoint data
    unsafe {
        // In a real implementation, we'd read from the USB HID endpoint
        // Example pseudocode:
        // let data = read_usb_endpoint(GAMEPAD_USB_ENDPOINT, device.get_id());
        
        // Xbox controllers use a specific report format we'd parse here
        // For now, just set some simulated data based on device ID
        let mut state = device.get_state();
        
        // Generate some test data based on time or counter
        // In a real implementation, this would come from the USB endpoint
        let counter = get_system_counter();
        if counter % 100 < 50 {
            state.buttons |= BTN_A;
        } else {
            state.buttons &= !BTN_A;
        }
        
        // Update thumbstick position
        let angle = (counter % 628) as f32 / 100.0; // 0 to 2π
        state.left_stick_x = (angle.sin() * 32767.0) as i16;
        state.left_stick_y = (angle.cos() * 32767.0) as i16;
        
        device.update_state(state);
    }
}

#[cfg(not(feature = "std"))]
fn process_playstation_controller(device: &mut GamepadDevice) {
    // Similar to Xbox but with PlayStation-specific report parsing
    // PlayStation controllers use different button mappings
    unsafe {
        let mut state = device.get_state();
        
        let counter = get_system_counter();
        if counter % 100 < 50 {
            state.buttons |= BTN_X; // Cross button on PlayStation
        } else {
            state.buttons &= !BTN_X;
        }
        
        // Right thumbstick for variety
        let angle = (counter % 628) as f32 / 100.0;
        state.right_stick_x = (angle.sin() * 32767.0) as i16;
        state.right_stick_y = (angle.cos() * 32767.0) as i16;
        
        device.update_state(state);
    }
}

#[cfg(not(feature = "std"))]
fn process_switch_controller(device: &mut GamepadDevice) {
    // Nintendo Switch Pro controller handling
    unsafe {
        let mut state = device.get_state();
        
        let counter = get_system_counter();
        if counter % 100 < 50 {
            state.buttons |= BTN_B; // B button
        } else {
            state.buttons &= !BTN_B;
        }
        
        device.update_state(state);
    }
}

#[cfg(not(feature = "std"))]
fn process_generic_controller(device: &mut GamepadDevice) {
    // Generic controller handling - uses standard HID reports
    unsafe {
        let mut state = device.get_state();
        
        let counter = get_system_counter();
        if counter % 100 < 50 {
            state.buttons |= BTN_START;
        } else {
            state.buttons &= !BTN_START;
        }
        
        device.update_state(state);
    }
}

#[cfg(not(feature = "std"))]
fn get_system_counter() -> u64 {
    // In a real implementation, this would come from the system's time counter
    // (e.g., TSC, PIT, or APIC timer)
    unsafe {
        // Read TSC if available
        let low: u32;
        let high: u32;
        
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack)
        );
        
        ((high as u64) << 32) | (low as u64)
    }
}

/// Process button state changes
#[cfg(not(feature = "std"))]
fn process_button_changes(device_id: usize, old_buttons: u32, new_buttons: u32) {
    // Determine which buttons were pressed and released
    let pressed = new_buttons & !old_buttons;
    let released = old_buttons & !new_buttons;
    
    // Process pressed buttons
    if pressed != 0 {
        for btn_mask in [BTN_A, BTN_B, BTN_X, BTN_Y, BTN_LEFT_SHOULDER, BTN_RIGHT_SHOULDER,
                        BTN_BACK, BTN_START, BTN_LEFT_THUMB, BTN_RIGHT_THUMB,
                        BTN_DPAD_UP, BTN_DPAD_DOWN, BTN_DPAD_LEFT, BTN_DPAD_RIGHT] {
            if pressed & btn_mask != 0 {
                // In a real implementation, you'd queue an event to your input system
                // For now, just print debug
                print_button_press(device_id, btn_mask);
            }
        }
    }
    
    // Process released buttons
    if released != 0 {
        for btn_mask in [BTN_A, BTN_B, BTN_X, BTN_Y, BTN_LEFT_SHOULDER, BTN_RIGHT_SHOULDER,
                        BTN_BACK, BTN_START, BTN_LEFT_THUMB, BTN_RIGHT_THUMB,
                        BTN_DPAD_UP, BTN_DPAD_DOWN, BTN_DPAD_LEFT, BTN_DPAD_RIGHT] {
            if released & btn_mask != 0 {
                // In a real implementation, you'd queue an event to your input system
            }
        }
    }
}

#[cfg(not(feature = "std"))]
fn print_button_press(device_id: usize, button: u32) {
    // Debug-only function to print button presses
    // In a real kernel, this might go to a debug console or serial port
    let name = match button {
        BTN_A => "A",
        BTN_B => "B",
        BTN_X => "X",
        BTN_Y => "Y",
        BTN_LEFT_SHOULDER => "LB",
        BTN_RIGHT_SHOULDER => "RB",
        BTN_BACK => "Back",
        BTN_START => "Start",
        BTN_LEFT_THUMB => "L3",
        BTN_RIGHT_THUMB => "R3",
        BTN_DPAD_UP => "Up",
        BTN_DPAD_DOWN => "Down",
        BTN_DPAD_LEFT => "Left",
        BTN_DPAD_RIGHT => "Right",
        _ => "Unknown"
    };
    
    // Print to debug output
    // Could be serial console in a real kernel
    unsafe {
        let msg = alloc::format!("Gamepad {}: Button {} pressed\n", device_id, name);
        for c in msg.bytes() {
            x86_64::instructions::port::Port::new(0x3F8).write(c);
        }
    }
}

/// Process analog stick movement
#[cfg(not(feature = "std"))]
fn process_stick_movement(device_id: usize, stick_id: u8, x: i16, y: i16) {
    // Handle analog stick movement
    // Apply deadzone
    let deadzone = 3200; // About 10% of full range
    
    if x.abs() < deadzone && y.abs() < deadzone {
        return; // Inside deadzone, ignore tiny movements
    }
    
    // In a real implementation, you would send this to your input system
    // For now, we just track significant movements for debugging
    if x.abs() > 16000 || y.abs() > 16000 {
        // Log major stick movements
        let stick_name = if stick_id == 0 { "Left" } else { "Right" };
        let direction = if x.abs() > y.abs() {
            if x > 0 { "Right" } else { "Left" }
        } else {
            if y > 0 { "Down" } else { "Up" }
        };
        
        unsafe {
            let msg = alloc::format!(
                "Gamepad {}: {} stick moved {}\n", 
                device_id, stick_name, direction
            );
            for c in msg.bytes() {
                x86_64::instructions::port::Port::new(0x3F8).write(c);
            }
        }
    }
}

/// Process trigger movement
#[cfg(not(feature = "std"))]
fn process_trigger_movement(device_id: usize, trigger_id: u8, value: u8) {
    // Only process significant trigger changes
    if value > 200 {
        let trigger_name = if trigger_id == 0 { "Left" } else { "Right" };
        
        unsafe {
            let msg = alloc::format!(
                "Gamepad {}: {} trigger pressed ({})\n", 
                device_id, trigger_name, value
            );
            for c in msg.bytes() {
                x86_64::instructions::port::Port::new(0x3F8).write(c);
            }
        }
    }
}