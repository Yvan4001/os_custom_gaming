#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(warnings)] // TODO: Address warnings

use core::panic::PanicInfo;
use fluxgridOs::boot::info::{CustomBootInfo, MemoryRegion, MemoryRegionType, set_global_boot_info};
use fluxgridOs::{kernel_entry_from_lib, hcf, println};

extern crate alloc;
use alloc::vec::Vec;
use core::arch::asm;

use multiboot2::{
    BootInformation, BootInformationHeader, TagType, MemoryAreaType, FramebufferTag,
    CommandLineTag, StringError, RsdpV1Tag, RsdpV2Tag
};
use x86_64::{PhysAddr, VirtAddr};

// --- Multiboot2 Header Definition ---
const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;
const MULTIBOOT2_ARCHITECTURE_I386: u32 = 0;
const MULTIBOOT2_HEADER_TAG_END: u16 = 0;
const MULTIBOOT2_HEADER_TAG_FLAGS_REQUIRED: u16 = 0;
#[repr(C, packed(4))]
struct Multiboot2Header {
    magic: u32, architecture: u32, header_length: u32, checksum: u32,
    end_tag_type: u16, end_tag_flags: u16, end_tag_size: u32,
}
const HEADER_LENGTH: u32 = core::mem::size_of::<Multiboot2Header>() as u32;
const CALCULATED_CHECKSUM: u32 = (0u32.wrapping_sub(MULTIBOOT2_HEADER_MAGIC))
    .wrapping_sub(MULTIBOOT2_ARCHITECTURE_I386).wrapping_sub(HEADER_LENGTH);
#[link_section = ".multiboot_header"] #[used] #[no_mangle]
pub static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MULTIBOOT2_HEADER_MAGIC, architecture: MULTIBOOT2_ARCHITECTURE_I386,
    header_length: HEADER_LENGTH, checksum: CALCULATED_CHECKSUM,
    end_tag_type: MULTIBOOT2_HEADER_TAG_END,
    end_tag_flags: MULTIBOOT2_HEADER_TAG_FLAGS_REQUIRED, end_tag_size: 8,
};
// --- End Multiboot2 Header ---

const VGA_BUFFER_ADDRESS: u64 = 0xb8000;
fn vga_print_char(character: u8, offset: usize, color: u8) { 
    let vga_buffer = unsafe { &mut *(VGA_BUFFER_ADDRESS as *mut [u16; 80 * 25]) };
    vga_buffer[offset] = (color as u16) << 8 | character as u16;
 }
fn vga_print_str(s: &str, line: usize, color: u8) { 
    let mut offset = line * 80;
    for byte in s.bytes() {
        if byte == b'\0' { break; }
        vga_print_char(byte, offset, color);
        offset += 1;
    }
}

fn vga_print_hex_u32(mut n: u32, line: usize, start_col: usize, color: u8) {
    let hex_chars = b"0123456789ABCDEF";
    // Always print 8 hex digits for a u32 for simplicity and fixed width
    for i in 0..8 {
        // Get the i-th hex digit from the left (most significant)
        let digit_val = (n >> ((7 - i) * 4)) & 0xF;
        let char_to_print = hex_chars[digit_val as usize];
        if start_col + i < 80 { // Ensure column is within screen width
             vga_print_char(char_to_print, line * 80 + start_col + i, color);
        }
    }
}

// Define a proper GDT for 64-bit mode
#[repr(C, align(16))]
struct GdtEntry {
    data: [u8; 8]
}

// Simplified GDT with null descriptor, code segment, and data segment
#[link_section = ".bss.page_tables"]
#[repr(align(4096))]
struct PageTable {
    entries: [u64; 512],
}

#[link_section = ".bss.stack"]
static mut BOOTSTRAP_STACK: [u8; 4096 * 4] = [0; 4096 * 4]; // 16KiB stack

// 1. Use properly aligned page tables
#[link_section = ".bss.page_tables"]
static mut PML4: PageTable = PageTable { entries: [0; 512] };

#[link_section = ".bss.page_tables"]
static mut PDPT: PageTable = PageTable { entries: [0; 512] };

// Create a minimal GDT
static mut GDT_ENTRIES: [GdtEntry; 3] = [
    GdtEntry { data: [0, 0, 0, 0, 0, 0, 0, 0] },                     // Null descriptor
    GdtEntry { data: [0xFF, 0xFF, 0, 0, 0, 0x9A, 0xAF, 0] },         // 64-bit code segment
    GdtEntry { data: [0xFF, 0xFF, 0, 0, 0, 0x92, 0, 0] },            // Data segment
];

#[repr(C, align(8))]
struct GdtDescriptor {
    size: u16,
    address: u64
}

#[link_section = ".bss.gdt"]
static mut GDT_DESCRIPTOR: GdtDescriptor = GdtDescriptor { 
    size: 0, 
    address: 0 
};

#[link_section = ".data"]
static mut GDT_DATA: [u8; 24] = [
    // Null descriptor (8 bytes)
    0, 0, 0, 0, 0, 0, 0, 0,
    // 64-bit code segment (8 bytes) 
    0xFF, 0xFF, 0, 0, 0, 0x9A, 0xAF, 0,
    // Data segment (8 bytes)
    0xFF, 0xFF, 0, 0, 0, 0x92, 0, 0
];

// Use a simple 6-byte GDT descriptor (2 bytes size + 4 bytes address)
#[link_section = ".data"]
static mut GDT_DESCRIPTOR_DATA: [u8; 10] = [
    // Size (2 bytes): GDT size - 1 = 23
    23, 0,
    // Address (8 bytes): Will be filled in at runtime
    0, 0, 0, 0, 0, 0, 0, 0
];


extern "C" {
    fn _rust_start() -> !;
}
#[no_mangle]
static mut multiboot_magic: u32 = 0;

#[no_mangle]
static mut multiboot_ptr: u32 = 0;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Simple VGA output to confirm we're running
    vga_print_str("FLUX_KERNEL_START", 0, 0x0F);
    
    // Get multiboot information from GRUB
    let magic_value: u32;
    let mbi_addr: u32;
    
    unsafe {
        asm!(
            "mov {0:e}, eax",  // Get magic from EAX
            "mov {1:e}, ebx",  // Get MBI pointer from EBX
            out(reg) magic_value,
            out(reg) mbi_addr,
            options(preserves_flags, nomem, nostack)
        );
    }
    
    vga_print_str("MAGIC:", 1, 0x0F);
    vga_print_hex_u32(magic_value, 1, 7, 0x0F);
    vga_print_str("MBI_ADDR:", 2, 0x0F);
    vga_print_hex_u32(mbi_addr, 2, 10, 0x0F);
    
    // Validate multiboot magic
    if magic_value != 0x36d76289 {
        vga_print_str("BAD_MAGIC", 3, 0x4F);
        hcf_early_panic_hook("Invalid multiboot magic");
    }
    
    if mbi_addr == 0 {
        vga_print_str("NULL_MBI", 4, 0x4F);
        hcf_early_panic_hook("MBI address is null");
    }
    
    vga_print_str("MAGIC_OK", 3, 0x2F);
    vga_print_str("MBI_OK", 4, 0x2F);
    
    // Parse multiboot information
    let boot_info_mbi = unsafe {
        match BootInformation::load(mbi_addr as *const BootInformationHeader) {
            Ok(bi) => {
                vga_print_str("MBI_PARSE_OK", 5, 0x2F);
                bi
            }
            Err(_) => {
                vga_print_str("MBI_PARSE_ERR", 5, 0x4F);
                hcf_early_panic_hook("Failed to parse MBI");
            }
        }
    };
    
    // Create boot info for kernel
    const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_8000_0000_0000;
    let mut custom_boot_info = CustomBootInfo {
        physical_memory_offset: Some(PHYSICAL_MEMORY_OFFSET),
        memory_map_regions: Vec::new(),
        ..CustomBootInfo::default()
    };
    
    // Parse memory map
    if let Some(memory_map_tag) = boot_info_mbi.memory_map_tag() {
        vga_print_str("PARSING_MMAP", 6, 0x0F);
        for area in memory_map_tag.memory_areas() {
            let region_type = match u32::from(area.typ()) {
                1 => MemoryRegionType::Usable,
                2 => MemoryRegionType::Reserved,
                3 => MemoryRegionType::AcpiReclaimable,
                4 => MemoryRegionType::AcpiNvs,
                5 => MemoryRegionType::BadMemory,
                _ => MemoryRegionType::Reserved,
            };
            
            custom_boot_info.memory_map_regions.push(MemoryRegion {
                range: PhysAddr::new(area.start_address())..PhysAddr::new(area.end_address()),
                region_type,
            });
        }
        vga_print_str("MMAP_OK", 7, 0x2F);
    } else {
        vga_print_str("NO_MMAP", 7, 0x4F);
        hcf_early_panic_hook("No memory map");
    }
    
    vga_print_str("STARTING_KERNEL", 8, 0x0F);
    
    // Set global boot info and start kernel
    set_global_boot_info(custom_boot_info);
    kernel_entry_from_lib();
}

/*#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Even the panic handler might not be reached if the crash is before this.
    // This hcf_early should be extremely simple, e.g., just cli; hlt.
    hcf_early_panic_hook("PANIC_MINIMAL");
}

fn hcf_early_panic_hook(_msg: &str) -> ! {
    loop {
        unsafe { asm!("cli; hlt", options(nomem, nostack, preserves_flags)); }
    }
}*/


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_print_str("PANIC_MAIN", 24, 0x4F);
    hcf();
}

fn hcf_early_panic_hook(msg: &str) -> ! { /* ... as before ... */
    vga_print_str("HCF_EARLY: ", 23, 0xCF);
    let mut line = 23; let mut col = 12;
    for &byte in msg.as_bytes() { if col >= 80 { line += 1; col = 0; } if line >= 25 { break; } vga_print_char(byte, line * 80 + col, 0xCF); col +=1; }
    loop { unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack, preserves_flags)); } }
}