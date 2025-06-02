#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use fluxgridOs::kernel_entry_from_lib;


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

#[link_section = ".bss.stack"]
static mut BOOTSTRAP_STACK: [u8; 4096 * 4] = [0; 4096 * 4]; // 16KiB stack


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
    color: u8,
}

impl VgaWriter {
    fn new() -> Self {
        VgaWriter {
            position: 0,
            color: 0x07, // Default white text on black background
        }
    }

    fn set_position(&mut self, row: usize, col: usize) {
        if row < 25 && col < 80 { // Basic bounds check
            self.position = row * 80 + col;
        }
    }

    // write_char as you provided
    fn write_char(&mut self, character: u8, color: u8) {
        if self.position >= 80 * 25 {
            // Simple wrap around, or could scroll/clear
            self.position = 0;
        }
        
        let byte_offset = self.position * 2;
        let vga_ptr = 0xb8000 as *mut u8;
        
        unsafe {
            *vga_ptr.add(byte_offset) = character;
            *vga_ptr.add(byte_offset + 1) = color;
        }
        
        self.position += 1;
    }

    fn write_string(&mut self, text: &str, color: u8) {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.newline();
            } else if byte.is_ascii_graphic() || byte == b' ' { // Print only displayable ASCII
                self.write_char(byte, color);
            }
        }
        
        if self.position >= 80 * 25 {
            // Simple wrap around, or could scroll/clear
            self.position = 0;
        }
    }

    fn newline(&mut self) {
        let current_row = self.position / 80;
        if current_row + 1 >= 25 {
            // For simplicity, wrap to top. A real implementation might scroll.
            self.clear_screen(); // Or just self.position = 0;
        } else {
            self.position = (current_row + 1) * 80;
        }
    }

    fn clear_screen(&mut self) {
        let vga_ptr = 0xb8000 as *mut u8;
        let space = b' ';
        let attribute = 0x07; // Light gray on black

        for i in 0..(80 * 25) {
            unsafe {
                *vga_ptr.add(i * 2) = space;
                *vga_ptr.add(i * 2 + 1) = attribute;
            }
        }
        self.position = 0;
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {

    unsafe {
        let stack_top = BOOTSTRAP_STACK.as_ptr() as u64 + BOOTSTRAP_STACK.len() as u64;
        asm!(
            "mov rsp, {}",
            in(reg) stack_top,
            options(nostack, nomem) // nostack because we are setting rsp
        );
    }
    
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
}

/*#[no_mangle]
pub extern "C" fn _start() -> ! {
    // PART 1: Direct VGA write using hardcoded addresses
    // This will produce "RUST OK" in bright red on first line
    unsafe {
        let vga = 0xb8000 as *mut u8;
        
        // Write "RUST OK" in first line with 0x0C (bright red)
        *vga.add(0) = b'R'; *vga.add(1) = 0x0C;
        *vga.add(2) = b'U'; *vga.add(3) = 0x0C;
        *vga.add(4) = b'S'; *vga.add(5) = 0x0C;
        *vga.add(6) = b'T'; *vga.add(7) = 0x0C;
        *vga.add(8) = b' '; *vga.add(9) = 0x0C;
        *vga.add(10) = b'O'; *vga.add(11) = 0x0C;
        *vga.add(12) = b'K'; *vga.add(13) = 0x0C;
    }
    
    // PART 2: Use VgaWriter to write on line 2
    // This will show "VgaWriter OK" on line 2 if working
    let mut vga = VgaWriter::new();
    vga.set_position(1, 0);
    vga.write_string("VgaWriter OK", 0x0A); // Green
    
    // PART 3: Direct array-based write on line 3
    // This will always work if code is running
    unsafe {
        let vga = 0xb8000 as *mut u8;
        let message = b"CODE LOADED";
        let line_offset = 160 * 2; // Line 3
        
        for (i, &byte) in message.iter().enumerate() {
            *vga.add(line_offset + i * 2) = byte;
            *vga.add(line_offset + i * 2 + 1) = 0x0B; // Cyan
        }
    }
    
    // PART 4: ASCII art to verify memory access (line 4)
    unsafe {
        let vga = 0xb8000 as *mut u8;
        let line_offset = 160 * 3; // Line 4
        let art = b"###################";
        
        for (i, &byte) in art.iter().enumerate() {
            *vga.add(line_offset + i * 2) = byte;
            *vga.add(line_offset + i * 2 + 1) = 0x0F; // Bright white
        }
    }
    
    // Halt CPU to prevent further execution
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}*/

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

    kernel_entry_from_lib();
    
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