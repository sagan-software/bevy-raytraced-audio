//! A sound source position in three-dimensional world space.

use crate::{GeometryError, Point3};

/// A 3D sound source location with an optional clearance radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emitter3d {
    /// Source position in meters.
    position: Point3,
    /// Radius around the source, in meters, within which surfaces are ignored.
    clearance_m: f32,
}

impl Emitter3d {
    /// Creates a source at the supplied position with no clearance.
    #[must_use]
    pub const fn new(position: Point3) -> Self {
        Self {
            position,
            clearance_m: 0.0,
        }
    }

    /// Ignores surfaces closer to the source than `clearance_m` meters.
    ///
    /// Use this for a source placed inside or flush against geometry, such as a
    /// sound attached inside a prop. A surface crossing on the direct path or on
    /// the source-to-reflection-point leg at or within the clearance neither
    /// attenuates nor blocks the path. A reflector whose reflection point lies at
    /// or within the clearance forms no reflection path. Surfaces beyond the
    /// clearance behave as without it. The default clearance of `0` ignores nothing.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite clearance or
    /// [`GeometryError::CoefficientOutOfRange`] for a negative clearance.
    pub fn try_with_clearance(mut self, clearance_m: f32) -> Result<Self, GeometryError> {
        if !clearance_m.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }
        if clearance_m < 0.0 {
            return Err(GeometryError::CoefficientOutOfRange);
        }
        // Negative zero is stored as positive zero so equal clearances compare equal.
        self.clearance_m = clearance_m + 0.0;
        Ok(self)
    }

    /// Returns the source position in meters.
    #[must_use]
    pub const fn position(self) -> Point3 {
        self.position
    }

    /// Returns the clearance radius in meters; `0` when none was set.
    #[must_use]
    pub const fn clearance_m(self) -> f32 {
        self.clearance_m
    }
}
