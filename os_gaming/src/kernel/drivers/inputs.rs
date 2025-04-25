use crate::kernel::drivers::timer::{InputEvent, InputEventType, register_input_handler};
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use lazy_static::lazy_static;
use spin::Mutex;

/// Maximum number of keys that can be tracked simultaneously
const MAX_KEYS: usize = 256;
/// Maximum number of mouse buttons that can be tracked
const MAX_MOUSE_BUTTONS: usize = 8;
/// Maximum number of gamepads supported
const MAX_GAMEPADS: usize = 4;
/// Maximum number of buttons per gamepad
const MAX_GAMEPAD_BUTTONS: usize = 16;
/// Maximum number of axes per gamepad
const MAX_GAMEPAD_AXES: usize = 8;

/// Represents a key state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    /// Key is currently pressed
    Pressed,
    /// Key is currently released
    Released,
    /// Key was just pressed this frame
    JustPressed,
    /// Key was just released this frame
    JustReleased,
}

/// Represents a mouse state
#[derive(Clone, Debug)]
pub struct MouseState {
    /// Current X position
    pub x: i32,
    /// Current Y position
    pub y: i32,
    /// X movement since last frame
    pub dx: i32,
    /// Y movement since last frame
    pub dy: i32,
    /// Wheel scrolling (vertical)
    pub wheel: i32,
    /// Wheel scrolling (horizontal)
    pub wheel_h: i32,
    /// Button states
    pub buttons: [KeyState; MAX_MOUSE_BUTTONS],
}

/// Represents a gamepad axis state
#[derive(Clone, Copy, Debug)]
pub struct GamepadAxisState {
    /// Current value (-1.0 to 1.0)
    pub value: f32,
    /// Raw value from hardware
    pub raw_value: i32,
    /// Deadzone for this axis
    pub deadzone: f32,
}

/// Represents a gamepad state
#[derive(Clone, Debug)]
pub struct GamepadState {
    /// Is this gamepad connected
    pub connected: bool,
    /// Button states
    pub buttons: [KeyState; MAX_GAMEPAD_BUTTONS],
    /// Axis states
    pub axes: [GamepadAxisState; MAX_GAMEPAD_AXES],
    /// Gamepad name/identifier
    pub name: &'static str,
}

lazy_static! {
    /// Keyboard state
    static ref KEYBOARD_STATE: Mutex<[KeyState; MAX_KEYS]> = Mutex::new([KeyState::Released; MAX_KEYS]);
    
    /// Previous keyboard state (for detecting state changes)
    static ref PREV_KEYBOARD_STATE: Mutex<[KeyState; MAX_KEYS]> = Mutex::new([KeyState::Released; MAX_KEYS]);
    
    /// Mouse state
    static ref MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState {
        x: 0,
        y: 0,
        dx: 0,
        dy: 0,
        wheel: 0,
        wheel_h: 0,
        buttons: [KeyState::Released; MAX_MOUSE_BUTTONS],
    });
    
    /// Previous mouse state
    static ref PREV_MOUSE_STATE: Mutex<[KeyState; MAX_MOUSE_BUTTONS]> = Mutex::new([KeyState::Released; MAX_MOUSE_BUTTONS]);
    
    /// Gamepad states
    static ref GAMEPAD_STATES: Mutex<[GamepadState; MAX_GAMEPADS]> = Mutex::new([
        GamepadState {
            connected: false,
            buttons: [KeyState::Released; MAX_GAMEPAD_BUTTONS],
            axes: [GamepadAxisState { value: 0.0, raw_value: 0, deadzone: 0.1 }; MAX_GAMEPAD_AXES],
            name: "Gamepad 0",
        },
        GamepadState {
            connected: false,
            buttons: [KeyState::Released; MAX_GAMEPAD_BUTTONS],
            axes: [GamepadAxisState { value: 0.0, raw_value: 0, deadzone: 0.1 }; MAX_GAMEPAD_AXES],
            name: "Gamepad 1",
        },
        GamepadState {
            connected: false,
            buttons: [KeyState::Released; MAX_GAMEPAD_BUTTONS],
            axes: [GamepadAxisState { value: 0.0, raw_value: 0, deadzone: 0.1 }; MAX_GAMEPAD_AXES],
            name: "Gamepad 2",
        },
        GamepadState {
            connected: false,
            buttons: [KeyState::Released; MAX_GAMEPAD_BUTTONS],
            axes: [GamepadAxisState { value: 0.0, raw_value: 0, deadzone: 0.1 }; MAX_GAMEPAD_AXES],
            name: "Gamepad 3",
        },
    ]);
    
    /// Previous gamepad button states
    static ref PREV_GAMEPAD_BUTTONS: Mutex<[[KeyState; MAX_GAMEPAD_BUTTONS]; MAX_GAMEPADS]> = 
        Mutex::new([[KeyState::Released; MAX_GAMEPAD_BUTTONS]; MAX_GAMEPADS]);
    
    /// Input event handlers (callbacks)
    static ref INPUT_CALLBACKS: Mutex<Vec<fn(InputEvent)>> = Mutex::new(Vec::new());
    static ref INPUT_EVENT_QUEUE: Mutex<VecDeque<InputEvent>> = Mutex::new(VecDeque::new());
}

/// Initialize the input system
pub fn initialize() {
    // Register our input handler with the timer system
    register_input_handler(process_input_event);
}

/// Main input event processor
fn process_input_event(event: InputEvent) {
    match event.event_type {
        InputEventType::KeyPress => {
            let key_code = event.data[0] as usize;
            if key_code < MAX_KEYS {
                KEYBOARD_STATE.lock()[key_code] = KeyState::Pressed;
            }
        },

        InputEventType::KeyRelease => {
            let key_code = event.data[0] as usize;
            if key_code < MAX_KEYS {
                KEYBOARD_STATE.lock()[key_code] = KeyState::Released;
            }
        },

        InputEventType::MouseMove => {
            let mut mouse = MOUSE_STATE.lock();
            let new_x = event.data[0] as i32;
            let new_y = event.data[1] as i32;

            // Calculate delta movement
            mouse.dx = new_x - mouse.x;
            mouse.dy = new_y - mouse.y;

            // Update position
            mouse.x = new_x;
            mouse.y = new_y;
        },

        InputEventType::MouseButton => {
            let button = event.data[0] as usize;
            let pressed = event.data[1] != 0;

            if button < MAX_MOUSE_BUTTONS {
                let state = if pressed { KeyState::Pressed } else { KeyState::Released };
                MOUSE_STATE.lock().buttons[button] = state;
            }
        },

        InputEventType::GamepadButton => {
            let gamepad_id = event.data[0] as usize;
            let button = event.data[1] as usize;
            let pressed = event.data[2] != 0;

            if gamepad_id < MAX_GAMEPADS && button < MAX_GAMEPAD_BUTTONS {
                let mut gamepads = GAMEPAD_STATES.lock();
                let state = if pressed { KeyState::Pressed } else { KeyState::Released };
                gamepads[gamepad_id].buttons[button] = state;

                // Mark gamepad as connected when we receive input from it
                gamepads[gamepad_id].connected = true;
            }
        },

        InputEventType::GamepadAxis => {
            let gamepad_id = event.data[0] as usize;
            let axis = event.data[1] as usize;
            let raw_value = event.data[2] as i32;

            if gamepad_id < MAX_GAMEPADS && axis < MAX_GAMEPAD_AXES {
                let mut gamepads = GAMEPAD_STATES.lock();

                // Convert to normalized float value (-1.0 to 1.0)
                let normalized = (raw_value as f32) / 32767.0;
                let deadzone = gamepads[gamepad_id].axes[axis].deadzone;

                // Apply deadzone
                let value = if normalized.abs() < deadzone {
                    0.0
                } else {
                    // Rescale the value to account for deadzone
                    let sign = if normalized < 0.0 { -1.0 } else { 1.0 };
                    sign * (normalized.abs() - deadzone) / (1.0 - deadzone)
                };

                // Update axis state
                gamepads[gamepad_id].axes[axis].raw_value = raw_value;
                gamepads[gamepad_id].axes[axis].value = value;

                // Mark gamepad as connected when we receive input from it
                gamepads[gamepad_id].connected = true;
            }
        },

        // Handle other event types if needed
        _ => {}
    }

    // Forward event to registered callbacks
    if let Some(callbacks) = INPUT_CALLBACKS.try_lock() {
        for callback in callbacks.iter() {
            callback(event);
        }
    }
}

/// Update input states - should be called once per frame
pub fn update() {
    // Update keyboard states (detect "just pressed" and "just released" states)
    {
        let mut keyboard = KEYBOARD_STATE.lock();
        let mut prev_keyboard = PREV_KEYBOARD_STATE.lock();

        for i in 0..MAX_KEYS {
            match (keyboard[i], prev_keyboard[i]) {
                (KeyState::Pressed, KeyState::Released) |
                (KeyState::Pressed, KeyState::JustReleased) => {
                    keyboard[i] = KeyState::JustPressed;
                },
                (KeyState::Released, KeyState::Pressed) |
                (KeyState::Released, KeyState::JustPressed) => {
                    keyboard[i] = KeyState::JustReleased;
                },
                _ => {}
            }

            // Store current state for next frame
            prev_keyboard[i] = keyboard[i];

            // Reset "just" states for next frame
            if keyboard[i] == KeyState::JustPressed {
                keyboard[i] = KeyState::Pressed;
            } else if keyboard[i] == KeyState::JustReleased {
                keyboard[i] = KeyState::Released;
            }
        }
    }

    // Update mouse button states
    {
        let mut mouse = MOUSE_STATE.lock();
        let mut prev_mouse = PREV_MOUSE_STATE.lock();

        for i in 0..MAX_MOUSE_BUTTONS {
            match (mouse.buttons[i], prev_mouse[i]) {
                (KeyState::Pressed, KeyState::Released) |
                (KeyState::Pressed, KeyState::JustReleased) => {
                    mouse.buttons[i] = KeyState::JustPressed;
                },
                (KeyState::Released, KeyState::Pressed) |
                (KeyState::Released, KeyState::JustPressed) => {
                    mouse.buttons[i] = KeyState::JustReleased;
                },
                _ => {}
            }

            // Store current state for next frame
            prev_mouse[i] = mouse.buttons[i];

            // Reset "just" states for next frame
            if mouse.buttons[i] == KeyState::JustPressed {
                mouse.buttons[i] = KeyState::Pressed;
            } else if mouse.buttons[i] == KeyState::JustReleased {
                mouse.buttons[i] = KeyState::Released;
            }
        }

        // Reset delta values for next frame
        mouse.dx = 0;
        mouse.dy = 0;
        mouse.wheel = 0;
        mouse.wheel_h = 0;
    }

    // Update gamepad button states
    {
        let mut gamepads = GAMEPAD_STATES.lock();
        let mut prev_buttons = PREV_GAMEPAD_BUTTONS.lock();

        for gamepad_id in 0..MAX_GAMEPADS {
            if !gamepads[gamepad_id].connected {
                continue;
            }

            for button in 0..MAX_GAMEPAD_BUTTONS {
                match (gamepads[gamepad_id].buttons[button], prev_buttons[gamepad_id][button]) {
                    (KeyState::Pressed, KeyState::Released) |
                    (KeyState::Pressed, KeyState::JustReleased) => {
                        gamepads[gamepad_id].buttons[button] = KeyState::JustPressed;
                    },
                    (KeyState::Released, KeyState::Pressed) |
                    (KeyState::Released, KeyState::JustPressed) => {
                        gamepads[gamepad_id].buttons[button] = KeyState::JustReleased;
                    },
                    _ => {}
                }

                // Store current state for next frame
                prev_buttons[gamepad_id][button] = gamepads[gamepad_id].buttons[button];

                // Reset "just" states for next frame
                if gamepads[gamepad_id].buttons[button] == KeyState::JustPressed {
                    gamepads[gamepad_id].buttons[button] = KeyState::Pressed;
                } else if gamepads[gamepad_id].buttons[button] == KeyState::JustReleased {
                    gamepads[gamepad_id].buttons[button] = KeyState::Released;
                }
            }
        }
    }
}

/// Check if a key is currently pressed
pub fn is_key_down(key_code: usize) -> bool {
    if key_code >= MAX_KEYS {
        return false;
    }

    let state = KEYBOARD_STATE.lock()[key_code];
    state == KeyState::Pressed || state == KeyState::JustPressed
}

/// Check if a key was just pressed this frame
pub fn is_key_just_pressed(key_code: usize) -> bool {
    if key_code >= MAX_KEYS {
        return false;
    }

    KEYBOARD_STATE.lock()[key_code] == KeyState::JustPressed
}

/// Check if a key was just released this frame
pub fn is_key_just_released(key_code: usize) -> bool {
    if key_code >= MAX_KEYS {
        return false;
    }

    KEYBOARD_STATE.lock()[key_code] == KeyState::JustReleased
}

/// Check if a mouse button is currently pressed
pub fn is_mouse_button_down(button: usize) -> bool {
    if button >= MAX_MOUSE_BUTTONS {
        return false;
    }

    let state = MOUSE_STATE.lock().buttons[button];
    state == KeyState::Pressed || state == KeyState::JustPressed
}

/// Check if a mouse button was just pressed this frame
pub fn is_mouse_button_just_pressed(button: usize) -> bool {
    if button >= MAX_MOUSE_BUTTONS {
        return false;
    }

    MOUSE_STATE.lock().buttons[button] == KeyState::JustPressed
}

/// Get mouse position
pub fn get_mouse_position() -> (i32, i32) {
    let mouse = MOUSE_STATE.lock();
    (mouse.x, mouse.y)
}

/// Get mouse movement since last frame
pub fn get_mouse_delta() -> (i32, i32) {
    let mouse = MOUSE_STATE.lock();
    (mouse.dx, mouse.dy)
}

/// Check if a gamepad is connected
pub fn is_gamepad_connected(gamepad_id: usize) -> bool {
    if gamepad_id >= MAX_GAMEPADS {
        return false;
    }

    GAMEPAD_STATES.lock()[gamepad_id].connected
}

/// Check if a gamepad button is currently pressed
pub fn is_gamepad_button_down(gamepad_id: usize, button: usize) -> bool {
    if gamepad_id >= MAX_GAMEPADS || button >= MAX_GAMEPAD_BUTTONS {
        return false;
    }

    let state = GAMEPAD_STATES.lock()[gamepad_id].buttons[button];
    state == KeyState::Pressed || state == KeyState::JustPressed
}

/// Get gamepad axis value (-1.0 to 1.0)
pub fn get_gamepad_axis(gamepad_id: usize, axis: usize) -> f32 {
    if gamepad_id >= MAX_GAMEPADS || axis >= MAX_GAMEPAD_AXES {
        return 0.0;
    }

    GAMEPAD_STATES.lock()[gamepad_id].axes[axis].value
}

/// Set gamepad axis deadzone
pub fn set_gamepad_axis_deadzone(gamepad_id: usize, axis: usize, deadzone: f32) {
    if gamepad_id >= MAX_GAMEPADS || axis >= MAX_GAMEPAD_AXES {
        return;
    }

    GAMEPAD_STATES.lock()[gamepad_id].axes[axis].deadzone = deadzone.clamp(0.0, 1.0);
}

/// Register a callback for raw input events
pub fn register_input_callback(callback: fn(InputEvent)) {
    if let Some(mut callbacks) = INPUT_CALLBACKS.try_lock() {
        callbacks.push(callback);
    }
}

/// Common keyboard key codes
pub mod keys {
    pub const KEY_SPACE: usize = 32;
    pub const KEY_APOSTROPHE: usize = 39;
    pub const KEY_COMMA: usize = 44;
    pub const KEY_MINUS: usize = 45;
    pub const KEY_PERIOD: usize = 46;
    pub const KEY_SLASH: usize = 47;

    pub const KEY_0: usize = 48;
    pub const KEY_1: usize = 49;
    pub const KEY_2: usize = 50;
    pub const KEY_3: usize = 51;
    pub const KEY_4: usize = 52;
    pub const KEY_5: usize = 53;
    pub const KEY_6: usize = 54;
    pub const KEY_7: usize = 55;
    pub const KEY_8: usize = 56;
    pub const KEY_9: usize = 57;

    pub const KEY_SEMICOLON: usize = 59;
    pub const KEY_EQUAL: usize = 61;

    pub const KEY_A: usize = 65;
    pub const KEY_B: usize = 66;
    pub const KEY_C: usize = 67;
    pub const KEY_D: usize = 68;
    pub const KEY_E: usize = 69;
    pub const KEY_F: usize = 70;
    pub const KEY_G: usize = 71;
    pub const KEY_H: usize = 72;
    pub const KEY_I: usize = 73;
    pub const KEY_J: usize = 74;
    pub const KEY_K: usize = 75;
    pub const KEY_L: usize = 76;
    pub const KEY_M: usize = 77;
    pub const KEY_N: usize = 78;
    pub const KEY_O: usize = 79;
    pub const KEY_P: usize = 80;
    pub const KEY_Q: usize = 81;
    pub const KEY_R: usize = 82;
    pub const KEY_S: usize = 83;
    pub const KEY_T: usize = 84;
    pub const KEY_U: usize = 85;
    pub const KEY_V: usize = 86;
    pub const KEY_W: usize = 87;
    pub const KEY_X: usize = 88;
    pub const KEY_Y: usize = 89;
    pub const KEY_Z: usize = 90;

    pub const KEY_ESCAPE: usize = 256;
    pub const KEY_ENTER: usize = 257;
    pub const KEY_TAB: usize = 258;
    pub const KEY_BACKSPACE: usize = 259;
    pub const KEY_INSERT: usize = 260;
    pub const KEY_DELETE: usize = 261;
    pub const KEY_RIGHT: usize = 262;
    pub const KEY_LEFT: usize = 263;
    pub const KEY_DOWN: usize = 264;
    pub const KEY_UP: usize = 265;
    pub const KEY_PAGE_UP: usize = 266;
    pub const KEY_PAGE_DOWN: usize = 267;
    pub const KEY_HOME: usize = 268;
    pub const KEY_END: usize = 269;

    pub const KEY_F1: usize = 290;
    pub const KEY_F2: usize = 291;
    pub const KEY_F3: usize = 292;
    pub const KEY_F4: usize = 293;
    pub const KEY_F5: usize = 294;
    pub const KEY_F6: usize = 295;
    pub const KEY_F7: usize = 296;
    pub const KEY_F8: usize = 297;
    pub const KEY_F9: usize = 298;
    pub const KEY_F10: usize = 299;
    pub const KEY_F11: usize = 300;
    pub const KEY_F12: usize = 301;

    pub const KEY_LEFT_SHIFT: usize = 340;
    pub const KEY_LEFT_CONTROL: usize = 341;
    pub const KEY_LEFT_ALT: usize = 342;
    pub const KEY_RIGHT_SHIFT: usize = 344;
    pub const KEY_RIGHT_CONTROL: usize = 345;
    pub const KEY_RIGHT_ALT: usize = 346;
}

/// Common mouse button constants
pub mod mouse {
    pub const BUTTON_LEFT: usize = 0;
    pub const BUTTON_RIGHT: usize = 1;
    pub const BUTTON_MIDDLE: usize = 2;
}

/// Common gamepad constants
pub mod gamepad {
    // Common gamepad button mappings
    pub const BUTTON_A: usize = 0;
    pub const BUTTON_B: usize = 1;
    pub const BUTTON_X: usize = 2;
    pub const BUTTON_Y: usize = 3;
    pub const BUTTON_LEFT_BUMPER: usize = 4;
    pub const BUTTON_RIGHT_BUMPER: usize = 5;
    pub const BUTTON_BACK: usize = 6;
    pub const BUTTON_START: usize = 7;
    pub const BUTTON_GUIDE: usize = 8;
    pub const BUTTON_LEFT_THUMB: usize = 9;
    pub const BUTTON_RIGHT_THUMB: usize = 10;
    pub const BUTTON_DPAD_UP: usize = 11;
    pub const BUTTON_DPAD_RIGHT: usize = 12;
    pub const BUTTON_DPAD_DOWN: usize = 13;
    pub const BUTTON_DPAD_LEFT: usize = 14;

    // Common gamepad axis mappings
    pub const AXIS_LEFT_X: usize = 0;
    pub const AXIS_LEFT_Y: usize = 1;
    pub const AXIS_RIGHT_X: usize = 2;
    pub const AXIS_RIGHT_Y: usize = 3;
    pub const AXIS_LEFT_TRIGGER: usize = 4;
    pub const AXIS_RIGHT_TRIGGER: usize = 5;
}
pub fn queue_input_event(event: InputEvent) {
    if let Some(mut queue) = INPUT_EVENT_QUEUE.try_lock() {
        queue.push_back(event);
    }
}
pub fn poll_events() -> Option<InputEvent> {
    if let Some(mut queue) = INPUT_EVENT_QUEUE.try_lock() {
        queue.pop_front()
    } else {
        None
    }
}

pub fn clear_input_queue() {
    if let Some(mut queue) = INPUT_EVENT_QUEUE.try_lock() {
        queue.clear();
    }
}

pub fn create_key_press_event(key_code: usize) -> InputEvent {
    InputEvent {
        event_type: InputEventType::KeyPress,
        device_id: 0,
        data: [key_code as u32, 0, 0, 0],
    }
}

pub fn create_key_release_event(key_code: usize) -> InputEvent {
    InputEvent {
        event_type: InputEventType::KeyRelease,
        device_id: 0,
        data: [key_code as u32, 0, 0, 0],
    }
}

pub fn create_mouse_move_event(x: i32, y: i32) -> InputEvent {
    InputEvent {
        event_type: InputEventType::MouseMove,
        device_id: 0,
        data: [x as u32, y as u32, 0, 0],
    }
}

pub fn create_mouse_button_event(button: usize, pressed: bool) -> InputEvent {
    InputEvent {
        event_type: InputEventType::MouseButton,
        device_id: 0,
        data: [button as u32, if pressed { 1 } else { 0 }, 0, 0],
    }
}

pub fn create_gamepad_button_event(gamepad_id: usize, button: usize, pressed: bool) -> InputEvent {
    InputEvent {
        event_type: InputEventType::GamepadButton,
        device_id: gamepad_id as u8,
        data: [gamepad_id as u32, button as u32, if pressed { 1 } else { 0 }, 0],
    }
}

pub fn create_gamepad_axis_event(gamepad_id: usize, axis: usize, value: i32) -> InputEvent {
    InputEvent {
        event_type: InputEventType::GamepadAxis,
        device_id: gamepad_id as u8,
        data: [gamepad_id as u32, axis as u32, value as u32, 0],
    }
}