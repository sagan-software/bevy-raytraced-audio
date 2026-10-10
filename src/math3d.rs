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
    #[expect(
        clippy::suboptimal_flops,
        reason = "avoids software FMA dispatch on generic native and WebAssembly targets"
    )]
    pub(crate) fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Returns the vector cross product with another vector.
    #[expect(
        clippy::suboptimal_flops,
        reason = "intersection tolerances exceed rounding error; avoid software FMA dispatch"
    )]
    pub(crate) fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Returns Euclidean vector length in meters.
    pub(crate) fn length(self) -> f64 {
        let squared = self.dot(self);
        if squared.is_normal() {
            squared.sqrt()
        } else {
            self.x.hypot(self.y).hypot(self.z)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Reference-norm regressions for extreme finite vectors.

    use super::Vector3;
    /// Finite large and tiny vectors retain the range guarantees of the reference norm.
    #[test]
    fn norm_preserves_extreme_ranges() {
        for scale in [
            0., 1.0e-300, 1.0e-160, 1.0e-30, 1., 1.0e30, 1.0e160, 1.0e300,
        ] {
            let v = Vector3 {
                x: 2. * scale,
                y: -3. * scale,
                z: 6. * scale,
            };
            let reference = v.x.hypot(v.y).hypot(v.z);
            assert!((v.length() - reference).abs() <= reference * 4. * f64::EPSILON);
        }
    }
}
