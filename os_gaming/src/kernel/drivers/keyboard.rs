use crate::kernel::interrupts;
use core::sync::atomic::{AtomicU8, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

#[cfg(feature = "std")]
use std::sync::mpsc::Sender;

// Keyboard port for data read
const KEYBOARD_PORT: u16 = 0x60;

// Scancode buffers to store last pressed key
lazy_static! {
    static ref KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
    #[cfg(feature = "std")]
    static ref KEY_EVENT_SENDER: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);
}

// Track if a key is pressed or released
static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);

pub struct KeyboardState {
    scancode: u8,
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
    num_lock: bool,
}

pub fn new() -> KeyboardState {
    KeyboardState {
        scancode: 0,
        shift_pressed: false,
        ctrl_pressed: false,
        alt_pressed: false,
        num_lock: false
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub character: char,
    pub scancode: u8,
    pub shift_pressed: bool,
    pub ctrl_pressed: bool,
    pub alt_pressed: bool,
    pub num_lock: bool,
}

impl KeyboardState {
    pub fn new() -> Self {
        KeyboardState {
            scancode: 0,
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
            num_lock: false,
        }
    }

    fn update_modifiers(&mut self, scancode: u8) {
        let released = scancode & 0x80 != 0;
        let key = scancode & 0x7F;

        match key {
            0x2A | 0x36 => self.shift_pressed = !released, // Left and right shift
            0x1D => self.ctrl_pressed = !released,         // Ctrl
            0x38 => self.alt_pressed = !released,          // Alt
            _ => {}
        }
    }
}

// Map scancodes to ASCII characters (US QWERTY layout)
fn map_scancode(scancode: u8, shift_pressed: bool, num_lock: bool) -> Option<char> {
    let released = scancode & 0x80 != 0;
    if released {
        return None;
    }

    let character = match scancode {
        0x01 => Some('\u{001B}'), // Escape
        0x02 => Some(if shift_pressed { '!' } else { '1' }),
        0x03 => Some(if shift_pressed { '@' } else { '2' }),
        0x04 => Some(if shift_pressed { '#' } else { '3' }),
        0x05 => Some(if shift_pressed { '$' } else { '4' }),
        0x06 => Some(if shift_pressed { '%' } else { '5' }),
        0x07 => Some(if shift_pressed { '^' } else { '6' }),
        0x08 => Some(if shift_pressed { '&' } else { '7' }),
        0x09 => Some(if shift_pressed { '*' } else { '8' }),
        0x0A => Some(if shift_pressed { '(' } else { '9' }),
        0x0B => Some(if shift_pressed { ')' } else { '0' }),
        0x0C => Some(if shift_pressed { '_' } else { '-' }),
        0x0D => Some(if shift_pressed { '+' } else { '=' }),
        0x0E => Some('\u{0008}'), // Backspace
        0x0F => Some('\t'),       // Tab
        0x10 => Some(if shift_pressed { 'Q' } else { 'q' }),
        0x11 => Some(if shift_pressed { 'W' } else { 'w' }),
        0x12 => Some(if shift_pressed { 'E' } else { 'e' }),
        0x13 => Some(if shift_pressed { 'R' } else { 'r' }),
        0x14 => Some(if shift_pressed { 'T' } else { 't' }),
        0x15 => Some(if shift_pressed { 'Y' } else { 'y' }),
        0x16 => Some(if shift_pressed { 'U' } else { 'u' }),
        0x17 => Some(if shift_pressed { 'I' } else { 'i' }),
        0x18 => Some(if shift_pressed { 'O' } else { 'o' }),
        0x19 => Some(if shift_pressed { 'P' } else { 'p' }),
        0x1A => Some(if shift_pressed { '{' } else { '[' }),
        0x1B => Some(if shift_pressed { '}' } else { ']' }),
        0x1C => Some('\n'),       // Enter
        0x1E => Some(if shift_pressed { 'A' } else { 'a' }),
        0x1F => Some(if shift_pressed { 'S' } else { 's' }),
        0x20 => Some(if shift_pressed { 'D' } else { 'd' }),
        0x21 => Some(if shift_pressed { 'F' } else { 'f' }),
        0x22 => Some(if shift_pressed { 'G' } else { 'g' }),
        0x23 => Some(if shift_pressed { 'H' } else { 'h' }),
        0x24 => Some(if shift_pressed { 'J' } else { 'j' }),
        0x25 => Some(if shift_pressed { 'K' } else { 'k' }),
        0x26 => Some(if shift_pressed { 'L' } else { 'l' }),
        0x27 => Some(if shift_pressed { ':' } else { ';' }),
        0x28 => Some(if shift_pressed { '"' } else { '\'' }),
        0x29 => Some(if shift_pressed { '~' } else { '`' }),
        0x2B => Some(if shift_pressed { '|' } else { '\\' }),
        0x2C => Some(if shift_pressed { 'Z' } else { 'z' }),
        0x2D => Some(if shift_pressed { 'X' } else { 'x' }),
        0x2E => Some(if shift_pressed { 'C' } else { 'c' }),
        0x2F => Some(if shift_pressed { 'V' } else { 'v' }),
        0x30 => Some(if shift_pressed { 'B' } else { 'b' }),
        0x31 => Some(if shift_pressed { 'N' } else { 'n' }),
        0x32 => Some(if shift_pressed { 'M' } else { 'm' }),
        0x33 => Some(if shift_pressed { '<' } else { ',' }),
        0x34 => Some(if shift_pressed { '>' } else { '.' }),
        0x35 => Some(if shift_pressed { '?' } else { '/' }),
        0x39 => Some(' '),        // Space

        // Numpad keys
        0x47 => Some(if num_lock { '7' } else { '7' }), // Numpad 7 (Home)
        0x48 => Some(if num_lock { '8' } else { '8' }), // Numpad 8 (Up)
        0x49 => Some(if num_lock { '9' } else { '9' }), // Numpad 9 (Page Up)
        0x4A => Some(if shift_pressed { '-' } else { '-' }), // Numpad Subtract
        0x4B => Some(if num_lock { '4' } else { '4' }), // Numpad 4 (Left)
        0x4C => Some(if num_lock { '5' } else { '5' }), // Numpad 5
        0x4D => Some(if num_lock { '6' } else { '6' }), // Numpad 6 (Right)
        0x4E => Some(if shift_pressed { '+' } else { '+' }), // Numpad Add
        0x4F => Some(if num_lock { '1' } else { '1' }), // Numpad 1 (End)
        0x50 => Some(if num_lock { '2' } else { '2' }), // Numpad 2 (Down)
        0x51 => Some(if num_lock { '3' } else { '3' }), // Numpad 3 (Page Down)
        0x52 => Some(if num_lock { '0' } else { '0' }), // Numpad 0 (Insert)
        0x53 => Some(if num_lock { '.' } else { '.' }), // Numpad Decimal

        // Additional special characters
        0x3A => Some(if shift_pressed { ':' } else { ';' }), // ; and :
        0x3B => Some(if shift_pressed { '+' } else { '=' }), // = and +
        0x3C => Some(if shift_pressed { '<' } else { ',' }), // , and <
        0x3D => Some(if shift_pressed { '-' } else { '-' }), // - and -
        0x3E => Some(if shift_pressed { '>' } else { '.' }), // . and >
        0x3F => Some(if shift_pressed { '?' } else { '/' }), // / and ?
        0x40 => Some(if shift_pressed { '@' } else { '\'' }), // ' and @
        0x41 => Some(if shift_pressed { '[' } else { '[' }), // [ and {
        0x42 => Some(if shift_pressed { ']' } else { ']' }), // ] and }
        0x43 => Some(if shift_pressed { '\\' } else { '\\' }), // \ and |

        _ => None,
    };

    character
}


// Keyboard interrupt handler
pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: interrupts::InterruptStackFrame) {
    
    let mut port = Port::new(KEYBOARD_PORT);
    let scancode: u8 = unsafe { port.read() };
    
    // Store the scancode
    LAST_SCANCODE.store(scancode, Ordering::SeqCst);
    
    // Update keyboard state
    let mut keyboard = KEYBOARD_STATE.lock();
    keyboard.scancode = scancode;
    keyboard.update_modifiers(scancode);
    
    // Convert scancode to character
    if let Some(key) = map_scancode(scancode, keyboard.shift_pressed, keyboard.num_lock) {
        let event = KeyEvent {
            character: key,
            scancode,
            shift_pressed: keyboard.shift_pressed,
            ctrl_pressed: keyboard.ctrl_pressed,
            alt_pressed: keyboard.alt_pressed,
            num_lock: keyboard.num_lock,
        };
        
        #[cfg(feature = "std")]
        {
            // Send key event to GUI if we're in std mode
            if let Some(sender) = &*KEY_EVENT_SENDER.lock() {
                let _ = sender.send(event);
            }
        }
        
        #[cfg(not(feature = "std"))]
        {
            // In kernel mode, maybe update a console directly
            if key == '\n' {
                crate::kernel::drivers::vga::new_line();
            } else if key == '\u{0008}' {  // Backspace
                crate::kernel::drivers::vga::backspace();
            } else {
                crate::kernel::drivers::vga::print_char(key);
            }
        }
    }
    
    // Send End-Of-Interrupt signal
    unsafe {
        interrupts::PICS.lock().notify_end_of_interrupt(interrupts::KEYBOARD_INTERRUPT_INDEX);
    }
}

// Initialize the keyboard
pub fn init() {
    // Register keyboard interrupt handler
    interrupts::set_irq_handler(interrupts::KEYBOARD_INTERRUPT_INDEX, keyboard_interrupt_handler);
    
    // Enable the keyboard IRQ in the PIC
    unsafe {
        interrupts::PICS.lock().unmask(interrupts::KEYBOARD_INTERRUPT_INDEX);
    }
}

// Get the last scancode
pub fn last_scancode() -> u8 {
    LAST_SCANCODE.load(Ordering::SeqCst)
}

// Check if a specific key is pressed (using scancode)
pub fn is_key_pressed(scancode: u8) -> bool {
    let state = KEYBOARD_STATE.lock();
    state.scancode == scancode && state.scancode & 0x80 == 0
}

// Get the current state of modifier keys
pub fn get_modifiers() -> (bool, bool, bool) {
    let state = KEYBOARD_STATE.lock();
    (state.shift_pressed, state.ctrl_pressed, state.alt_pressed)
}