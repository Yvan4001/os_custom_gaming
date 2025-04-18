#![no_std]

use core::{
    default::Default,
    option::Option::{self, None, Some},
    result::Result::{self, Err, Ok},
};

pub type Adler32Imp = fn(&mut Adler32, &[u8]) -> u32;

pub struct Adler32 {
    a: u32,
    b: u32,
}

impl Adler32 {
    pub fn new() -> Self {
        Self { a: 1, b: 0 }
    }

    pub fn from_checksum(checksum: u32) -> Self {
        Self {
            a: checksum & 0xFFFF,
            b: (checksum >> 16) & 0xFFFF,
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> u32 {
        for &byte in buf {
            self.a = (self.a + byte as u32) % 65521;
            self.b = (self.b + self.a) % 65521;
        }
        (self.b << 16) | self.a
    }

    pub fn finish(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

impl Default for Adler32 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn adler32(data: &[u8]) -> u32 {
    let mut hash = Adler32::new();
    hash.write(data)
} 