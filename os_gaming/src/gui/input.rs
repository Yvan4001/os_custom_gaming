use alloc::collections::VecDeque;
use alloc::vec::Vec;
use hashbrown::HashSet;

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
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    Space,
    Enter,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    Number0,
    Number1,
    Number2,
    Number3,
    Number4,
    Number5,
    Number6,
    Number7,
    Number8,
    Number9,
}

/// Represents mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
    MouseMove(f32, f32),
    MousePress(MouseButton),
    MouseRelease(MouseButton),
    MouseScroll(i32),
    WindowResize(u32, u32),
    WindowClose,
    WindowFocus,
    WindowBlur,
    Quit
}

/// Manages input state for the GUI.
pub struct InputManager {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_position: (f32, f32),
    last_mouse_position: (f32, f32),
    pressed_mouse_buttons: HashSet<MouseButton>,
    held_mouse_buttons: HashSet<MouseButton>,
    released_mouse_buttons: HashSet<MouseButton>,
    event_queue: VecDeque<Event>,
    static_instance: Option<&'static mut InputManager>,
}

impl InputManager {
    /// Creates a new InputManager.
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_position: (0.0, 0.0),
            last_mouse_position: (0.0, 0.0),
            pressed_mouse_buttons: HashSet::new(),
            held_mouse_buttons: HashSet::new(),
            released_mouse_buttons: HashSet::new(),
            event_queue: VecDeque::new(),
            static_instance: None,
        }
    }

    pub fn get_instance() -> Option<&'static mut InputManager> {
        // In a real implementation, this would use proper singleton management
        // For demonstration purposes only
        static mut INSTANCE: Option<InputManager> = None;

        unsafe {
            if INSTANCE.is_none() {
                INSTANCE = Some(InputManager::new());
            }
            INSTANCE.as_mut()
        }
    }

    /// Updates the input state for the next frame.
    pub fn update(&mut self) {
        // Store previous mouse position to detect movement
        self.last_mouse_position = self.mouse_position;

        // Check for hardware events (in a real system, this would poll hardware)
        self.poll_hardware_events();

        // Process key states
        // Move pressed keys to held keys
        let mut pressed_keys_copy = self.pressed_keys.clone();
        for key in pressed_keys_copy.drain() {
            self.event_queue.push_back(Event::KeyPress(key));
            self.held_keys.insert(key);
        }
        self.pressed_keys.clear();

        // Process released keys
        let mut released_keys_copy = self.released_keys.clone();
        for key in released_keys_copy.drain() {
            self.event_queue.push_back(Event::KeyRelease(key));
        }
        self.released_keys.clear();

        // Process mouse buttons
        let mut pressed_buttons_copy = self.pressed_mouse_buttons.clone();
        for button in pressed_buttons_copy.drain() {
            self.event_queue.push_back(Event::MousePress(button));
            self.held_mouse_buttons.insert(button);
        }
        self.pressed_mouse_buttons.clear();

        // Process released mouse buttons
        let mut released_buttons_copy = self.released_mouse_buttons.clone();
        for button in released_buttons_copy.drain() {
            self.event_queue.push_back(Event::MouseRelease(button));
        }
        self.released_mouse_buttons.clear();

        // Check if mouse has moved
        if self.mouse_position != self.last_mouse_position {
            self.event_queue.push_back(Event::MouseMove(
                self.mouse_position.0,
                self.mouse_position.1,
            ));
        }
    }
    fn poll_hardware_events(&mut self) {
        // For now, we'll simulate with stub code that would be replaced
        if let Some(raw_events) = self.read_hardware_input_buffer() {
            for event in raw_events {
                match event {
                    Event::KeyPress(scancode) => {
                        self.process_key_press(scancode);
                    }
                    Event::KeyRelease(scancode) => {
                        self.process_key_release(scancode);
                    }
                    Event::MouseMove(x, y) => {
                        self.process_mouse_move(x, y);
                    }
                    Event::MousePress(button_id) => {
                        self.process_mouse_button_press(button_id);
                    }
                    Event::MouseRelease(button_id) => {
                        self.process_mouse_button_release(button_id);
                    }
                    Event::MouseScroll(delta) => {
                        self.process_mouse_scroll(delta);
                    }
                    Event::WindowResize(width, height) => {
                        self.process_window_resize(width, height);
                    }
                    Event::WindowClose => {
                        self.process_window_close();
                    }
                    Event::WindowFocus => {
                        self.process_window_focus();
                    }
                    Event::WindowBlur => {
                        self.process_window_blur();
                    }
                    Event::Quit => {
                        // Handle quit event
                        self.event_queue.push_back(Event::Quit);
                    }
                }
            }
        }
    }
    /// Processes a keyboard key press.
    pub fn process_key_press(&mut self, key: Key) {
        self.pressed_keys.insert(key);
    }

    fn read_hardware_input_buffer(&self) -> Option<Vec<Event>> {
        // In real implementation: read from hardware/driver
        None // No events for now
    }
    fn scancode_to_key(&self, scancode: u8) -> Option<Key> {
        // Map hardware scancode to Key enum
        match scancode {
            0x01 => Some(Key::Escape),
            0x1E => Some(Key::A),
            0x30 => Some(Key::B),
            0x2E => Some(Key::C),
            0x20 => Some(Key::D),
            0x12 => Some(Key::E),
            0x21 => Some(Key::F),
            0x22 => Some(Key::G),
            0x23 => Some(Key::H),
            0x17 => Some(Key::I),
            0x24 => Some(Key::J),
            0x25 => Some(Key::K),
            0x26 => Some(Key::L),
            0x32 => Some(Key::M),
            0x31 => Some(Key::N),
            0x18 => Some(Key::O),
            0x19 => Some(Key::P),
            0x10 => Some(Key::Q),
            0x13 => Some(Key::R),
            0x1F => Some(Key::S),
            0x14 => Some(Key::T),
            0x16 => Some(Key::U),
            0x2F => Some(Key::V),
            0x11 => Some(Key::W),
            0x2D => Some(Key::X),
            0x15 => Some(Key::Y),
            0x2C => Some(Key::Z),
            0x39 => Some(Key::Space),
            0x1C => Some(Key::Enter),
            0x0E => Some(Key::Backspace),
            0x50 => Some(Key::Down), // Arrow Down and KeypadDown share the same scancode
            0x4B => Some(Key::Left), // Arrow Left and KeypadLeft share the same scancode
            0x4D => Some(Key::Right), // Arrow Right and KeypadRight share the same scancode
            0x52 => Some(Key::Number0),
            0x4F => Some(Key::Number1),
            // 0x50 => Some(Key::Number2), // Handled in the Down key case
            0x51 => Some(Key::Number3),
            // 0x4B => Some(Key::Number4), // Handled in the Left key case
            0x4C => Some(Key::Number5),
            // 0x4D => Some(Key::Number6), // Handled in the Right key case
            0x47 => Some(Key::Number7),
            0x48 => Some(Key::Number8), // Changed from 0x49 to 0x48
            0x4A => Some(Key::Number9),
            0x4C => Some(Key::Number5),
            0x4D => Some(Key::Number6),
            0x47 => Some(Key::Number7),
            0x49 => Some(Key::Number8),
            0x4A => Some(Key::Number9),
            _ => None,
        }
    }

    /// Processes a keyboard key release.
    pub fn process_key_release(&mut self, key: Key) {
        if self.held_keys.remove(&key) || self.pressed_keys.remove(&key) {
            self.released_keys.insert(key);
        }
    }

    pub fn process_mouse_scroll(&mut self, delta: i32) {
        // Handle mouse scroll event
        self.event_queue.push_back(Event::MouseScroll(delta));
    }
    pub fn process_window_resize(&mut self, width: u32, height: u32) {
        // Handle window resize event
        self.event_queue.push_back(Event::WindowResize(width, height));
    }
    pub fn process_window_close(&mut self) {
        // Handle window close event
        self.event_queue.push_back(Event::WindowClose);
    }
    pub fn process_window_focus(&mut self) {
        // Handle window focus event
        self.event_queue.push_back(Event::WindowFocus);
    }
    pub fn process_window_blur(&mut self) {
        // Handle window blur event
        self.event_queue.push_back(Event::WindowBlur);
    }

    fn button_id_to_mouse_button(&self, button_id: u8) -> Option<MouseButton> {
        match button_id {
            0 => Some(MouseButton::Left),
            1 => Some(MouseButton::Right),
            2 => Some(MouseButton::Middle),
            _ => None,
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

    pub fn next_event(&mut self) -> Option<Event> {
        self.event_queue.pop_front()
    }

    /// Check if there are any events in the queue
    pub fn has_events(&self) -> bool {
        !self.event_queue.is_empty()
    }

    /// Add a custom event to the queue
    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push_back(event);
    }

    /// Clear the event queue
    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }
}
