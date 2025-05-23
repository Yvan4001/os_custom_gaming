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

    unsafe {
        asm!(
            "mov {0:e}, eax",
            "mov {1:e}, ebx",
            out(reg) multiboot_magic,
            out(reg) multiboot_ptr,
            options(preserves_flags, nomem, nostack)
        );
    }
    
    // Force values to be static for debugging
    unsafe {
        let raw_magic = multiboot_magic;
        vga_print_str("RAW_EAX:", 0, 0x0F);
        vga_print_hex_u32(raw_magic, 0, 9, 0x0F);
    }

    let magic_value_from_asm = unsafe { multiboot_magic };
    let mbi_physical_address_from_asm = unsafe { multiboot_ptr };

    vga_print_str("_START_STACK_OK", 0, 0x0F);
    vga_print_str("MAGIC:", 1, 0x0F);
    vga_print_hex_u32(magic_value_from_asm, 1, 7, 0x0F);
    vga_print_str("MBI_PA:", 2, 0x0F);
    vga_print_hex_u32(mbi_physical_address_from_asm, 2, 8, 0x0F);

    vga_print_str("HEX_MAGIC:", 3, 0x0F);
    vga_print_hex_u32(magic_value_from_asm, 3, 10, 0x0F);
    
    // Use hardcoded comparison to verify the hex representation is correct
    let expected_grub_magic: u32 = 0xB0DC4ACD;
    vga_print_str("EXPECTED:", 3, 0x0F);
    vga_print_hex_u32(expected_grub_magic, 3, 20, 0x0F);
    
    // Then modify your check to use a variable comparison
    /*if magic_value_from_asm == expected_grub_magic {
        vga_print_str("MB_YOUR_GRUB_OK", 4, 0x2F);
    } else if magic_value_from_asm == 0xB0FE5F6D { // Actual value seen
        vga_print_str("MB_ACTUAL_OK", 4, 0x2F);
    } else if magic_value_from_asm == 0x36d76289 { // Expected Multiboot2
        vga_print_str("MBv2_OK", 4, 0x2F);
    } else if magic_value_from_asm == 0x2BADB002 { // Multiboot1
        vga_print_str("MBv1_OK", 4, 0x2F);
    } else if magic_value_from_asm == 0x36d76283 { // Value you saw in GDB for rax earlier
        vga_print_str("MB_ALT_OK", 4, 0x2F);
    } else {
        vga_print_str("BAD_MAGIC_HALT", 4, 0x4F);
        
        // Print actual value before panicking
        vga_print_str("ACTUAL:", 5, 0x4F);
        vga_print_hex_u32(magic_value_from_asm, 5, 8, 0x4F);
        
        hcf_early_panic_hook("Invalid Magic");
    }*/
    vga_print_str("SKIPPING_MAGIC_CHECK", 4, 0x0F);

    if mbi_physical_address_from_asm == 0 {
        vga_print_str("MBI_NULL_HALT", 5, 0x4F);
        hcf_early_panic_hook("MBI Address is NULL");
    }
    // If your GDB showed rbx = 0x1000e0, this check should pass.
    // The value you printed for 'mbi_physical_address_from_ebx' was 0x1000e0 ($2 = 1048800)
    // So this check should now be with `mbi_physical_address_from_asm`.
    vga_print_str("MBI_ADDR_OK", 5, 0x2F);

    vga_print_str("MBI_PTR_CHECK", 6, 0x0F);

    /*if (mbi_physical_address_from_asm & 0x7) != 0 {
        vga_print_str("MBI_MISALIGNED", 6, 0x4F);
        hcf_early_panic_hook("MBI address not 8-byte aligned");
    }*/

    // Try to safely read the basic header first before full parsing
    unsafe {
        // Try using a volatile read for safety
        vga_print_str("ATTEMPTING DIRECT READ...", 7, 0x0F);
        let header_magic = core::ptr::read_volatile(mbi_physical_address_from_asm as *const u32);
        vga_print_str("MBI_MAGIC: ", 8, 0x0F);
        vga_print_hex_u32(header_magic, 8, 11, 0x0F);
    }

    vga_print_str("LOAD ADDR: ", 7, 0x0F);
    // Print both the high and low 32-bits


    vga_print_str("MBI_ACC_TRY", 6, 0x0F);
    vga_print_hex_u32(mbi_physical_address_from_asm, 6, 14, 0x0F);

    unsafe {
        // Set up the GDT descriptor address at runtime
        let gdt_addr = &GDT_DATA as *const _ as u64;
        GDT_DESCRIPTOR_DATA[2] = (gdt_addr & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[3] = ((gdt_addr >> 8) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[4] = ((gdt_addr >> 16) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[5] = ((gdt_addr >> 24) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[6] = ((gdt_addr >> 32) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[7] = ((gdt_addr >> 40) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[8] = ((gdt_addr >> 48) & 0xFF) as u8;
        GDT_DESCRIPTOR_DATA[9] = ((gdt_addr >> 56) & 0xFF) as u8;
        
        // Load the GDT - use the raw address of the descriptor
        asm!("lgdt [{}]", in(reg) &GDT_DESCRIPTOR_DATA);
    }

    #[link_section = ".bss.page_tables"]
    static mut BOOT_PML4: PageTable = PageTable { entries: [0; 512] };

    #[link_section = ".bss.page_tables"]
    static mut BOOT_PDPT: PageTable = PageTable { entries: [0; 512] };

    unsafe {
        vga_print_str("STATIC_PAGE_TABLES", 10, 0x0F);
        
        // Get physical addresses of static tables
        let pml4_phys = &BOOT_PML4 as *const _ as u64;
        let pdpt_phys = &BOOT_PDPT as *const _ as u64;
        
        // Set up identity mapping
        BOOT_PML4.entries[0] = pdpt_phys | 0x3;  // Present + writable
        BOOT_PDPT.entries[0] = 0x83;             // Present + writable + huge page (1GB)
        
        // Map higher half
        let high_index = (0xFFFF_8000_0000_0000u64 >> 39) & 0x1FF;
        BOOT_PML4.entries[high_index as usize] = pdpt_phys | 0x3;
        
        // Print addresses for debugging
        vga_print_str("PML4:", 11, 0x0F);
        vga_print_hex_u32(pml4_phys as u32, 11, 6, 0x0F);
        
        // Enable PAE
        let mut cr4: u32;
        asm!("mov {0}, cr4", out(reg) cr4);
        cr4 |= 1 << 5;  // Set PAE bit
        asm!("mov cr4, {0}", in(reg) cr4);
        
        // Point CR3 to page table
        asm!("mov cr3, {0}", in(reg) pml4_phys);
        
        // Load the GDT
        asm!("lgdt [{}]", in(reg) &GDT_DESCRIPTOR);
        
        // Enable long mode
        asm!(
            "mov ecx, 0xC0000080",
            "rdmsr",
            "or eax, 0x100",
            "wrmsr",
            options(nomem, nostack)
        );
        
        // Enable paging with far jump
        let mut cr0: u32;
        asm!("mov {0}, cr0", out(reg) cr0);
        cr0 |= 1 << 31;  // Set PG bit
        
        // Far jump to 64-bit code segment after enabling paging
        asm!(
            "mov cr0, {0}",
            "push 0x08",              // Code segment selector
            "lea rax, [rip + 2f]",    // Changed to reference label 2
            "push rax",
            "retfq",                  // Far return (acts like far jump)
            "2:",                     // Target after mode switch (matches 2f)
            in(reg) cr0
        );
        
        vga_print_str("LONG_MODE_OK", 6, 0x2F);
    }

    let boot_info_mbi = unsafe {
        // Ensure 8-byte alignment
        let aligned_addr = mbi_physical_address_from_asm & !0x7; 
        
        // Debug the address
        vga_print_str("ALIGNED:", 9, 0x0F);
        vga_print_hex_u32(aligned_addr, 9, 10, 0x0F);
        
        // Now load it - the identity mapping should allow access
        match BootInformation::load(aligned_addr as *const BootInformationHeader) {
            Ok(bi) => { vga_print_str("MBI_LOAD_OK", 10, 0x2F); bi }
            Err(_) => { vga_print_str("MBI_LOAD_ERR", 10, 0x4F); hcf_early_panic_hook("Failed to load MBI!"); }
        }
    };
    vga_print_str("MBI_PARSED", 8, 0x0F);

    const PHYSICAL_MEMORY_OFFSET_KERNEL: u64 = 0xFFFF_8000_0000_0000;
    let mut custom_boot_info = CustomBootInfo {
        physical_memory_offset: Some(PHYSICAL_MEMORY_OFFSET_KERNEL),
        memory_map_regions: Vec::new(),
        ..CustomBootInfo::default()
    };

    // Command Line
    if let Some(cmdline_tag) = boot_info_mbi.command_line_tag() {
        match cmdline_tag.cmdline() {
            Ok(_cmd_str) => { vga_print_str("CMD_OK", 10, 0x2F); }
            Err(_) => { vga_print_str("CMD_ERR_PARSE", 10, 0x4F); }
        }
    } else { vga_print_str("NO_CMD_TAG", 10, 0x0F); }

    // Memory Map
    if let Some(memory_map_tag) = boot_info_mbi.memory_map_tag() {
        vga_print_str("MMAP_START", 11, 0x0F);
        for area in memory_map_tag.memory_areas() {
            let region_type = match u32::from(area.typ()) {  // Fixed conversion
                1 => MemoryRegionType::Usable,           // Available
                3 => MemoryRegionType::AcpiReclaimable,  // ACPI reclaimable
                2 => MemoryRegionType::Reserved,         // Reserved
                4 => MemoryRegionType::AcpiNvs,          // ACPI NVS
                5 => MemoryRegionType::BadMemory,        // Bad RAM
                _ => MemoryRegionType::Reserved,         // Default for others
            };
            custom_boot_info.memory_map_regions.push(MemoryRegion {
                range: PhysAddr::new(area.start_address())..PhysAddr::new(area.end_address()),
                region_type,
            });
        }
        vga_print_str("MMAP_OK", 7, 0x2F);
    } else { 
        vga_print_str("NO_MMAP", 7, 0x4F); 
        hcf_early_panic_hook("No MemMapTag"); 
    }

    // Framebuffer Info
    if let Some(framebuffer_tag_option_result) = boot_info_mbi.framebuffer_tag() {
        match framebuffer_tag_option_result {
            Ok(tag_ref) => { /* ... populate custom_boot_info ... */ vga_print_str("FB_OK", 8, 0x2F); }
            Err(_) => { vga_print_str("FB_TAG_ERR", 8, 0x4F); }
        }
    } else { vga_print_str("NO_FB_TAG", 8, 0x0F); }

    // RSDP (ACPI)
    if let Some(rsdp_v1_tag_ref) = boot_info_mbi.rsdp_v1_tag() {
        custom_boot_info.rsdp_addr = Some(rsdp_v1_tag_ref.rsdt_address() as u64);
        vga_print_str("RSDPv1_OK", 9, 0x2F);
    } else if let Some(rsdp_v2_tag_ref) = boot_info_mbi.rsdp_v2_tag() {
        custom_boot_info.rsdp_addr = Some(rsdp_v2_tag_ref.xsdt_address() as u64);
        vga_print_str("RSDPv2_OK", 9, 0x2F);
    } else { vga_print_str("NO_RSDP", 9, 0x0F); }


    vga_print_str("SET_GLOBAL", 11, 0x0F);
    set_global_boot_info(custom_boot_info);
    vga_print_str("CALL_LIB", 12, 0x0F);

    kernel_entry_from_lib();
}

/*#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Absolute minimum: disable interrupts and halt.
    // If QEMU still crashes (VNC "connection closed by server"),
    // the issue is likely with the ELF file itself, its loading,
    // or the very first instruction being invalid in the GRUB-prepared environment.
    unsafe {
        asm!("cli; hlt", options(nomem, nostack, preserves_flags));
    }
    // This loop is technically unreachable due to hlt if no interrupts.
    loop {}
}

#[panic_handler]
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