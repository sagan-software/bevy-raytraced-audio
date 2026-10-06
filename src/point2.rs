//! Finite two-dimensional coordinates measured in meters.

use crate::GeometryError;

/// A two-dimensional point with meter units and finite coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 {
    /// Horizontal world coordinate in meters.
    x: f32,
    /// Vertical world coordinate in meters.
    y: f32,
}

impl Point2 {
    /// Creates a point after checking that both coordinates are finite.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] when either coordinate is not finite.
    pub const fn try_new(x: f32, y: f32) -> Result<Self, GeometryError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }

        Ok(Self { x, y })
    }

    /// Returns the horizontal coordinate in meters.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the vertical coordinate in meters.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}
