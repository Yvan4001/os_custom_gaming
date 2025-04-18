#![no_std]

use core::fmt;
use super::PhysAddr::PhysAddr;
use super::Page::Page;

/// Represents a memory frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame {
    /// The page of the frame.
    pub page: Page,
}

impl Frame {
    /// Creates a new frame.
    pub const fn new(page: Page) -> Self {
        Self { page }
    }

    /// Returns the physical address of the frame.
    pub const fn as_addr(&self) -> PhysAddr {
        self.page.as_addr()
    }

    /// Returns the frame number.
    pub const fn as_u64(&self) -> u64 {
        self.page.as_u64()
    }

    /// Creates a frame from a frame number.
    pub const fn from_u64(frame: u64) -> Self {
        Self {
            page: Page::from_u64(frame),
        }
    }

    /// Aligns a physical address up to a frame boundary.
    pub const fn align_up(addr: PhysAddr) -> Self {
        Self {
            page: Page::align_up(addr),
        }
    }

    /// Aligns a physical address down to a frame boundary.
    pub const fn align_down(addr: PhysAddr) -> Self {
        Self {
            page: Page::align_down(addr),
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame({})", self.as_u64())
    }
} 