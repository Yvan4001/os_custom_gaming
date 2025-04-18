#![no_std]

use core::fmt;
use super::PhysAddr::PhysAddr;
use super::VirtAddr::VirtAddr;
use super::Page::Page;

/// Represents a page table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageTableEntry {
    /// The raw value of the page table entry.
    pub value: u64,
}

impl PageTableEntry {
    /// Creates a new page table entry.
    pub const fn new(value: u64) -> Self {
        Self { value }
    }

    /// Returns the physical address of the page.
    pub const fn addr(&self) -> PhysAddr {
        PhysAddr::new_unchecked(self.value & 0x000fffff_fffff000)
    }

    /// Returns the flags of the page table entry.
    pub const fn flags(&self) -> u64 {
        self.value & 0x00000000_00000fff
    }

    /// Sets the physical address of the page.
    pub fn set_addr(&mut self, addr: PhysAddr) {
        self.value = (self.value & 0x00000000_00000fff) | (addr.as_u64() & 0x000fffff_fffff000);
    }

    /// Sets the flags of the page table entry.
    pub fn set_flags(&mut self, flags: u64) {
        self.value = (self.value & 0x000fffff_fffff000) | (flags & 0x00000000_00000fff);
    }

    /// Returns whether the page is present.
    pub const fn is_present(&self) -> bool {
        (self.value & 1) != 0
    }

    /// Returns whether the page is writable.
    pub const fn is_writable(&self) -> bool {
        (self.value & 2) != 0
    }

    /// Returns whether the page is user accessible.
    pub const fn is_user_accessible(&self) -> bool {
        (self.value & 4) != 0
    }

    /// Returns whether the page is accessed.
    pub const fn is_accessed(&self) -> bool {
        (self.value & 32) != 0
    }

    /// Returns whether the page is dirty.
    pub const fn is_dirty(&self) -> bool {
        (self.value & 64) != 0
    }

    /// Returns whether the page is huge.
    pub const fn is_huge(&self) -> bool {
        (self.value & 128) != 0
    }

    /// Returns whether the page is global.
    pub const fn is_global(&self) -> bool {
        (self.value & 256) != 0
    }

    /// Returns whether the page is no execute.
    pub const fn is_no_execute(&self) -> bool {
        (self.value & 0x8000000000000000) != 0
    }
}

impl fmt::Display for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageTableEntry({:x})", self.value)
    }
} 