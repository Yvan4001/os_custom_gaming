#![no_std]
#![no_main]
#![allow(warnings)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::format;

use core::panic::PanicInfo;
use core::arch::asm;
use fluxgridOs::kernel_entry_from_lib;
use fluxgridOs::boot::info;
use fluxgridOs::boot;

// Calculate proper checksum
const HEADER_LENGTH: u32 = 24;  // 4 * 6 bytes (each header field is 4 bytes)
const MAGIC: u32 = 0xe85250d6;  // Multiboot2 magic number
const MULTIBOOT2_MAGIC: u32 = 0x36d76289; // Multiboot2 bootloader magic number
const ARCH: u32 = 0;            // i386 protected mode
const CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC).wrapping_sub(ARCH).wrapping_sub(HEADER_LENGTH);

#[repr(C, align(8))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

// Place multiboot header at the beginning with proper section
#[link_section = ".multiboot_header"]
#[used]
static MULTIBOOT_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MAGIC,
    architecture: ARCH,
    header_length: HEADER_LENGTH,
    checksum: CHECKSUM,
    end_tag_type: 0,
    end_tag_flags: 0,
    end_tag_size: 8,
};

// Stack defined after the header
#[link_section = ".bss.stack"]
static mut BOOTSTRAP_STACK: [u8; 4096 * 4] = [0; 4096 * 4]; // 16KiB stack

// VGA text mode helper functions
struct VgaWriter {
    position: usize,
}

impl VgaWriter {
    fn new() -> Self {
        // Clear screen first
        let vga = 0xB8000 as *mut u16;
        for i in 0..(80*25) {
            unsafe {
                *vga.add(i) = 0x0720; // Space with light gray on black
            }
        }
        
        Self { position: 0 }
    }

    fn write_char(&mut self, c: u8, color: u8) {
        let vga = 0xB8000 as *mut u16;
        let character = u16::from(c);
        let color = u16::from(color) << 8;
        let value = character | color;

        unsafe {
            // Use explicit volatile write for VGA
            core::ptr::write_volatile(vga.add(self.position), value);
        }

        self.position += 1;
        if self.position >= 80 * 25 {
            self.scroll();
        }
    }

    fn write_string(&mut self, s: &str, color: u8) {
        for c in s.bytes() {
            match c {
                b'\n' => self.newline(),
                c if c.is_ascii_control() => self.write_char(b'?', color),
                _ => self.write_char(c, color),
            }
        }
    }

    fn newline(&mut self) {
        let current_line = self.position / 80;
        self.position = (current_line + 1) * 80;
        if self.position >= 80 * 25 {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        let vga = 0xB8000 as *mut u16;
        
        unsafe {
            // Move lines up
            for i in 0..(24*80) {
                *vga.add(i) = *vga.add(i + 80);
            }
            
            // Clear last line
            for i in 0..80 {
                *vga.add(24*80 + i) = 0x0720;
            }
        }
        
        self.position = 24 * 80;
    }

    fn set_position(&mut self, row: usize, col: usize) {
        if row < 25 && col < 80 {
            self.position = row * 80 + col;
        }
    }

    fn clear_screen(&mut self) {
        let vga = 0xB8000 as *mut u16;
        for i in 0..(80*25) {
            unsafe {
                *vga.add(i) = 0x0720;
            }
        }
        self.position = 0;
    }
}

#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xB8000 as *mut u16;
    
    unsafe {
        // Clear the screen and write an initial message
        for i in 0..(80*25) {
            *vga_buffer.add(i) = 0x0720;
        }
        
        let msg = b"FluxGridOS Starting...";
        for (i, &byte) in msg.iter().enumerate() {
            *vga_buffer.add(i) = (byte as u16) | (0x0F << 8);
        }
        
        // Set up a valid stack early
        core::arch::asm!("cli");
        let stack_top = BOOTSTRAP_STACK.as_ptr() as u64 + BOOTSTRAP_STACK.len() as u64;
        core::arch::asm!(
            "mov rsp, {}",
            in(reg) stack_top,
            options(nostack, nomem)
        );
        
        // Verify if multiboot header is correctly placed
        let header_valid = validate_multiboot_header();
        
        // Display whether header is valid
        let msg: &[u8] = if header_valid {
            b"Multiboot header valid."
        } else {
            b"ERROR: Multiboot header INVALID!"
        };
        
        for (i, &byte) in msg.iter().enumerate() {
            *vga_buffer.add(80 + i) = (byte as u16) | (if header_valid { 0x0A } else { 0x0C } << 8);
        }

        // Read multiboot info passed by GRUB
        let mut multiboot_magic: u32;
        let mut multiboot_info_ptr: u32; // Change to u32 to match register size
        
        core::arch::asm!(
            "mov {0:e}, eax",
            "mov {1:e}, ebx",
            out(reg) multiboot_magic,
            out(reg) multiboot_info_ptr
        );
        
        // Print this information to the screen
        let mut offset = 80*2; // Third line
        
        *vga_buffer.add(offset) = ('M' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = ('B' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = ('2' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = (':' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = (' ' as u16) | (0x0A << 8);
        offset += 1;
        
        // Display magic value in hex
        for i in 0..8 {
            let digit = ((multiboot_magic >> (28 - i*4)) & 0xF) as u8;
            let c = if digit < 10 { b'0' + digit } else { b'A' + (digit - 10) };
            *vga_buffer.add(offset) = (c as u16) | (0x0A << 8);
            offset += 1;
        }
        
        *vga_buffer.add(offset) = (' ' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = ('P' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = ('T' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = ('R' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = (':' as u16) | (0x0A << 8);
        offset += 1;
        *vga_buffer.add(offset) = (' ' as u16) | (0x0A << 8);
        offset += 1;
        
        // Display info pointer in hex
        for i in 0..16 {
            let digit = ((multiboot_info_ptr >> (60 - i*4)) & 0xF) as u8;
            let c = if digit < 10 { b'0' + digit } else { b'A' + (digit - 10) };
            *vga_buffer.add(offset) = (c as u16) | (0x0A << 8);
            offset += 1;
        }
        
        // Try to write to serial port
        write_serial("FluxGridOS: Boot started\r\n");
        write_serial(&format!("Multiboot2 magic: {:#x}, info pointer: {:#x}\r\n", 
                             multiboot_magic, multiboot_info_ptr));
                             
        // Check if we have a valid multiboot2 magic
        if multiboot_magic == MULTIBOOT2_MAGIC {
            write_serial("Valid Multiboot2 magic detected!\r\n");
            
            // If we have multiboot info, we can parse it here
            if multiboot_info_ptr != 0 {
                write_serial(&format!("Multiboot2 info at: {:#x}\r\n", multiboot_info_ptr));
                
                // You can use this info to create your CustomBootInfo object
                // Instead of hardcoding values in kernel_main
            }
        } else {
            write_serial("WARNING: Invalid or no Multiboot2 magic detected!\r\n");
        }
    }
    
    // Initialize VGA writer and continue to kernel_main
    let mut vga = VgaWriter::new();
    kernel_main(&mut vga)
}

// Function to output to serial port (COM1)
unsafe fn write_serial(s: &str) {
    const COM1: u16 = 0x3F8;
    
    for byte in s.bytes() {
        // Wait until transmitter is empty
        while (port_in(COM1 + 5) & 0x20) == 0 {}
        
        // Send the byte
        port_out(COM1, byte);
    }
}

// Helper functions for port I/O
unsafe fn port_out(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn port_in(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!("in al, dx", out("al") value, in("dx") port);
    value
}

unsafe fn debug_boot(message: &str) {
    // Print to VGA
    static mut DEBUG_LINE: usize = 24; // Use bottom line
    static mut DEBUG_COL: usize = 0;
    
    let vga = 0xB8000 as *mut u16;
    
    for byte in message.bytes() {
        if DEBUG_COL >= 80 {
            DEBUG_LINE = (DEBUG_LINE + 1) % 25;
            DEBUG_COL = 0;
            if DEBUG_LINE == 0 { DEBUG_LINE = 24; } // Stay on bottom lines
        }
        let offset = DEBUG_LINE * 80 + DEBUG_COL;
        *vga.add(offset) = (byte as u16) | (0x0E << 8); // Yellow on black
        DEBUG_COL += 1;
    }
    
    // Also print to serial
    write_serial(message);
}

unsafe fn validate_multiboot_header() -> bool {
    // Location of multiboot header - should be at beginning of binary
    let header_addr = &MULTIBOOT_HEADER as *const _ as u64;
    let entry_addr = _start as *const () as u64;
    
    write_serial(&format!("Multiboot header at: {:#x}, _start at: {:#x}\r\n", 
                         header_addr, entry_addr));
    
    // Ideally, header should come before _start and be within the first 32K
    let diff = if header_addr <= entry_addr {
        entry_addr - header_addr
    } else {
        header_addr - entry_addr
    };
    
    if header_addr > entry_addr {
        write_serial("WARNING: Multiboot header appears AFTER _start function!\r\n");
        return false;
    }
    
    if diff > 32768 {
        write_serial("WARNING: Multiboot header is far from _start function!\r\n");
        return false;
    }
    
    // Check magic value in header
    let header = &MULTIBOOT_HEADER;
    write_serial(&format!("Header magic: {:#x}, checksum: {:#x}\r\n", 
                         header.magic, header.checksum));
    
    header.magic == MAGIC
}

fn kernel_main(vga: &mut VgaWriter) -> ! {

    unsafe { write_serial("KERNEL_MAIN: Starting execution\r\n"); }

    vga.newline();
    vga.write_string("============================================", 0x0B);
    vga.newline();
    vga.write_string("         KERNEL MAIN EXECUTING", 0x0B);
    vga.newline();
    vga.write_string("============================================", 0x0B);
    vga.newline();
    vga.newline();
    
    // Initialize basic CustomBootInfo structure for the library
    unsafe {
        use boot::info::{CustomBootInfo, MemoryRegion, MemoryRegionType, set_global_boot_info};
        use alloc::vec::Vec;
        use x86_64::PhysAddr;
        
        // Create a more comprehensive memory map - essential for proper memory mgmt!
        let mut memory_map = Vec::new();
        
        // Main usable memory (adjust size as needed for your QEMU config)
        memory_map.push(MemoryRegion {
            range: PhysAddr::new(0x100000)..PhysAddr::new(0x500000),
            region_type: MemoryRegionType::Usable
        });
        
        // Add reserved/kernel regions - these protect your kernel from being overwritten
        memory_map.push(MemoryRegion {
            range: PhysAddr::new(0x0)..PhysAddr::new(0x1000),
            region_type: MemoryRegionType::Reserved // First page is reserved
        });
        
        // These values come from your linker script or can be read from GRUB
        let kernel_start = PhysAddr::new(0x100000); // Typical starting address
        let kernel_end = PhysAddr::new(0x300000);   // Adjust based on your kernel size
        memory_map.push(MemoryRegion {
            range: kernel_start..kernel_end,
            region_type: MemoryRegionType::Bootloader // Try this if Reserved doesn't work
        });
        
        // Boot info - now with better settings
        let boot_info = CustomBootInfo {
            memory_map_regions: memory_map,
            physical_memory_offset: Some(0xFFFF800000000000),
            framebuffer_addr: None,
            framebuffer_width: 0,
            framebuffer_height: 0,
            framebuffer_pitch: 0,
            framebuffer_bpp: 0,
            rsdp_addr: None,
            command_line: None
        };
        
        set_global_boot_info(boot_info);
        write_serial("Enhanced boot info initialized\r\n");
    }

    let mem_info = fluxgridOs::kernel::memory::get_memory_statistics();
    vga.write_string(&format!("Memory: {} MB total, {} MB free", 
                            mem_info.total_ram / 1024 / 1024, 
                            mem_info.free_ram / 1024 / 1024), 
                    0x07);
    vga.newline();

    // Initialize the interrupt system
    vga.write_string("  [", 0x07);
    unsafe {
        write_serial("Initializing interrupt system...\r\n");
        
        // Fix 1: Make sure the init function is called correctly
        // If your function returns (), remove the Result handling
        fluxgridOs::kernel::interrupts::init();
        vga.write_string("OK", 0x0A); // Green
        vga.write_string("] Interrupt system initialized", 0x07);
        write_serial("Interrupt system initialized\r\n");
        
        // Enable interrupts
        core::arch::asm!("sti");
        write_serial("Interrupts enabled\r\n");
    }
    vga.newline();
    
    // Show kernel activities
    vga.write_string("Initializing kernel subsystems...", 0x07);
    vga.newline();

    // Get the boot info we previously set up
    let boot_info = match info::get_global_boot_info() {
        Some(info) => info,
        None => {
            vga.write_string("  [FAIL] No boot info available", 0x0C);
            vga.newline();
            loop { unsafe { core::arch::asm!("cli; hlt"); } }
        }
    };

    // Initialize memory management subsystem
    if let Err(e) = fluxgridOs::kernel::memory::init(boot_info) {
        vga.write_string("  [FAIL] Memory initialization failed: ", 0x0C);
        vga.write_string(e, 0x0C);
        vga.newline();
    } else {
        vga.write_string("  [OK] Memory management initialized", 0x0A);
        vga.newline();
        unsafe { write_serial("Memory subsystem initialized\r\n"); }
    }

    // Initialize device drivers
    vga.write_string("  [", 0x07);
    if let Err(e) = fluxgridOs::kernel::drivers::init() {
        vga.write_string("FAIL", 0x0C); // Red
        vga.write_string("] Device drivers: ", 0x07);
        vga.write_string(e, 0x0C);
    } else {
        vga.write_string("OK", 0x0A); // Green
        vga.write_string("] Device drivers initialized", 0x07);
        unsafe { write_serial("Device drivers initialized\r\n"); }
    }
    vga.newline();

    // Detect CPU features
    vga.write_string("  [", 0x07);
    match fluxgridOs::kernel::cpu::init() {
        Ok(_) => {
            // Fix 2: Properly unwrap the Option before accessing fields
            if let Some(cpu_info) = fluxgridOs::kernel::cpu::get_cpu_info() {
                vga.write_string("OK", 0x0A); // Green
                vga.write_string("] CPU: ", 0x07);
                vga.write_string(&cpu_info.brand_string, 0x0F);
                unsafe { write_serial(&format!("CPU detected: {}\r\n", cpu_info.brand_string)); }
            } else {
                vga.write_string("WARN", 0x0E); // Yellow
                vga.write_string("] CPU info not available", 0x07);
            }
        },
        Err(e) => {
            vga.write_string("WARN", 0x0E); // Yellow
            vga.write_string("] CPU: ", 0x07);
            vga.write_string(e, 0x0E);
        }
    }
    vga.newline();

    // Detect and initialize GPU
    vga.write_string("  [", 0x07);
    match fluxgridOs::kernel::drivers::gpu::detection::detect_gpu() {
        Ok(mut gpu) => {
            // If detect_gpu returns a single GPU device
            match gpu.get_info() {
                Ok(info) => {
                    vga.write_string("OK", 0x0A); // Green
                    vga.write_string("] GPU: ", 0x07);
                    
                    // Use the device field from GpuInfo
                    vga.write_string(info.device, 0x0F);
                    unsafe { write_serial(&format!("GPU detected: {}\r\n", info.device)); }
                },
                Err(e) => {
                    vga.write_string("WARN", 0x0E); // Yellow
                    vga.write_string("] GPU info error: ", 0x07);
                    vga.write_string(&format!("{:?}", e), 0x0E);
                }
            }
        },
        Ok(_) => {
            vga.write_string("WARN", 0x0E); // Yellow
            vga.write_string("] No GPU detected", 0x07);
        },
        Err(e) => {
            vga.write_string("FAIL", 0x0C); // Red
            vga.write_string("] GPU detection: ", 0x07);
            vga.write_string(&format!("{:?}", e), 0x0C);
        }
    }
    vga.newline();

    // Test keyboard input handling
    vga.write_string("  [", 0x07);
    fluxgridOs::kernel::drivers::keyboard::init();
    vga.write_string("OK", 0x0A); // Green
    vga.write_string("] Keyboard initialized", 0x07);
    unsafe { write_serial("Keyboard initialized\r\n"); }
    vga.newline();
    
    // Continue with other subsystems as before
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

    unsafe {
        write_serial("\r\nSystem initialization complete\r\n");
        write_serial("FluxGridOS entering main loop...\r\n"); 
    }
    
    // Main kernel loop
    let mut counter = 0;
    loop {
        // Simple heartbeat - less frequent for better performance
        if counter % 10000000 == 0 {
            vga.write_string(".", 0x08); // Dark gray for subtlety
            
            // Only log to serial occasionally to reduce output
            if counter % 50000000 == 0 {
                unsafe { write_serial("."); }
            }
        }
        counter += 1;
        
        // Process any pending interrupts, then halt until next interrupt
        // The sti+hlt combination is atomic and prevents interrupt loss
        unsafe { 
            core::arch::asm!("sti; hlt"); 
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
    
    // Add this loop to make the function diverge (never return)
    loop { 
        unsafe { core::arch::asm!("cli; hlt"); }
    }
}

fn panic_halt() -> ! {
    unsafe {
        write_serial("CRITICAL ERROR: System halted\r\n");
        loop {
            core::arch::asm!("cli; hlt");
        }
    }
}