use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// A virtual memory address.
///
/// This type represents a virtual memory address. It is guaranteed to be valid
/// for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtAddr(u64);

impl VirtAddr {
    /// Creates a new virtual address.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// address is valid for the current platform.
    pub const unsafe fn new(addr: u64) -> VirtAddr {
        VirtAddr(addr)
    }

    /// Creates a new virtual address without checking if it's valid.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// address is valid for the current platform.
    pub const unsafe fn new_unchecked(addr: u64) -> VirtAddr {
        VirtAddr(addr)
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
    pub fn align_up(self, align: u64) -> VirtAddr {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        let mask = align - 1;
        let addr = (self.0 + mask) & !mask;
        unsafe { VirtAddr::new_unchecked(addr) }
    }

    /// Aligns the address downwards to the given alignment.
    ///
    /// # Panics
    ///
    /// Panics if the alignment is not a power of two.
    pub fn align_down(self, align: u64) -> VirtAddr {
        assert!(align.is_power_of_two(), "Alignment must be a power of two");
        let mask = align - 1;
        let addr = self.0 & !mask;
        unsafe { VirtAddr::new_unchecked(addr) }
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

impl Add<u64> for VirtAddr {
    type Output = VirtAddr;

    fn add(self, rhs: u64) -> VirtAddr {
        unsafe { VirtAddr::new_unchecked(self.0 + rhs) }
    }
}

impl AddAssign<u64> for VirtAddr {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for VirtAddr {
    type Output = VirtAddr;

    fn sub(self, rhs: u64) -> VirtAddr {
        unsafe { VirtAddr::new_unchecked(self.0 - rhs) }
    }
}

impl SubAssign<u64> for VirtAddr {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VirtAddr({:#x})", self.0)
    }
} 