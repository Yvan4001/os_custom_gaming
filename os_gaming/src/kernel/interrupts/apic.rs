//! Advanced Programmable Interrupt Controller (APIC) support

use core::ptr::{read_volatile, write_volatile};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::registers::control::{Cr4, Cr4Flags};
use x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::PhysAddr;
use x86_64::instructions::port::Port;

// APIC register offsets
const APIC_ID: u32 = 0x20;
const APIC_VERSION: u32 = 0x30;
const APIC_TPR: u32 = 0x80;
const APIC_EOI: u32 = 0xB0;
const APIC_LDR: u32 = 0xD0;
const APIC_DFR: u32 = 0xE0;
const APIC_SPURIOUS: u32 = 0xF0;
const APIC_ICR_LOW: u32 = 0x300;
const APIC_ICR_HIGH: u32 = 0x310;
const APIC_TIMER: u32 = 0x320;
const APIC_THERMAL: u32 = 0x330;
const APIC_PERF: u32 = 0x340;
const APIC_LINT0: u32 = 0x350;
const APIC_LINT1: u32 = 0x360;
const APIC_ERROR: u32 = 0x370;
const APIC_TIMER_INIT: u32 = 0x380;
const APIC_TIMER_COUNT: u32 = 0x390;
const APIC_TIMER_DIV: u32 = 0x3E0;

// APIC base MSR
const MSR_APIC_BASE: u32 = 0x1B;
const APIC_BASE_ENABLE: u32 = 0x800;

lazy_static! {
    static ref APIC_BASE: Mutex<Option<u64>> = Mutex::new(None);
}

/// Check if APIC is supported and enabled
pub fn is_enabled() -> bool {
    APIC_BASE.lock().is_some()
}
/// Initialize APIC
pub fn init() -> Result<(), &'static str> {
    // Check if CPUID supports APIC
    if !check_apic_support() {
        return Err("APIC not supported by CPU");
    }

    // Get APIC base address
    let apic_base = get_apic_base();

    // Enable APIC in the MSR
    enable_apic(apic_base);

    // Save the base address
    *APIC_BASE.lock() = Some(apic_base);

    // Configure APIC
    configure_apic();

    #[cfg(feature = "std")]
    log::info!("APIC initialized at physical address 0x{:x}", apic_base);

    Ok(())
}

/// Check if APIC is supported by the CPU
fn check_apic_support() -> bool {
    use raw_cpuid::CpuId;

    // Use CPUID to check for APIC support
    if let Some(features) = CpuId::new().get_feature_info() {
        return features.has_apic();
    }

    false
}

/// Get the APIC base address from MSR
fn get_apic_base() -> u64 {
    use x86_64::registers::model_specific::Msr;

    unsafe {
        let apic_base = Msr::new(MSR_APIC_BASE);
        let base = apic_base.read();

        // Base address is bits 12-35
        base & 0xFFFFFF000
    }
}

/// Enable APIC in the MSR
fn enable_apic(base: u64) {
    use x86_64::registers::model_specific::Msr;

    unsafe {
        let mut apic_base = Msr::new(MSR_APIC_BASE);
        let current = apic_base.read();

        // Set enable bit
        apic_base.write(current | APIC_BASE_ENABLE as u64);
    }
}

/// Configure APIC for operation
fn configure_apic() {
    if let Some(base) = *APIC_BASE.lock() {
        unsafe {
            // Map APIC region to virtual memory
            // This would typically be done by your memory management system
            // For now, we'll assume identity mapping

            // Set spurious interrupt vector to 0xFF and enable APIC
            write_apic_reg(APIC_SPURIOUS, 0xFF | 0x100);

            // Disable all local interrupts
            write_apic_reg(APIC_LINT0, 0x10000);
            write_apic_reg(APIC_LINT1, 0x10000);

            // Set task priority to 0 to accept all interrupts
            write_apic_reg(APIC_TPR, 0);

            // Configure the timer
            write_apic_reg(APIC_TIMER_DIV, 0x3); // Divide by 16
            write_apic_reg(APIC_TIMER, 32 | 0x20000); // Vector 32, periodic mode
            write_apic_reg(APIC_TIMER_INIT, 10000000); // Initial count
        }
    }
}

/// Read from an APIC register
unsafe fn read_apic_reg(reg: u32) -> u32 {
    if let Some(base) = *APIC_BASE.lock() {
        let addr = (base + reg as u64) as *const u32;
        read_volatile(addr)
    } else {
        0
    }
}

/// Write to an APIC register
unsafe fn write_apic_reg(reg: u32, value: u32) {
    if let Some(base) = *APIC_BASE.lock() {
        let addr = (base + reg as u64) as *mut u32;
        write_volatile(addr, value);
    }
}

/// Send End Of Interrupt signal
pub fn end_of_interrupt() {
    unsafe {
        write_apic_reg(APIC_EOI, 0);
    }
}
