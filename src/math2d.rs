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
    #[expect(
        clippy::suboptimal_flops,
        reason = "avoids software FMA dispatch on generic native and WebAssembly targets"
    )]
    pub(crate) fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Returns the scalar 2D cross product with another vector.
    #[expect(
        clippy::suboptimal_flops,
        reason = "intersection tolerances exceed rounding error; avoid software FMA dispatch"
    )]
    pub(crate) fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Returns Euclidean vector length in meters.
    pub(crate) fn length(self) -> f64 {
        let squared = self.dot(self);
        if squared.is_normal() {
            squared.sqrt()
        } else {
            // Preserve hypot's overflow/underflow behavior for extreme public coordinates.
            self.x.hypot(self.y)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Reference-norm regressions for extreme finite vectors.

    use super::Vector2;
    /// The fast norm agrees with scaled hypot, including overflow/underflow fallbacks.
    #[test]
    fn norm_preserves_extreme_ranges() {
        for scale in [
            0., 1.0e-300, 1.0e-160, 1.0e-30, 1., 1.0e30, 1.0e160, 1.0e300,
        ] {
            let v = Vector2 {
                x: 3. * scale,
                y: -4. * scale,
            };
            let reference = v.x.hypot(v.y);
            assert!((v.length() - reference).abs() <= reference * 4. * f64::EPSILON);
        }
    }
}
