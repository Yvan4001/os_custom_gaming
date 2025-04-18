use core::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign};
use super::{Rotate, Scale, Lerp, Clamp, ApproxEq, ToString};

/// A 2D vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Create a new vector
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a zero vector
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Create a unit vector in the x direction
    pub const fn unit_x() -> Self {
        Self { x: 1.0, y: 0.0 }
    }

    /// Create a unit vector in the y direction
    pub const fn unit_y() -> Self {
        Self { x: 0.0, y: 1.0 }
    }

    /// Get the length of the vector
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Get the squared length of the vector
    pub fn length_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Normalize the vector
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            *self
        }
    }

    /// Get the dot product with another vector
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Get the cross product with another vector
    pub fn cross(&self, other: &Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// Rotate the vector by an angle in radians
    pub fn rotate(&self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Rotate for Vec2 {
    fn rotate(&self, angle: f32) -> Self {
        self.rotate(angle)
    }
}

impl Scale for Vec2 {
    fn scale(&self, factor: f32) -> Self {
        *self * factor
    }
}

impl Lerp for Vec2 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl Clamp for Vec2 {
    fn clamp(&self, min: &Self, max: &Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
        }
    }
}

impl ApproxEq for Vec2 {
    fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        (self.x - other.x).abs() <= epsilon && (self.y - other.y).abs() <= epsilon
    }
}

impl ToString for Vec2 {
    fn to_string(&self) -> alloc::string::String {
        alloc::format!("Vec2({}, {})", self.x, self.y)
    }
} 