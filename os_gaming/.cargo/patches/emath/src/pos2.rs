use core::ops::{Add, AddAssign, Sub, SubAssign};
use super::{Vec2, ToVec2, ToString};

/// A 2D position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
}

impl Pos2 {
    /// Create a new position
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a position at the origin
    pub const fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Get the distance to another position
    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get the squared distance to another position
    pub fn distance_sq_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Get the vector from this position to another position
    pub fn to_vec2(&self, other: &Self) -> Vec2 {
        Vec2::new(other.x - self.x, other.y - self.y)
    }
}

impl Add<Vec2> for Pos2 {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign<Vec2> for Pos2 {
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub<Vec2> for Pos2 {
    type Output = Self;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign<Vec2> for Pos2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Sub for Pos2 {
    type Output = Vec2;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl ToVec2 for Pos2 {
    fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl ToString for Pos2 {
    fn to_string(&self) -> alloc::string::String {
        alloc::format!("Pos2({}, {})", self.x, self.y)
    }
} 