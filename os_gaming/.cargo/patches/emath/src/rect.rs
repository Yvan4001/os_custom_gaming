use core::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign};
use super::{Pos2, Vec2, ToString};

/// A 2D rectangle
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: Pos2,
    pub max: Pos2,
}

impl Rect {
    /// Create a new rectangle
    pub const fn new(min: Pos2, max: Pos2) -> Self {
        Self { min, max }
    }

    /// Create a rectangle from a position and size
    pub fn from_pos_size(pos: Pos2, size: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    /// Create a rectangle from a center position and size
    pub fn from_center_size(center: Pos2, size: Vec2) -> Self {
        let half_size = size * 0.5;
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Get the width of the rectangle
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Get the height of the rectangle
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Get the size of the rectangle
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    /// Get the center of the rectangle
    pub fn center(&self) -> Pos2 {
        Pos2::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    /// Check if the rectangle contains a point
    pub fn contains(&self, point: &Pos2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && point.y >= self.min.y && point.y <= self.max.y
    }

    /// Check if the rectangle intersects with another rectangle
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x && self.min.y <= other.max.y && self.max.y >= other.min.y
    }

    /// Get the intersection with another rectangle
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        Some(Self {
            min: Pos2::new(
                self.min.x.max(other.min.x),
                self.min.y.max(other.min.y),
            ),
            max: Pos2::new(
                self.max.x.min(other.max.x),
                self.max.y.min(other.max.y),
            ),
        })
    }

    /// Get the union with another rectangle
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: Pos2::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
            ),
            max: Pos2::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
            ),
        }
    }

    /// Expand the rectangle by a vector
    pub fn expand(&self, delta: Vec2) -> Self {
        Self {
            min: self.min - delta,
            max: self.max + delta,
        }
    }

    /// Shrink the rectangle by a vector
    pub fn shrink(&self, delta: Vec2) -> Self {
        Self {
            min: self.min + delta,
            max: self.max - delta,
        }
    }
}

impl Add<Vec2> for Rect {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            min: self.min + rhs,
            max: self.max + rhs,
        }
    }
}

impl AddAssign<Vec2> for Rect {
    fn add_assign(&mut self, rhs: Vec2) {
        self.min += rhs;
        self.max += rhs;
    }
}

impl Sub<Vec2> for Rect {
    type Output = Self;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            min: self.min - rhs,
            max: self.max - rhs,
        }
    }
}

impl SubAssign<Vec2> for Rect {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.min -= rhs;
        self.max -= rhs;
    }
}

impl Mul<f32> for Rect {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            min: self.min,
            max: self.min + (self.max - self.min) * rhs,
        }
    }
}

impl MulAssign<f32> for Rect {
    fn mul_assign(&mut self, rhs: f32) {
        self.max = self.min + (self.max - self.min) * rhs;
    }
}

impl ToString for Rect {
    fn to_string(&self) -> alloc::string::String {
        alloc::format!("Rect({}, {}, {}, {})", self.min.x, self.min.y, self.max.x, self.max.y)
    }
} 