use crate::println;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};



lazy_static! {
    static ref MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());
}

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,  // Bit flags for button states
    pub scroll_wheel: i8,
    pub relative_x: i32,
    pub relative_y: i32,
}

pub struct Mouse {
    state: MouseState,
    data_port: PortReadOnly<u8>,
    command_port: PortWriteOnly<u8>,
    status_port: Port<u8>,
    cycle: u8,
    packet: [u8; 3],
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            buttons: 0,
            scroll_wheel: 0,
            relative_x: 0,
            relative_y: 0,
        }
    }
}

impl Clone for MouseState {
    fn clone(&self) -> Self {
        Self {
            x: self.x,
            y: self.y,
            buttons: self.buttons,
            scroll_wheel: self.scroll_wheel,
            relative_x: self.relative_x,
            relative_y: self.relative_y,
        }
    }
}


impl Mouse {
    fn new() -> Self {
        Self {
            state: MouseState::new(),
            data_port: PortReadOnly::new(0x60),
            command_port: PortWriteOnly::new(0x64),
            status_port: Port::new(0x64),
            cycle: 0,
            packet: [0; 3],
        }
    }

    fn write_command(&mut self, command: u8) {
        // Wait until we can send a command
        unsafe {
            while (self.status_port.read() & 2) != 0 {}
            self.command_port.write(0xD4);
            while (self.status_port.read() & 2) != 0 {}
            self.command_port.write(command);
        }
    }

    fn read_data(&mut self) -> u8 {
        unsafe { self.data_port.read() }
    }

    fn handle_packet(&mut self) {
        if self.packet[0] & 0x80 != 0 || self.packet[0] & 0x40 != 0 {
            // Overflow occurred, discard the packet
            return;
        }

        // Update the state
        self.state.buttons = self.packet[0] & 0x7;
        
        let x = self.packet[1] as i32;
        let y = self.packet[2] as i32;
        
        // Apply sign extension
        let x_sign = (self.packet[0] & 0x10) != 0;
        let y_sign = (self.packet[0] & 0x20) != 0;
        
        let x_movement = if x_sign { x - 256 } else { x };
        let y_movement = if y_sign { 256 - y } else { -y };
        
        self.state.x += x_movement;
        self.state.y += y_movement;
        
        // Ensure coordinates stay within bounds (example with 800x600 resolution)
        self.state.x = self.state.x.clamp(0, 799);
        self.state.y = self.state.y.clamp(0, 599);
        
        println!("Mouse: x={}, y={}, buttons={:03b}", 
                 self.state.x, self.state.y, self.state.buttons);
    }
}

pub fn init() {
    let mut mouse = MOUSE.lock();
    
    // Enable the auxiliary mouse device
    mouse.write_command(0xA8);
    
    // Enable interrupts
    mouse.write_command(0x20);
    let status = mouse.read_data();
    mouse.write_command(0x60);
    mouse.write_command(status | 2);
    
    // Set default settings
    mouse.write_command(0xF6);
    
    // Enable data reporting
    mouse.write_command(0xF4);
    
    println!("Mouse initialized");
}

pub fn handle_interrupt() {
    let mut mouse = MOUSE.lock();
    
    let status = unsafe { mouse.status_port.read() };
    if status & 0x20 == 0 {
        // Not a mouse interrupt
        return;
    }
    
    let data = mouse.read_data();
    match mouse.cycle {
        0 => {
            // First byte in packet must have bit 3 set
            if data & 0x08 != 0 {
                mouse.packet[0] = data;
                mouse.cycle = 1;
            }
        }
        1 => {
            mouse.packet[1] = data;
            mouse.cycle = 2;
        }
        2 => {
            mouse.packet[2] = data;
            mouse.cycle = 0;
            mouse.handle_packet();
        }
        _ => {
            mouse.cycle = 0;
        }
    }
}

pub fn get_state() -> MouseState {
    let mouse = MOUSE.lock();
    MouseState {
        x: mouse.state.x,
        y: mouse.state.y,
        buttons: mouse.state.buttons,
        scroll_wheel: mouse.state.scroll_wheel,
        relative_x: mouse.state.relative_x,
        relative_y: mouse.state.relative_y,
    }
}

pub fn get_pending_events() -> Result<Vec<MouseState>, &'static str> {
    // Static for tracking previous state to detect changes
    static mut PREV_STATE: Option<MouseState> = None;
    static mut EVENT_BUFFER: Option<Vec<MouseState>> = None;

    // Initialize event buffer if needed
    unsafe {
        if EVENT_BUFFER.is_none() {
            EVENT_BUFFER = Some(Vec::with_capacity(16));
        }
    }

    let mut mouse = MOUSE.lock();
    let mut events = Vec::new();

    // Get current state
    let current_state = MouseState {
        x: mouse.state.x,
        y: mouse.state.y,
        buttons: mouse.state.buttons,
        scroll_wheel: mouse.state.scroll_wheel,
        // Calculate relative movement since last poll
        relative_x: unsafe {
            if let Some(prev) = &PREV_STATE {
                mouse.state.x - prev.x
            } else {
                0
            }
        },
        relative_y: unsafe {
            if let Some(prev) = &PREV_STATE {
                mouse.state.y - prev.y
            } else {
                0
            }
        },
    };

    // Check if state has changed significantly
    let has_changes = unsafe {
        if let Some(prev) = &PREV_STATE {
            // Detect any relevant change
            current_state.x != prev.x ||
            current_state.y != prev.y ||
            current_state.buttons != prev.buttons ||
            current_state.scroll_wheel != prev.scroll_wheel
        } else {
            // First call, report initial state
            true
        }
    };

    // If there were changes, add to event list
    if has_changes {
        events.push(current_state.clone());
    }

    // Check if there are any events in the buffer
    unsafe {
        if let Some(ref mut buffer) = EVENT_BUFFER {
            // Add any buffered events
            events.append(buffer);
            buffer.clear();
        }
    }

    // Update previous state for next call
    unsafe {
        PREV_STATE = Some(current_state);
    }

    Ok(events)
}