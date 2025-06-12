#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]
#![allow(warnings)]

use core::panic::PanicInfo;
use core::arch::asm;

// These are the only functions you should need from your lib.rs in main.rs
// Ensure they are `pub` in your lib.rs
use fluxgridOs::{kernel_entry_from_lib, hcf}; 

// --- Multiboot2 Header for GRUB ---
#[repr(C, packed(4))]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}
const HEADER_LENGTH: u32 = core::mem::size_of::<Multiboot2Header>() as u32;
const MAGIC: u32 = 0xe85250d6;
const ARCH: u32 = 0;
const CALCULATED_CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC).wrapping_sub(ARCH).wrapping_sub(HEADER_LENGTH);

#[link_section = ".multiboot_header"]
#[used]
#[no_mangle]
static MULTIBOOT_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MAGIC,
    architecture: ARCH,
    header_length: HEADER_LENGTH,
    checksum: CALCULATED_CHECKSUM,
    end_tag_type: 0,
    end_tag_flags: 0,
    end_tag_size: 8,
};

// --- Very Early VGA Printing & Panic ---
const VGA_BUFFER: *mut u16 = 0xb8000 as *mut u16;

fn kprint_char_early(character: u8, offset: usize, color: u8) {
    // This function assumes the VGA buffer at 0xb8000 is identity-mapped and writable.
    if offset < 80 * 25 {
        unsafe {
            VGA_BUFFER.add(offset).write_volatile((color as u16) << 8 | (character as u16));
        }
    }
}

fn kprint_str_early(s: &str, line: isize, col: isize, color: u8) {
    let line_offset = line * 80;
    for (i, &byte) in s.as_bytes().iter().enumerate() {
        if line_offset + col + (i as isize) >= 0 && line_offset + col + (i as isize) < 80 * 25 {
            kprint_char_early(byte, (line_offset + col + i as isize) as usize, color);
        }
    }
}

fn kprint_hex_u32_early(mut n: u32, line: isize, col: isize, color: u8) {
    let hex_chars = b"0123456789ABCDEF";
    for i in 0..8 {
        let digit_val = (n >> ((7 - i) * 4)) & 0xF;
        let char_to_print = hex_chars[digit_val as usize];
        kprint_char_early(char_to_print, (line * 80 + col + i as isize) as usize, color);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kprint_str_early("KERNEL PANIC!", 24, 0, 0x4F); // White on Red
    hcf();
}

// --- Bootstrap Stack ---
#[link_section = ".bss.stack"]
static mut BOOTSTRAP_STACK: [u8; 4096 * 4] = [0; 4096 * 4]; // 16KiB

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Set up a reliable stack IMMEDIATELY.
    unsafe {
        let stack_top = BOOTSTRAP_STACK.as_ptr() as u64 + BOOTSTRAP_STACK.len() as u64;
        asm!("mov rsp, {}", in(reg) stack_top, options(nostack, nomem));
    }
    
    // 2. Read the boot info from registers passed by GRUB.
    let magic_val: u32;
    let mbi_addr: u32;
    unsafe {
        asm!(
        "mov {0:e}, eax",
        "mov ecx, ebx",
        "mov {1:e}, ecx",
        out(reg) magic_val,
        out(reg) mbi_addr,
        options(nostack, nomem, preserves_flags)
    );
    }

    // 3. Print the RAW values received from GRUB for debugging.
    kprint_str_early("GRUB -> EAX:", 1, 0, 0x0F);
    kprint_hex_u32_early(magic_val, 1, 13, 0x0E);
    kprint_str_early("GRUB -> EBX:", 2, 0, 0x0F);
    kprint_hex_u32_early(mbi_addr, 2, 13, 0x0E);

    // 4. Check the magic number strictly.
    if magic_val != 0x36d76289 {
        kprint_str_early("FATAL: Invalid Multiboot2 Magic Number.", 4, 0, 0x4F);
        hcf(); // Halt the system. Do not proceed.
    }
    
    // 5. Check the MBI address.
    if mbi_addr == 0 {
        kprint_str_early("FATAL: MBI Address is NULL.", 5, 0, 0x4F);
        hcf();
    }
    
    // If we get here, the bootloader contract seems to be met.
    kprint_str_early("OK: Bootloader check passed. Handoff to kernel...", 7, 0, 0x0A);

    // Call the main entry point in your library, passing the boot information.
    kernel_entry_from_lib(magic_val as u64, mbi_addr as u64);
}
