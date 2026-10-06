//! Double-precision 3D math for robust scene intersection calculations.

use crate::Point3;

/// A three-dimensional vector used internally by the CPU solver.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vector3 {
    /// X vector component.
    pub(crate) x: f64,
    /// Y vector component.
    pub(crate) y: f64,
    /// Z vector component.
    pub(crate) z: f64,
}

impl Vector3 {
    /// Converts a meter point to double precision for geometry work.
    pub(crate) fn from_point(point: Point3) -> Self {
        Self {
            x: f64::from(point.x()),
            y: f64::from(point.y()),
            z: f64::from(point.z()),
        }
    }

    /// Subtracts another vector from this vector.
    pub(crate) fn subtract(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    /// Adds another vector to this vector.
    pub(crate) fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    /// Scales all vector components by one finite factor.
    pub(crate) fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    /// Returns the dot product with another vector.
    pub(crate) fn dot(self, other: Self) -> f64 {
        self.z
            .mul_add(other.z, self.y.mul_add(other.y, self.x * other.x))
    }

    /// Returns the vector cross product with another vector.
    pub(crate) fn cross(self, other: Self) -> Self {
        Self {
            x: self.z.mul_add(-other.y, self.y * other.z),
            y: self.x.mul_add(-other.z, self.z * other.x),
            z: self.y.mul_add(-other.x, self.x * other.y),
        }
    }

    /// Returns Euclidean vector length in meters.
    pub(crate) fn length(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }
}
