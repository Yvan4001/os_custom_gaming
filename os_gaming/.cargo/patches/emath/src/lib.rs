#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign};

mod vec2;
mod pos2;
mod rect;
mod history;

pub use vec2::Vec2;
pub use pos2::Pos2;
pub use rect::Rect;
pub use history::History;

/// A trait for types that can be converted to a Vec2
pub trait ToVec2 {
    fn to_vec2(&self) -> Vec2;
}

/// A trait for types that can be converted to a Pos2
pub trait ToPos2 {
    fn to_pos2(&self) -> Pos2;
}

/// A trait for types that can be converted to a Rect
pub trait ToRect {
    fn to_rect(&self) -> Rect;
}

/// A trait for types that can be rotated
pub trait Rotate {
    fn rotate(&self, angle: f32) -> Self;
}

/// A trait for types that can be scaled
pub trait Scale {
    fn scale(&self, factor: f32) -> Self;
}

/// A trait for types that can be interpolated
pub trait Lerp {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

/// A trait for types that can be clamped
pub trait Clamp {
    fn clamp(&self, min: &Self, max: &Self) -> Self;
}

/// A trait for types that can be compared with a tolerance
pub trait ApproxEq {
    fn approx_eq(&self, other: &Self, epsilon: f32) -> bool;
}

/// A trait for types that can be converted to a string
pub trait ToString {
    fn to_string(&self) -> alloc::string::String;
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec2({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Pos2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pos2({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rect({}, {}, {}, {})", self.min.x, self.min.y, self.max.x, self.max.y)
    }
} 