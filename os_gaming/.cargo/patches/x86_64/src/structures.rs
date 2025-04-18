#![no_std]

/// Represents a CPU structure.
#[derive(Debug, Clone, Copy)]
pub struct Gdt {
    /// The base address of the GDT.
    pub base: u64,
    /// The limit of the GDT.
    pub limit: u16,
}

impl Gdt {
    /// Creates a new GDT.
    pub const fn new(base: u64, limit: u16) -> Self {
        Self { base, limit }
    }
}

/// Represents an IDT entry.
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry {
    /// The base address of the interrupt handler.
    pub base: u64,
    /// The segment selector.
    pub selector: u16,
    /// The flags.
    pub flags: u16,
}

impl IdtEntry {
    /// Creates a new IDT entry.
    pub const fn new(base: u64, selector: u16, flags: u16) -> Self {
        Self {
            base,
            selector,
            flags,
        }
    }
}

/// Represents an IDT.
#[derive(Debug, Clone, Copy)]
pub struct Idt {
    /// The base address of the IDT.
    pub base: u64,
    /// The limit of the IDT.
    pub limit: u16,
}

impl Idt {
    /// Creates a new IDT.
    pub const fn new(base: u64, limit: u16) -> Self {
        Self { base, limit }
    }
} 