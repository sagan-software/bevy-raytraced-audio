//! Three-band linear amplitude gains used by propagation responses.

use crate::GeometryError;
use crate::geometry_error::validate_coefficient_bands;

/// Linear amplitude transmission in low, mid, and high frequency bands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandGain {
    /// Low-frequency amplitude transmission in the inclusive range `[0, 1]`.
    low: f32,
    /// Mid-frequency amplitude transmission in the inclusive range `[0, 1]`.
    mid: f32,
    /// High-frequency amplitude transmission in the inclusive range `[0, 1]`.
    high: f32,
}

impl BandGain {
    /// No transmitted amplitude in any band.
    pub const ZERO: Self = Self {
        low: 0.0,
        mid: 0.0,
        high: 0.0,
    };

    /// Full transmitted amplitude in every band.
    pub const UNITY: Self = Self {
        low: 1.0,
        mid: 1.0,
        high: 1.0,
    };

    /// Creates finite gains in the inclusive range `[0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for non-finite values or
    /// [`GeometryError::CoefficientOutOfRange`] when a value falls outside `[0, 1]`.
    pub fn try_new(low: f32, mid: f32, high: f32) -> Result<Self, GeometryError> {
        validate_coefficient_bands(low, mid, high)?;
        Ok(Self { low, mid, high })
    }

    /// Returns low-frequency amplitude transmission.
    #[must_use]
    pub const fn low(self) -> f32 {
        self.low
    }

    /// Returns mid-frequency amplitude transmission.
    #[must_use]
    pub const fn mid(self) -> f32 {
        self.mid
    }

    /// Returns high-frequency amplitude transmission.
    #[must_use]
    pub const fn high(self) -> f32 {
        self.high
    }

    /// Multiplies the transmission of sequential path segments.
    #[must_use]
    pub fn multiply(self, other: Self) -> Self {
        Self {
            low: self.low * other.low,
            mid: self.mid * other.mid,
            high: self.high * other.high,
        }
    }
}

impl Default for BandGain {
    /// Returns full transmission in every frequency band.
    fn default() -> Self {
        Self::UNITY
    }
}
