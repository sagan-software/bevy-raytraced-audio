//! Validation failures for coordinates, coefficients, and scene geometry.

use std::error::Error;
use std::fmt::{Display, Formatter};

/// An invalid value supplied to the acoustic geometry API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    /// A coordinate or coefficient is NaN or infinite.
    NonFiniteValue,
    /// A gain or absorption coefficient is outside its supported range.
    CoefficientOutOfRange,
    /// An aggregate reflected-energy value is negative.
    NegativeEnergy,
    /// A line segment has zero length or a triangle has zero area.
    DegeneratePrimitive,
}

impl Display for GeometryError {
    /// Formats a stable, concise validation diagnostic.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NonFiniteValue => "value must be finite",
            Self::CoefficientOutOfRange => "coefficient must be in the inclusive range [0, 1]",
            Self::NegativeEnergy => "reflected energy must be non-negative",
            Self::DegeneratePrimitive => "acoustic geometry primitive must have nonzero measure",
        };
        formatter.write_str(message)
    }
}

impl Error for GeometryError {}

/// Checks that each absorption or gain coefficient is finite and bounded.
pub(crate) fn validate_coefficient_bands(
    low: f32,
    mid: f32,
    high: f32,
) -> Result<(), GeometryError> {
    if !low.is_finite() || !mid.is_finite() || !high.is_finite() {
        return Err(GeometryError::NonFiniteValue);
    }

    if !(0.0..=1.0).contains(&low) || !(0.0..=1.0).contains(&mid) || !(0.0..=1.0).contains(&high) {
        return Err(GeometryError::CoefficientOutOfRange);
    }

    Ok(())
}
