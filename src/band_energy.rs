//! Aggregate non-negative reflected energy in low, mid, and high bands.

use crate::GeometryError;

/// Aggregate reflected energy relative to one unobstructed direct path.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BandEnergy {
    /// Low-frequency reflected energy.
    low: f64,
    /// Mid-frequency reflected energy.
    mid: f64,
    /// High-frequency reflected energy.
    high: f64,
}

impl BandEnergy {
    /// An aggregate with no reflected energy.
    pub const ZERO: Self = Self {
        low: 0.0,
        mid: 0.0,
        high: 0.0,
    };

    /// Constructs solver output whose non-negative finite invariant follows from path bounds.
    pub(crate) const fn from_solver(low: f64, mid: f64, high: f64) -> Self {
        Self { low, mid, high }
    }

    /// Creates finite non-negative band energy values.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for non-finite values or
    /// [`GeometryError::NegativeEnergy`] when any band is negative.
    pub fn try_new(low: f64, mid: f64, high: f64) -> Result<Self, GeometryError> {
        if !low.is_finite() || !mid.is_finite() || !high.is_finite() {
            return Err(GeometryError::NonFiniteValue);
        }

        if low < 0.0 || mid < 0.0 || high < 0.0 {
            return Err(GeometryError::NegativeEnergy);
        }

        Ok(Self { low, mid, high })
    }

    /// Returns reflected energy in the low-frequency band.
    #[must_use]
    pub const fn low(self) -> f64 {
        self.low
    }

    /// Returns reflected energy in the mid-frequency band.
    #[must_use]
    pub const fn mid(self) -> f64 {
        self.mid
    }

    /// Returns reflected energy in the high-frequency band.
    #[must_use]
    pub const fn high(self) -> f64 {
        self.high
    }
}
