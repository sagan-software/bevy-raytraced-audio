//! Material absorption used when a traced path reaches a surface.

use crate::BandAbsorption;

/// Frequency-dependent surface absorption for direct and reflected paths.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AcousticMaterial {
    /// Absorbed energy fraction for each of the three bands.
    absorption: BandAbsorption,
}

impl AcousticMaterial {
    /// Creates a material with explicit low, mid, and high absorption.
    #[must_use]
    pub const fn new(absorption: BandAbsorption) -> Self {
        Self { absorption }
    }

    /// Returns the material's three-band absorption coefficients.
    #[must_use]
    pub const fn absorption(self) -> BandAbsorption {
        self.absorption
    }
}
