//! Direct and reflected propagation values returned by a scene query.

use crate::{BandEnergy, BandGain};

/// Distance, direct transmission, and obstruction for one source-to-listener path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathResponse {
    /// Geometric path distance in meters.
    distance_m: f64,
    /// Product of crossed-surface amplitude transmission by band; excludes distance attenuation.
    gain: BandGain,
    /// Whether any crossed scene surface attenuates at least one band.
    occluded: bool,
}

impl PathResponse {
    /// Returns the geometric path distance in meters.
    #[must_use]
    pub const fn distance_m(self) -> f64 {
        self.distance_m
    }

    /// Returns direct-path amplitude transmission by band, excluding distance attenuation.
    #[must_use]
    pub const fn gain(self) -> BandGain {
        self.gain
    }

    /// Returns whether a crossed surface attenuates at least one transmission band.
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
    /// Replaces direct transmission while preserving CPU-computed distance and reflections.
    ///
    /// GPU backends can use this method after validating a direct-path gain. Occlusion derives
    /// from whether any band differs from unity because each material transmission is in `[0, 1]`.
    #[must_use]
    pub fn with_direct_gain(mut self, gain: BandGain) -> Self {
        self.direct = self.direct.with_gain(gain);
        self
    }

    /// Builds a response from the solver's computed path values.
    pub(crate) const fn new(direct: PathResponse, reflected_energy: BandEnergy) -> Self {
        Self {
            direct,
            reflected_energy,
        }
    }
}

impl PathResponse {
    /// Keeps the traced distance and derives occlusion from validated band transmission.
    fn with_gain(self, gain: BandGain) -> Self {
        Self {
            distance_m: self.distance_m,
            gain,
            occluded: gain != BandGain::UNITY,
        }
    }

    /// Builds a path result from validated internal geometry values.
    pub(crate) const fn new(distance_m: f64, gain: BandGain, occluded: bool) -> Self {
        Self {
            distance_m,
            gain,
            occluded,
        }
    }
}
