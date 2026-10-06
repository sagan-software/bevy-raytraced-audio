//! Three-band energy absorption coefficients for acoustic materials.

use crate::GeometryError;
use crate::geometry_error::validate_coefficient_bands;

/// Fraction of incident acoustic energy absorbed in low, mid, and high bands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandAbsorption {
    /// Low-frequency energy absorption in the inclusive range `[0, 1]`.
    low: f32,
    /// Mid-frequency energy absorption in the inclusive range `[0, 1]`.
    mid: f32,
    /// High-frequency energy absorption in the inclusive range `[0, 1]`.
    high: f32,
}

impl BandAbsorption {
    /// Creates absorption coefficients after checking finiteness and bounds.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for non-finite values or
    /// [`GeometryError::CoefficientOutOfRange`] when a value falls outside `[0, 1]`.
    pub fn try_new(low: f32, mid: f32, high: f32) -> Result<Self, GeometryError> {
        validate_coefficient_bands(low, mid, high)?;
        Ok(Self { low, mid, high })
    }

    /// Returns the absorption coefficient for the low-frequency band.
    #[must_use]
    pub const fn low(self) -> f32 {
        self.low
    }

    /// Returns the absorption coefficient for the mid-frequency band.
    #[must_use]
    pub const fn mid(self) -> f32 {
        self.mid
    }

    /// Returns the absorption coefficient for the high-frequency band.
    #[must_use]
    pub const fn high(self) -> f32 {
        self.high
    }
}

impl Default for BandAbsorption {
    /// Returns a fully reflective material profile.
    fn default() -> Self {
        Self {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }
}
