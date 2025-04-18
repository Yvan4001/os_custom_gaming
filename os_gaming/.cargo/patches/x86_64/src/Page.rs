#![no_std]

use core::fmt;
use super::PhysAddr::PhysAddr;

/// Represents a memory page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Page {
    /// The physical address of the page.
    pub addr: PhysAddr,
}

impl Page {
    /// Creates a new page.
    pub const fn new(addr: PhysAddr) -> Self {
        Self { addr }
    }

    /// Returns the physical address of the page.
    pub const fn as_addr(&self) -> PhysAddr {
        self.addr
    }

    /// Returns the page number.
    pub const fn as_u64(&self) -> u64 {
        self.addr.as_u64() / 4096
    }

    /// Creates a page from a page number.
    pub const fn from_u64(page: u64) -> Self {
        Self {
            addr: PhysAddr::new_unchecked(page * 4096),
        }
    }

    /// Aligns a physical address up to a page boundary.
    pub const fn align_up(addr: PhysAddr) -> Self {
        Self {
            addr: addr.align_up(4096),
        }
    }

    /// Aligns a physical address down to a page boundary.
    pub const fn align_down(addr: PhysAddr) -> Self {
        Self {
            addr: addr.align_down(4096),
        }
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({})", self.as_u64())
    }
} 