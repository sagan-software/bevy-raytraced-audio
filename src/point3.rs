//! Finite three-dimensional coordinates measured in meters.

use crate::GeometryError;

/// A three-dimensional point with meter units and finite coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3 {
    /// X world coordinate in meters.
    x: f32,
    /// Y world coordinate in meters.
    y: f32,
    /// Z world coordinate in meters.
    z: f32,
}

impl Point3 {
    /// Creates a point after checking that all coordinates are finite.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] when any coordinate is not finite.
    pub const fn try_new(x: f32, y: f32, z: f32) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }

        Ok(Self { x, y, z })
    }

    /// Returns the X coordinate in meters.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the Y coordinate in meters.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }

    /// Returns the Z coordinate in meters.
    #[must_use]
    pub const fn z(self) -> f32 {
        self.z
    }
}
