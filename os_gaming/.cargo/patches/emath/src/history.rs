use core::option::Option;
use core::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use super::Vec2;

/// A history of values that can be averaged
pub struct History<T> {
    values: alloc::vec::Vec<T>,
    capacity: usize,
}

impl<T> History<T> {
    /// Creates a new history with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            values: alloc::vec::Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Adds a value to the history
    pub fn add(&mut self, value: T) {
        if self.values.len() >= self.capacity {
            self.values.remove(0);
        }
        self.values.push(value);
    }

    /// Clears the history
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Returns the number of values in the history
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the history is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the values in the history
    pub fn values(&self) -> &[T] {
        &self.values
    }
}

impl<T: Clone + Add<Output = T> + Div<f32, Output = T>> History<T> {
    /// Returns the average of the values in the history
    pub fn average(&self) -> Option<T> {
        if self.values.is_empty() {
            return None;
        }

        let sum = self.values.iter().cloned().fold(self.values[0].clone(), |acc, val| acc + val);
        Some(sum / self.values.len() as f32)
    }
}

impl<T: Clone> History<T> {
    /// Returns the most recent value in the history
    pub fn latest(&self) -> Option<&T> {
        self.values.last()
    }
}

impl<T: Clone + Add<Output = T> + Sub<Output = T> + Div<f32, Output = T>> History<T> {
    /// Returns the standard deviation of the values in the history
    pub fn std_dev(&self) -> Option<T> {
        if self.values.len() < 2 {
            return None;
        }

        let avg = self.average().unwrap();
        let sum_sq_diff = self.values.iter().cloned().fold(avg.clone(), |acc, val| {
            let diff = val - avg.clone();
            acc + (diff * diff)
        });
        Some((sum_sq_diff / (self.values.len() - 1) as f32).sqrt())
    }
}

impl<T: Clone + Add<Output = T> + Sub<Output = T> + Div<f32, Output = T>> History<Vec2<T>> {
    /// Returns the average of the vectors in the history
    pub fn average_vec(&self) -> Option<Vec2<T>> {
        if self.values.is_empty() {
            return None;
        }

        let sum_x = self.values.iter().cloned().fold(self.values[0].x.clone(), |acc, val| acc + val.x);
        let sum_y = self.values.iter().cloned().fold(self.values[0].y.clone(), |acc, val| acc + val.y);
        
        Some(Vec2::new(
            sum_x / self.values.len() as f32,
            sum_y / self.values.len() as f32,
        ))
    }
}
