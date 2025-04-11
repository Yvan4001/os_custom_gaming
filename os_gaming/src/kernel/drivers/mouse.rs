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
    pub buttons: u8,
}

struct Mouse {
    state: MouseState,
    data_port: PortReadOnly<u8>,
    command_port: PortWriteOnly<u8>,
    status_port: Port<u8>,
    cycle: u8,
    packet: [u8; 3],
}

impl Mouse {
    fn new() -> Self {
        Self {
            state: MouseState {
                x: 0,
                y: 0,
                buttons: 0,
            },
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
    }
}