#![no_std]
#![deny(missing_docs)]

//! Single assignment cells and lazy values.
//! 
//! This crate provides two types:
//! * `OnceCell<T>` - a cell which can be written to only once
//! * `Lazy<T>` - a value which is computed on the first access
//!
//! This is a no_std compatible version of the crate.

extern crate alloc;

mod race;

pub use race::{OnceBox, OnceBool, OnceRef, OnceNonZeroUsize};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

mod unsync;

pub use unsync::Lazy;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_cell() {
        let cell = OnceCell::new();
        assert!(cell.get().is_none());
        
        cell.set(42).unwrap();
        assert_eq!(cell.get(), Some(&42));
        
        cell.set(43).unwrap_err();
        assert_eq!(cell.get(), Some(&42));
    }

    #[test]
    fn test_lazy() {
        let lazy = Lazy::new(|| 42);
        assert_eq!(*lazy, 42);
        assert_eq!(*lazy, 42); // Should not recompute
    }
} 