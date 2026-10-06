//! Direct and reflected propagation values returned by a scene query.

use crate::{BandEnergy, BandGain};

/// Distance, obstruction, and transmission for one source-to-listener path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathResponse {
    /// Geometric path distance in meters.
    distance_m: f64,
    /// Obstruction and material transmission; excludes distance attenuation.
    gain: BandGain,
    /// Whether any opaque scene primitive blocks the path.
    occluded: bool,
}

impl PathResponse {
    /// Returns the geometric path distance in meters.
    #[must_use]
    pub const fn distance_m(self) -> f64 {
        self.distance_m
    }

    /// Returns per-band transmission, excluding distance attenuation.
    #[must_use]
    pub const fn gain(self) -> BandGain {
        self.gain
    }

    /// Returns whether the direct path intersects an opaque primitive.
    #[must_use]
    pub const fn is_occluded(self) -> bool {
        self.occluded
    }
}

/// Direct-path and aggregate first-order reflection results.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcousticResponse {
    /// Direct path from the source to the listener.
    pub direct: PathResponse,
    /// Reflected energy relative to an unobstructed direct path.
    pub reflected_energy: BandEnergy,
}

impl AcousticResponse {
    /// Builds a response from the solver's computed path values.
    pub(crate) const fn new(direct: PathResponse, reflected_energy: BandEnergy) -> Self {
        Self {
            direct,
            reflected_energy,
        }
    }
}

impl PathResponse {
    /// Builds a path result from validated internal geometry values.
    pub(crate) const fn new(distance_m: f64, gain: BandGain, occluded: bool) -> Self {
        Self {
            distance_m,
            gain,
            occluded,
        }
    }
}
