use std::collections::HashSet;


/// Represents different input states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputState {
    Pressed,
    Released,
    Held,
}

/// Represents keyboard keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Space, Enter, Escape, Backspace, Tab,
    Shift, Ctrl, Alt,
    Up, Down, Left, Right,
    // Add more keys as needed
}

/// Represents mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

/// Manages input state for the GUI.
pub struct InputManager {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_position: (f32, f32),
    pressed_mouse_buttons: HashSet<MouseButton>,
    held_mouse_buttons: HashSet<MouseButton>,
    released_mouse_buttons: HashSet<MouseButton>,
}

impl InputManager {
    /// Creates a new InputManager.
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_position: (0.0, 0.0),
            pressed_mouse_buttons: HashSet::new(),
            held_mouse_buttons: HashSet::new(),
            released_mouse_buttons: HashSet::new(),
        }
    }

    /// Updates the input state for the next frame.
    pub fn update(&mut self) {
        // Move pressed keys to held keys
        for key in &self.pressed_keys {
            self.held_keys.insert(*key);
        }
        self.pressed_keys.clear();
        self.released_keys.clear();

        // Do the same for mouse buttons
        for button in &self.pressed_mouse_buttons {
            self.held_mouse_buttons.insert(*button);
        }
        self.pressed_mouse_buttons.clear();
        self.released_mouse_buttons.clear();
    }

    /// Processes a keyboard key press.
    pub fn process_key_press(&mut self, key: Key) {
        self.pressed_keys.insert(key);
    }

    /// Processes a keyboard key release.
    pub fn process_key_release(&mut self, key: Key) {
        if self.held_keys.remove(&key) || self.pressed_keys.remove(&key) {
            self.released_keys.insert(key);
        }
    }

    /// Processes mouse movement.
    pub fn process_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Processes a mouse button press.
    pub fn process_mouse_button_press(&mut self, button: MouseButton) {
        self.pressed_mouse_buttons.insert(button);
    }

    /// Processes a mouse button release.
    pub fn process_mouse_button_release(&mut self, button: MouseButton) {
        if self.held_mouse_buttons.remove(&button) || self.pressed_mouse_buttons.remove(&button) {
            self.released_mouse_buttons.insert(button);
        }
    }

    /// Checks if a key is currently pressed (only for the current frame).
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Checks if a key is currently held down.
    pub fn is_key_held(&self, key: Key) -> bool {
        self.held_keys.contains(&key)
    }

    /// Checks if a key was released this frame.
    pub fn is_key_released(&self, key: Key) -> bool {
        self.released_keys.contains(&key)
    }

    /// Gets the current mouse position.
    pub fn get_mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Checks if a mouse button is currently pressed (only for the current frame).
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    /// Checks if a mouse button is currently held down.
    pub fn is_mouse_button_held(&self, button: MouseButton) -> bool {
        self.held_mouse_buttons.contains(&button)
    }

    /// Checks if a mouse button was released this frame.
    pub fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.released_mouse_buttons.contains(&button)
    }
}