#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::marker::Sized;
use core::option::Option;
use core::result::Result;

mod ser;
mod de;

pub use ser::*;
pub use de::*;
pub use serde_derive::{Serialize, Deserialize};

pub trait Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer;
}

pub trait Deserialize<'de>: Sized {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>;
} 