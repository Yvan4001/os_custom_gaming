#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Correct multiboot2 header structure
#[repr(C, packed)]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // End tag
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

// Calculate proper checksum
const HEADER_LENGTH: u32 = 24;
const MAGIC: u32 = 0xe85250d6;
const ARCH: u32 = 0;
const CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC).wrapping_sub(ARCH).wrapping_sub(HEADER_LENGTH);

#[link_section = ".multiboot_header"]
#[used]
static MULTIBOOT_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MAGIC,
    architecture: ARCH,
    header_length: HEADER_LENGTH,
    checksum: CHECKSUM,
    end_tag_type: 0,      // End tag type
    end_tag_flags: 0,     // End tag flags  
    end_tag_size: 8,      // End tag size
};

// VGA text mode helper functions
struct VgaWriter {
    position: usize,
}

impl VgaWriter {
    fn new() -> Self {
        VgaWriter { position: 0 }
    }

    fn set_position(&mut self, row: usize, col: usize) {
        self.position = row * 80 + col;
    }

    fn write_char(&mut self, character: u8, color: u8) {
        if self.position >= 2000 {
            return;
        }
        
        let byte_offset = self.position * 2;
        
        unsafe {
            let vga = 0xb8000 as *mut u8;
            *vga.add(byte_offset) = character;
            *vga.add(byte_offset + 1) = color;
        }
        
        self.position += 1;
    }

    fn write_string(&mut self, text: &str, color: u8) {
        for byte in text.bytes() {
            self.write_char(byte, color);
        }
    }

    fn newline(&mut self) {
        let current_row = self.position / 80;
        self.position = (current_row + 1) * 80;
    }

    fn clear_screen(&mut self) {
        unsafe {
            let vga = 0xb8000 as *mut u8;
            for i in 0..4000 {
                *vga.add(i) = if i % 2 == 0 { b' ' } else { 0x07 };
            }
        }
        self.position = 0;
    }
}

/*#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize VGA writer
    let mut vga = VgaWriter::new();
    
    // Clear screen first
    vga.clear_screen();
    
    // Welcome header
    vga.set_position(0, 0);
    vga.write_string("============================================", 0x0F); // White
    vga.newline();
    vga.write_string("       Welcome to FluxGridOS v1.0", 0x0A); // Green
    vga.newline();
    vga.write_string("============================================", 0x0F);
    vga.newline();
    vga.newline();
    
    // System information
    vga.write_string("System Info:", 0x0E); // Yellow
    vga.newline();
    vga.write_string("  - Architecture: x86_64", 0x07);
    vga.newline();
    vga.write_string("  - Boot Protocol: Multiboot2", 0x07);
    vga.newline();
    vga.write_string("  - Kernel Language: Rust", 0x07);
    vga.newline();
    vga.write_string("  - VGA Mode: Text 80x25", 0x07);
    vga.newline();
    vga.newline();
    
    // Status messages
    vga.write_string("Kernel Status:", 0x0E);
    vga.newline();
    vga.write_string("  [", 0x07);
    vga.write_string("OK", 0x0A); // Green
    vga.write_string("] Multiboot header loaded", 0x07);
    vga.newline();
    vga.write_string("  [", 0x07);
    vga.write_string("OK", 0x0A);
    vga.write_string("] VGA text mode initialized", 0x07);
    vga.newline();
    vga.write_string("  [", 0x07);
    vga.write_string("OK", 0x0A);
    vga.write_string("] Memory access working", 0x07);
    vga.newline();
    vga.newline();
    
    // Call kernel main
    vga.write_string("Starting kernel main...", 0x0C);
    vga.newline();
    
    kernel_main(&mut vga);
}*/

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Clear screen with basic colors
    unsafe {
        let vga = 0xb8000 as *mut u8;
        for i in 0..4000 {
            *vga.add(i) = if i % 2 == 0 { b' ' } else { 0x07 }; // White on black
        }
    }
    
    // Test with BASIC VGA colors only
    unsafe {
        let vga = 0xb8000 as *mut u8;
        
        // Line 1: White text on black background (most compatible)
        let text1 = b"FluxGridOS Boot Test";
        for (i, &byte) in text1.iter().enumerate() {
            *vga.add(i * 2) = byte;
            *vga.add(i * 2 + 1) = 0x07; // White on black
        }
        
        // Line 2: Different basic color
        let text2 = b"VgaWriter Test Line 2";
        for (i, &byte) in text2.iter().enumerate() {
            *vga.add(160 + i * 2) = byte;      // Line 2 start
            *vga.add(160 + i * 2 + 1) = 0x0F; // Bright white
        }
        
        // Line 3: Another basic color
        let text3 = b"Line 3 Different Color";
        for (i, &byte) in text3.iter().enumerate() {
            *vga.add(320 + i * 2) = byte;      // Line 3 start
            *vga.add(320 + i * 2 + 1) = 0x02; // Green
        }
    }
    
    // Test VgaWriter with basic colors
    let mut vga = VgaWriter::new();
    vga.set_position(4, 0); // Line 5
    vga.write_string("VgaWriter Basic Test", 0x07); // White on black
    
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn kernel_main(vga: &mut VgaWriter) -> ! {
    vga.newline();
    vga.write_string("============================================", 0x0B); // Cyan
    vga.newline();
    vga.write_string("         KERNEL MAIN EXECUTING", 0x0B);
    vga.newline();
    vga.write_string("============================================", 0x0B);
    vga.newline();
    vga.newline();
    
    // Show kernel activities
    vga.write_string("Initializing kernel subsystems...", 0x07);
    vga.newline();
    
    // Simulate some kernel initialization
    let subsystems = [
        "Memory Manager",
        "Process Scheduler", 
        "File System",
        "Device Drivers",
        "Network Stack",
        "Graphics Engine",
    ];
    
    for (i, subsystem) in subsystems.iter().enumerate() {
        vga.write_string("  [", 0x07);
        
        // Simulate different states
        match i % 3 {
            0 => {
                vga.write_string("OK", 0x0A); // Green
            },
            1 => {
                vga.write_string("INIT", 0x0E); // Yellow
            },
            _ => {
                vga.write_string("WAIT", 0x09); // Blue
            }
        }
        
        vga.write_string("] ", 0x07);
        vga.write_string(subsystem, 0x07);
        vga.newline();
    }
    
    vga.newline();
    vga.write_string("FluxGridOS is ready for commands!", 0x0A);
    vga.newline();
    vga.write_string("Kernel main loop started...", 0x0F);
    vga.newline();
    
    // Main kernel loop
    let mut counter = 0;
    loop {
        // Simple heartbeat
        if counter % 100000000 == 0 {
            vga.write_string(".", 0x08);
        }
        counter += 1;
        
        unsafe { 
            core::arch::asm!("cli; hlt"); 
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let mut vga = VgaWriter::new();
    vga.set_position(20, 0);
    vga.write_string("KERNEL PANIC: ", 0x4F); // Red background
    if let Some(location) = _info.location() {
        vga.write_string("at ", 0x4F);
        vga.write_string(location.file(), 0x4F);
    }
    
    loop { 
        unsafe { 
            core::arch::asm!("cli; hlt"); 
        } 
    }
}