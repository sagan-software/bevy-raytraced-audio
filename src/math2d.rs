//! Double-precision 2D math for robust scene intersection calculations.

use crate::Point2;

/// A two-dimensional vector used internally by the CPU solver.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vector2 {
    /// Horizontal vector component.
    pub(crate) x: f64,
    /// Vertical vector component.
    pub(crate) y: f64,
}

impl Vector2 {
    /// Converts a meter point to double precision for geometry work.
    pub(crate) fn from_point(point: Point2) -> Self {
        Self {
            x: f64::from(point.x()),
            y: f64::from(point.y()),
        }
    }

    /// Subtracts another vector from this vector.
    pub(crate) fn subtract(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    /// Adds another vector to this vector.
    pub(crate) fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// Scales both vector components by one finite factor.
    pub(crate) fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
        }
    }

    /// Returns the dot product with another vector.
    pub(crate) fn dot(self, other: Self) -> f64 {
        self.y.mul_add(other.y, self.x * other.x)
    }

    /// Returns the scalar 2D cross product with another vector.
    pub(crate) fn cross(self, other: Self) -> f64 {
        self.y.mul_add(-other.x, self.x * other.y)
    }

    /// Returns Euclidean vector length in meters.
    pub(crate) fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
}
