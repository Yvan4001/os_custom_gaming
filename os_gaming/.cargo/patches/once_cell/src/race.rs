#![no_std]

extern crate alloc;

use core::marker::{PhantomData, Send, Sync};
use core::mem::drop;
use core::ops::{Drop, FnOnce};
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Ok, Err};
use core::sync::atomic::{AtomicPtr, Ordering};
use core::default::Default;
use core::clone::Clone;
use alloc::boxed::Box;

use core::{
    num::NonZeroUsize,
    ptr,
    fmt,
};

pub struct OnceNonZeroUsize {
    ptr: AtomicPtr<()>,
}

impl OnceNonZeroUsize {
    pub const fn new() -> OnceNonZeroUsize {
        OnceNonZeroUsize {
            ptr: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn get(&self) -> Option<NonZeroUsize> {
        let val = self.ptr.load(Ordering::Acquire);
        if val.is_null() {
            None
        } else {
            Some(unsafe { NonZeroUsize::new_unchecked(val as usize) })
        }
    }

    pub fn set(&self, value: NonZeroUsize) -> Result<(), ()> {
        let exchange = self.compare_exchange(value);
        match exchange {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> NonZeroUsize
    where
        F: FnOnce() -> NonZeroUsize,
    {
        match self.get_or_try_init(|| Ok::<NonZeroUsize, Void>(f())) {
            Ok(val) => val,
            Err(void) => match void {},
        }
    }

    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<NonZeroUsize, E>
    where
        F: FnOnce() -> Result<NonZeroUsize, E>,
    {
        if let Some(it) = self.get() {
            Ok(it)
        } else {
            self.init(f)
        }
    }

    fn init<E>(&self, f: impl FnOnce() -> Result<NonZeroUsize, E>) -> Result<NonZeroUsize, E> {
        let nz = f()?;
        if let Err(old) = self.compare_exchange(nz) {
            return Ok(unsafe { NonZeroUsize::new_unchecked(old as usize) });
        }
        Ok(unsafe { NonZeroUsize::new_unchecked(nz.get()) })
    }

    fn compare_exchange(&self, val: NonZeroUsize) -> Result<usize, usize> {
        let old = self.ptr.compare_exchange(
            ptr::null_mut(),
            val.get() as *mut (),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        match old {
            Ok(old) => Ok(old as usize),
            Err(old) => Err(old as usize),
        }
    }
}

pub struct OnceBool {
    inner: OnceNonZeroUsize,
}

impl OnceBool {
    pub const fn new() -> OnceBool {
        OnceBool {
            inner: OnceNonZeroUsize::new(),
        }
    }

    pub fn get(&self) -> Option<bool> {
        self.inner.get().map(|it| it.get() == 2)
    }

    pub fn set(&self, value: bool) -> Result<(), ()> {
        self.inner.set(unsafe {
            NonZeroUsize::new_unchecked(if value { 2 } else { 1 })
        })
    }

    pub fn get_or_init<F>(&self, f: F) -> bool
    where
        F: FnOnce() -> bool,
    {
        self.inner
            .get_or_init(|| unsafe {
                NonZeroUsize::new_unchecked(if f() { 2 } else { 1 })
            })
            .get()
            == 2
    }

    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<bool, E>
    where
        F: FnOnce() -> Result<bool, E>,
    {
        self.inner
            .get_or_try_init(|| {
                Ok(unsafe {
                    NonZeroUsize::new_unchecked(if f()? { 2 } else { 1 })
                })
            })
            .map(|it| it.get() == 2)
    }
}

pub struct OnceRef<'a, T> {
    inner: OnceNonZeroUsize,
    _marker: PhantomData<&'a T>,
}

unsafe impl<'a, T: Sync> Sync for OnceRef<'a, T> {}

impl<'a, T> Default for OnceRef<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> OnceRef<'a, T> {
    pub const fn new() -> OnceRef<'a, T> {
        OnceRef {
            inner: OnceNonZeroUsize::new(),
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> Option<&'a T> {
        self.inner.get().map(|it| unsafe {
            let ptr = it.get() as *const T;
            &*ptr
        })
    }

    pub fn set(&self, value: &'a T) -> Result<(), ()> {
        self.inner.set(unsafe {
            NonZeroUsize::new_unchecked(value as *const T as usize)
        })
    }

    pub fn get_or_init<F>(&self, f: F) -> &'a T
    where
        F: FnOnce() -> &'a T,
    {
        match self.get_or_try_init(|| Ok::<&'a T, Void>(f())) {
            Ok(val) => val,
            Err(void) => match void {},
        }
    }

    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&'a T, E>
    where
        F: FnOnce() -> Result<&'a T, E>,
    {
        if let Some(val) = self.get() {
            Ok(val)
        } else {
            self.init(f)
        }
    }

    fn init<E>(&self, f: impl FnOnce() -> Result<&'a T, E>) -> Result<&'a T, E> {
        let value = f()?;
        if let Err(old) = self.compare_exchange(value) {
            return Ok(unsafe { &*(old as *const T) });
        }
        Ok(value)
    }

    fn compare_exchange(&self, value: &'a T) -> Result<(), *const T> {
        let old = self.inner.compare_exchange(unsafe {
            NonZeroUsize::new_unchecked(value as *const T as usize)
        });
        match old {
            Ok(_) => Ok(()),
            Err(old) => Err(old as *const T),
        }
    }
}

pub struct OnceBox<T> {
    inner: OnceNonZeroUsize,
    ghost: PhantomData<Option<Box<T>>>,
}

impl<T> Default for OnceBox<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for OnceBox<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.inner.get().map(|it| it.get() as *mut T) {
            if !ptr.is_null() {
                drop(unsafe { Box::from_raw(ptr) })
            }
        }
    }
}

impl<T> OnceBox<T> {
    pub const fn new() -> OnceBox<T> {
        OnceBox {
            inner: OnceNonZeroUsize::new(),
            ghost: PhantomData,
        }
    }

    pub fn with_value(value: Box<T>) -> OnceBox<T> {
        let mut res = OnceBox::new();
        res.set(value).unwrap();
        res
    }

    pub fn get(&self) -> Option<&T> {
        let ptr = self.inner.get().map(|it| it.get() as *mut T)?;
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { &*ptr })
    }

    pub fn set(&self, value: Box<T>) -> Result<(), Box<T>> {
        let value = Box::into_raw(value);
        let old = self.inner.compare_exchange(unsafe {
            NonZeroUsize::new_unchecked(value as usize)
        });
        match old {
            Ok(_) => Ok(()),
            Err(_) => {
                let value = unsafe { Box::from_raw(value) };
                return Err(value);
            }
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> Box<T>,
    {
        match self.get_or_try_init(|| Ok::<Box<T>, Void>(f())) {
            Ok(val) => val,
            Err(void) => match void {},
        }
    }

    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<Box<T>, E>,
    {
        if let Some(val) = self.get() {
            Ok(val)
        } else {
            self.init(f)
        }
    }

    fn init<E>(&self, f: impl FnOnce() -> Result<Box<T>, E>) -> Result<&T, E> {
        let value = Box::into_raw(f()?);
        let exchange = self.inner.compare_exchange(unsafe {
            NonZeroUsize::new_unchecked(value as usize)
        });
        if let Err(old) = exchange {
            drop(unsafe { Box::from_raw(value) });
            return Ok(unsafe { &*(old as *const T) });
        }
        Ok(unsafe { &*value })
    }
}

unsafe impl<T: Sync + Send> Sync for OnceBox<T> {}

impl<T: Clone> Clone for OnceBox<T> {
    fn clone(&self) -> OnceBox<T> {
        match self.get() {
            Some(value) => OnceBox::with_value(Box::new(value.clone())),
            None => OnceBox::new(),
        }
    }
}

pub enum Void {}