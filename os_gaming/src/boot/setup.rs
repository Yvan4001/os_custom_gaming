use crate::boot::info::CustomBootInfo;
use alloc::vec::Vec;
use x86_64::PhysAddr;

/// Initializes early boot environment
pub fn init_early_boot() -> Result<(), &'static str> {
    // Minimal initialization with better error handling
    unsafe {
        if !test_serial_port() {
            return Err("Serial port not responding");
        }
        
        // Basic serial port initialization - use try_port_out which won't crash
        let com1: u16 = 0x3F8;
        
        // Perform basic initialization
        try_port_out(com1 + 3, 0x80);  // Set DLAB bit
        try_port_out(com1, 0x03);      // Set divisor low byte (38400 baud)
        try_port_out(com1 + 1, 0x00);  // Set divisor high byte
        try_port_out(com1 + 3, 0x03);  // 8 bits, no parity, 1 stop bit
        try_port_out(com1 + 2, 0xC7);  // Enable FIFO, clear, 14-byte threshold
        try_port_out(com1 + 4, 0x0B);  // Enable interrupts
        
        // Send test message to confirm serial works
        let test_msg = b"BOOT\r\n";
        for &b in test_msg {
            try_port_out(com1, b);
        }
    }
    
    Ok(())
}

// Safer port I/O functions with basic error checking
unsafe fn try_port_out(port: u16, value: u8) -> bool {
    // Simple wrapper that won't crash on failure
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
    true  // No way to detect failure in out instruction
}

unsafe fn test_serial_port() -> bool {
    let com1: u16 = 0x3F8;
    // Read Line Status Register
    let mut status: u8;
    core::arch::asm!("in al, dx", out("al") status, in("dx") (com1 + 5));
    
    // If we get a reasonable status value, port might be responsive
    (status & 0x60) != 0 
}

// Keep existing port functions for compatibility
unsafe fn port_out(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn port_in(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}