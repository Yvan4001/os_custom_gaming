#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(warnings)] // TODO: Address warnings

use core::panic::PanicInfo;
use fluxgridOs::boot::info::{CustomBootInfo, MemoryRegion, MemoryRegionType, set_global_boot_info};
use fluxgridOs::{kernel_entry_from_lib, hcf, println};

extern crate alloc;
use alloc::vec::Vec;

use multiboot2::{
    BootInformation, BootInformationHeader, TagType, MemoryAreaType, FramebufferTag,
    CommandLineTag, StringError, // For CommandLineTag::cmdline() result
    // ElfSectionsTag, ModuleTag, RsdpV1Tag, RsdpV2Tag, // Not directly used if using BootInformation methods
    // RsdpError, // Not needed if using BootInformation::rsdp_addr() and handling its Result
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

#[no_mangle]
pub extern "C" fn _start(magic: u64, multiboot_info_physical_address: u64) -> ! {
    if magic != 0x36d76289 { hcf_early_panic_hook("Invalid Multiboot2 magic!"); }
    if multiboot_info_physical_address == 0 { hcf_early_panic_hook("MBI Address is NULL!"); }

    const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_8000_0000_0000;

    let multiboot_info_virtual_address =
        VirtAddr::new(PHYSICAL_MEMORY_OFFSET + multiboot_info_physical_address);

    let boot_info_mbi = match unsafe {
        BootInformation::load(multiboot_info_virtual_address.as_ptr::<BootInformationHeader>())
    } {
        Ok(bi) => bi,
        Err(_load_error) => {
            hcf_early_panic_hook("Failed to load MBI structure!");
        }
    };

    let mut custom_boot_info = CustomBootInfo {
        physical_memory_offset: Some(PHYSICAL_MEMORY_OFFSET),
        memory_map_regions: Vec::new(),
        ..CustomBootInfo::default()
    };

    // Command Line
    if let Some(cmdline_tag) = boot_info_mbi.command_line_tag() {
        match cmdline_tag.cmdline() {
            Ok(cmd) => {
                // Allocate a static string using Box::leak
                let static_cmd = alloc::boxed::Box::leak(
                    alloc::string::String::from(cmd).into_boxed_str()
                );
                custom_boot_info.command_line = Some(static_cmd);
            }
            Err(_) => { /* Optionally log or handle the StringError */ }
        }
    }

    // Memory Map
    if let Some(memory_map_tag) = boot_info_mbi.memory_map_tag() {
        for area in memory_map_tag.memory_areas() {
            // area.typ() returns multiboot2::MemoryAreaType enum.
            // The match should work directly with its variants.
            use multiboot2::MemoryAreaTypeId;

            let region_type = match area.typ().into() {
                1 => MemoryRegionType::Usable,           // MULTIBOOT_MEMORY_AVAILABLE
                3 => MemoryRegionType::AcpiReclaimable,  // MULTIBOOT_MEMORY_ACPI_RECLAIMABLE
                2 => MemoryRegionType::Reserved,         // MULTIBOOT_MEMORY_RESERVED
                4 => MemoryRegionType::AcpiNvs,          // MULTIBOOT_MEMORY_ACPI_NVS
                5 => MemoryRegionType::BadMemory,        // MULTIBOOT_MEMORY_BADRAM
                _ => MemoryRegionType::Reserved,         // Default to reserved for unknown types
            };
            custom_boot_info.memory_map_regions.push(MemoryRegion {
                range: PhysAddr::new(area.start_address())..PhysAddr::new(area.end_address()),
                region_type,
            });
        }
    } else {
        hcf_early_panic_hook("Multiboot2 memory map tag not found!");
    }

    // Framebuffer Info
    // MODIFIED: framebuffer_tag() returns Option<Result<&FramebufferTag, TagError>>
    // Need to handle both Option and Result.
    if let Some(framebuffer_tag_result) = boot_info_mbi.framebuffer_tag() {
        match framebuffer_tag_result {
            Ok(framebuffer_tag_ref) => {
                custom_boot_info.framebuffer_addr = Some(framebuffer_tag_ref.address());
                custom_boot_info.framebuffer_width = framebuffer_tag_ref.width();
                custom_boot_info.framebuffer_height = framebuffer_tag_ref.height();
                custom_boot_info.framebuffer_pitch = framebuffer_tag_ref.pitch();
                custom_boot_info.framebuffer_bpp = framebuffer_tag_ref.bpp();
            }
            Err(_) => { /* Framebuffer tag present but invalid, handle error or ignore */ }
        }
    }

    if let Some(rsdp_v1_tag) = boot_info_mbi.rsdp_v1_tag() {
        custom_boot_info.rsdp_addr = Some(rsdp_v1_tag.rsdt_address() as u64);
    } else if let Some(rsdp_v2_tag) = boot_info_mbi.rsdp_v2_tag() {
        // Prefer XSDT over RSDT for v2 tags
        custom_boot_info.rsdp_addr = Some(rsdp_v2_tag.xsdt_address() as u64);
    }
    set_global_boot_info(custom_boot_info);
    kernel_entry_from_lib();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[KERNEL PANIC] in main.rs: {:?}", info);
    hcf();
}

fn hcf_early_panic_hook(msg: &str) -> ! { /* ... as before ... */
    loop { unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack, preserves_flags)); } }
}