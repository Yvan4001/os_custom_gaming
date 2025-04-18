#![no_std]

use core::{
    cell::UnsafeCell,
    fmt,
    ops::Deref,
    marker::PhantomData,
};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

pub struct Lazy<T, F = fn() -> T> {
    cell: UnsafeCell<Option<T>>,
    init: UnsafeCell<Option<F>>,
}

unsafe impl<T, F: Send> Sync for Lazy<T, F> where T: Send + Sync {}

impl<T, F> Lazy<T, F> {
    pub const fn new(f: F) -> Lazy<T, F> {
        Lazy {
            cell: UnsafeCell::new(None),
            init: UnsafeCell::new(Some(f)),
        }
    }

    pub fn get(&self) -> &T {
        unsafe {
            match *self.cell.get() {
                Some(ref value) => value,
                None => {
                    let init = (*self.init.get()).take().unwrap();
                    let value = init();
                    *self.cell.get() = Some(value);
                    (*self.cell.get()).as_ref().unwrap()
                }
            }
        }
    }
}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    pub fn force(this: &Lazy<T, F>) -> &T {
        this.get()
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        Lazy::force(self)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lazy")
            .field("cell", &unsafe { &*self.cell.get() })
            .finish()
    }
}

impl<T: Default> Default for Lazy<T, fn() -> T> {
    fn default() -> Self {
        Self::new(T::default)
    }
}

pub struct OnceCell<T> {
    value: UnsafeCell<Option<T>>,
}

impl<T> OnceCell<T> {
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { &*self.value.get() }.as_ref()
    }

    pub fn set(&self, value: T) -> Result<(), T> {
        unsafe {
            if (*self.value.get()).is_some() {
                return Err(value);
            }
            *self.value.get() = Some(value);
        }
        Ok(())
    }
}

unsafe impl<T: Send> Send for OnceCell<T> {}
unsafe impl<T: Sync> Sync for OnceCell<T> {} 