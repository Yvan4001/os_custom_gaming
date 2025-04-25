extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};
use core::arch::asm;
use spin::Mutex;
use lazy_static::lazy_static;
use micromath::F32Ext;

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
    static ref EVENT_BUFFER: Mutex<Vec<InputEvent>> = Mutex::new(Vec::with_capacity(64));
    
    // Store previous state of all gamepads to detect changes
    static ref PREVIOUS_STATES: Mutex<Vec<(usize, GamepadState)>> = Mutex::new(Vec::new());
    
    // Track if we've initialized our previous states
    static ref INITIALIZED: AtomicBool = AtomicBool::new(false);
}

#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    /// Device ID
    pub id: u8,
    /// Button state bit flags
    pub buttons: u32,
    /// Analog stick positions (-32768 to 32767)
    pub axes: [i16; 6],
    /// Trigger values (0-255)
    pub triggers: [u8; 2],
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Event type (button, axis, connection, etc)
    pub event_type: InputEventType,
}

/// Type of input event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEventType {
    /// Button pressed or released
    Button,
    /// Analog stick movement
    Axis, 
    /// Trigger movement
    Trigger,
    /// Device connected or disconnected
    Connection,
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

/// Get all pending gamepad events since the last call
pub fn get_pending_events() -> Result<Vec<InputEvent>, &'static str> {
    // Poll for new events
    poll_gamepads();
    
    // Get the event buffer
    if let Some(mut buffer) = EVENT_BUFFER.try_lock() {
        // Take ownership of events
        let events = core::mem::take(&mut *buffer);
        return Ok(events);
    } else {
        return Err("Failed to lock event buffer");
    }
}

/// Poll all gamepads for state changes and generate events
fn poll_gamepads() {
    // Get the manager
    let manager = GAMEPAD_MANAGER.lock();
    
    // Initialize previous states if needed
    if !INITIALIZED.load(Ordering::SeqCst) {
        let mut prev_states = PREVIOUS_STATES.lock();
        *prev_states = manager
            .get_devices()
            .iter()
            .filter(|d| d.is_connected())
            .map(|d| (d.get_id(), d.get_state()))
            .collect();
        
        INITIALIZED.store(true, Ordering::SeqCst);
    }
    
    // Get current timestamp
    let timestamp = get_timestamp_ms();
    
    // Check for device changes and generate events
    for device in manager.get_devices() {
        if !device.is_connected() {
            continue;
        }
        
        let current_state = device.get_state();
        let device_id = device.get_id();
        
        // Find previous state
        let mut prev_states = PREVIOUS_STATES.lock();
        let prev_state = prev_states
            .iter_mut()
            .find(|(id, _)| *id == device_id)
            .map(|(_, state)| state);
        
        if let Some(prev_state) = prev_state {
            // Check for button changes
            if current_state.buttons != prev_state.buttons {
                queue_button_event(device_id, 
                                  prev_state.buttons, 
                                  current_state.buttons, 
                                  timestamp);
            }
            
            // Check for left stick changes (with deadzone)
            if significant_stick_change(
                prev_state.left_stick_x, current_state.left_stick_x,
                prev_state.left_stick_y, current_state.left_stick_y) {
                
                queue_axis_event(device_id, 
                               0, // Left stick X
                               1, // Left stick Y
                               current_state.left_stick_x,
                               current_state.left_stick_y,
                               timestamp);
            }
            
            // Check for right stick changes (with deadzone)
            if significant_stick_change(
                prev_state.right_stick_x, current_state.right_stick_x,
                prev_state.right_stick_y, current_state.right_stick_y) {
                
                queue_axis_event(device_id, 
                               2, // Right stick X
                               3, // Right stick Y
                               current_state.right_stick_x,
                               current_state.right_stick_y,
                               timestamp);
            }
            
            // Check for trigger changes
            if significant_trigger_change(prev_state.left_trigger, current_state.left_trigger) {
                queue_trigger_event(device_id, 0, current_state.left_trigger, timestamp);
            }
            
            if significant_trigger_change(prev_state.right_trigger, current_state.right_trigger) {
                queue_trigger_event(device_id, 1, current_state.right_trigger, timestamp);
            }
            
            // Update previous state
            *prev_state = current_state;
        } else {
            // New device, add to tracking
            prev_states.push((device_id, current_state));
            
            // Generate connection event
            queue_connection_event(device_id, true, timestamp);
        }
    }
    
    // Check for disconnected devices
    let mut prev_states = PREVIOUS_STATES.lock();
    let mut i = 0;
    while i < prev_states.len() {
        let (id, _) = prev_states[i];
        let device_still_connected = manager
            .get_devices()
            .iter()
            .any(|d| d.get_id() == id && d.is_connected());
        
        if !device_still_connected {
            // Device disconnected
            queue_connection_event(id, false, timestamp);
            prev_states.remove(i);
        } else {
            i += 1;
        }
    }
}

/// Check if stick movement is significant (outside deadzone)
fn significant_stick_change(old_x: i16, new_x: i16, old_y: i16, new_y: i16) -> bool {
    const DEADZONE: i16 = 3000; // About 9% of full range
    
    // Calculate euclidean distance squared for efficiency
    let dx_old = old_x.abs();
    let dy_old = old_y.abs();
    let dx_new = new_x.abs();
    let dy_new = new_y.abs();
    
    // Movement crossed deadzone threshold
    if (dx_old < DEADZONE && dy_old < DEADZONE) && (dx_new >= DEADZONE || dy_new >= DEADZONE) {
        return true;
    }
    
    // Already outside deadzone, check for significant movement
    if dx_old >= DEADZONE || dy_old >= DEADZONE {
        // At least 5% change in position
        const MOVEMENT_THRESHOLD: i16 = 1600;
        let dx = (new_x - old_x).abs();
        let dy = (new_y - old_y).abs();
        
        return dx > MOVEMENT_THRESHOLD || dy > MOVEMENT_THRESHOLD;
    }
    
    false
}

/// Check if trigger movement is significant
fn significant_trigger_change(old_value: u8, new_value: u8) -> bool {
    const TRIGGER_THRESHOLD: u8 = 15; // About 6% of full range
    
    // Movement crossed threshold
    if (old_value < TRIGGER_THRESHOLD && new_value >= TRIGGER_THRESHOLD) ||
       (old_value >= TRIGGER_THRESHOLD && new_value < TRIGGER_THRESHOLD) {
        return true;
    }
    
    // Already beyond threshold, check for significant change
    if old_value >= TRIGGER_THRESHOLD || new_value >= TRIGGER_THRESHOLD {
        return (new_value as i16 - old_value as i16).abs() > 20;
    }
    
    false
}

/// Queue a button state change event
fn queue_button_event(device_id: usize, old_buttons: u32, new_buttons: u32, timestamp: u64) {
    // Create an event for any button change
    if old_buttons != new_buttons {
        // Create an axes array with stick positions from the device
        let manager = GAMEPAD_MANAGER.lock();
        let device = manager.get_device(device_id);
        
        let axes = if let Some(device) = device {
            let state = device.get_state();
            [
                state.left_stick_x, 
                state.left_stick_y,
                state.right_stick_x, 
                state.right_stick_y,
                0, // Extra axes if needed
                0,
            ]
        } else {
            [0; 6]
        };
        
        let triggers = if let Some(device) = device {
            let state = device.get_state();
            [state.left_trigger, state.right_trigger]
        } else {
            [0, 0]
        };
        
        let event = InputEvent {
            id: device_id as u8,
            buttons: new_buttons,
            axes,
            triggers,
            timestamp,
            event_type: InputEventType::Button,
        };
        
        let mut buffer = EVENT_BUFFER.lock();
        buffer.push(event);
        
        #[cfg(feature = "log")]
        {
            // Log button changes for debugging
            let pressed = new_buttons & !old_buttons;
            let released = old_buttons & !new_buttons;
            
            if pressed != 0 {
                log::trace!("Gamepad {}: Buttons pressed: {:#x}", device_id, pressed);
            }
            if released != 0 {
                log::trace!("Gamepad {}: Buttons released: {:#x}", device_id, released);
            }
        }
    }
}

/// Queue an axis movement event
fn queue_axis_event(device_id: usize, axis_x: usize, axis_y: usize, 
                  x_value: i16, y_value: i16, timestamp: u64) {
    // Get current device state
    let manager = GAMEPAD_MANAGER.lock();
    let device = manager.get_device(device_id);
    
    if let Some(device) = device {
        let state = device.get_state();
        
        // Create an axes array with positions
        let mut axes = [0; 6];
        axes[axis_x] = x_value;
        axes[axis_y] = y_value;
        
        // Copy other axes values
        if axis_x != 0 && axis_y != 1 {
            axes[0] = state.left_stick_x;
            axes[1] = state.left_stick_y;
        }
        
        if axis_x != 2 && axis_y != 3 {
            axes[2] = state.right_stick_x;
            axes[3] = state.right_stick_y;
        }
        
        let event = InputEvent {
            id: device_id as u8,
            buttons: state.buttons,
            axes,
            triggers: [state.left_trigger, state.right_trigger],
            timestamp,
            event_type: InputEventType::Axis,
        };
        
        let mut buffer = EVENT_BUFFER.lock();
        buffer.push(event);
        
        #[cfg(feature = "log")]
        {
            log::trace!("Gamepad {}: Axis {}/{} movement: {}/{}", 
                      device_id, axis_x, axis_y, x_value, y_value);
        }
    }
}

/// Queue a trigger movement event
fn queue_trigger_event(device_id: usize, trigger_id: usize, value: u8, timestamp: u64) {
    // Get current device state
    let manager = GAMEPAD_MANAGER.lock();
    let device = manager.get_device(device_id);
    
    if let Some(device) = device {
        let state = device.get_state();
        
        // Create trigger array
        let mut triggers = [0; 2];
        triggers[trigger_id] = value;
        
        // Copy other trigger value
        if trigger_id == 0 {
            triggers[1] = state.right_trigger;
        } else {
            triggers[0] = state.left_trigger;
        }
        
        let event = InputEvent {
            id: device_id as u8,
            buttons: state.buttons,
            axes: [
                state.left_stick_x, 
                state.left_stick_y,
                state.right_stick_x, 
                state.right_stick_y,
                0, 0
            ],
            triggers,
            timestamp,
            event_type: InputEventType::Trigger,
        };
        
        let mut buffer = EVENT_BUFFER.lock();
        buffer.push(event);
        
        #[cfg(feature = "log")]
        {
            log::trace!("Gamepad {}: Trigger {} value: {}", 
                      device_id, trigger_id, value);
        }
    }
}

/// Queue a connection/disconnection event
fn queue_connection_event(device_id: usize, connected: bool, timestamp: u64) {
    let event = InputEvent {
        id: device_id as u8,
        buttons: 0,
        axes: [0; 6],
        triggers: [0; 2],
        timestamp,
        event_type: InputEventType::Connection,
    };
    
    let mut buffer = EVENT_BUFFER.lock();
    buffer.push(event);
    
    #[cfg(feature = "log")]
    {
        if connected {
            log::info!("Gamepad {} connected", device_id);
        } else {
            log::info!("Gamepad {} disconnected", device_id);
        }
    }
}

/// Get current timestamp in milliseconds
fn get_timestamp_ms() -> u64 {
    #[cfg(feature = "std")]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    #[cfg(not(feature = "std"))]
    {
        // Use system timer or PIT/TSC for timestamp
        crate::kernel::drivers::timer::get_system_time_ms()
    }
}