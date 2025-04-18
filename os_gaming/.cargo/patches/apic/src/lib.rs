#![no_std]

use x86_64::instructions::port::Port;

const APIC_BASE: u64 = 0xFEE00000;

#[derive(Debug)]
pub struct LocalApic {
    base: u64,
}

impl LocalApic {
    pub fn new() -> Self {
        // For simplicity, we'll use a fixed base address
        Self { base: APIC_BASE }
    }

    pub fn init(&mut self) {
        // Disable PIC
        unsafe {
            Port::<u8>::new(0x20).write(0x11);
            Port::<u8>::new(0xA1).write(0x11);
            Port::<u8>::new(0x21).write(0x20);
            Port::<u8>::new(0xA1).write(0x28);
            Port::<u8>::new(0x21).write(0x04);
            Port::<u8>::new(0xA1).write(0x02);
            Port::<u8>::new(0x21).write(0x01);
            Port::<u8>::new(0xA1).write(0x01);
            Port::<u8>::new(0x21).write(0xFF);
            Port::<u8>::new(0xA1).write(0xFF);
        }

        // Enable APIC
        unsafe {
            let spiv = self.read(0xF0);
            self.write(0xF0, spiv | 0x100);
        }
    }

    unsafe fn read(&self, offset: u32) -> u32 {
        core::ptr::read_volatile((self.base + offset as u64) as *const u32)
    }

    unsafe fn write(&self, offset: u32, value: u32) {
        core::ptr::write_volatile((self.base + offset as u64) as *mut u32, value);
    }
}