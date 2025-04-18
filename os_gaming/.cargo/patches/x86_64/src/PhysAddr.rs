use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// A physical memory address.
///
/// This type represents a physical memory address. It is guaranteed to be valid
/// for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysAddr(u64);

impl PhysAddr {
    /// Creates a new physical address.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// address is valid for the current platform.
    pub const unsafe fn new(addr: u64) -> PhysAddr {
        PhysAddr(addr)
    }

    /// Creates a new physical address without checking if it's valid.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// address is valid for the current platform.
    pub const unsafe fn new_unchecked(addr: u64) -> PhysAddr {
        PhysAddr(addr)
    }

    /// Returns the address as a raw integer.
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Aligns the address upwards to the given alignment.
    ///
    /// # Panics
    ///
    /// Panics if the alignment is not a power of two.
    pub fn align_up(self, align: u64) -> PhysAddr {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        let mask = align - 1;
        let addr = (self.0 + mask) & !mask;
        unsafe { PhysAddr::new_unchecked(addr) }
    }

    /// Aligns the address downwards to the given alignment.
    ///
    /// # Panics
    ///
    /// Panics if the alignment is not a power of two.
    pub fn align_down(self, align: u64) -> PhysAddr {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        let mask = align - 1;
        let addr = self.0 & !mask;
        unsafe { PhysAddr::new_unchecked(addr) }
    }

    /// Checks if the address is aligned to the given alignment.
    ///
    /// # Panics
    ///
    /// Panics if the alignment is not a power of two.
    pub fn is_aligned(self, align: u64) -> bool {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        self.0 & (align - 1) == 0
    }
}

impl Add<u64> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: u64) -> PhysAddr {
        unsafe { PhysAddr::new_unchecked(self.0 + rhs) }
    }
}

impl AddAssign<u64> for PhysAddr {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for PhysAddr {
    type Output = PhysAddr;

    fn sub(self, rhs: u64) -> PhysAddr {
        unsafe { PhysAddr::new_unchecked(self.0 - rhs) }
    }
}

impl SubAssign<u64> for PhysAddr {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PhysAddr({:#x})", self.0)
    }
} 