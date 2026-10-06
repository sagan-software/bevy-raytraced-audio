//! Per-band energy absorption and amplitude transmission for acoustic surfaces.

use crate::{BandAbsorption, BandEnergy, BandGain, GeometryError};

/// Frequency-dependent surface energy absorption and direct-path amplitude transmission.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcousticMaterial {
    /// Absorbed energy fraction for each of the three bands.
    absorption: BandAbsorption,
    /// Transmitted amplitude fraction for each of the three bands.
    transmission: BandGain,
}

impl AcousticMaterial {
    /// Creates explicit absorption coefficients and leaves direct transmission at zero.
    #[must_use]
    pub const fn new(absorption: BandAbsorption) -> Self {
        Self {
            absorption,
            transmission: BandGain::ZERO,
        }
    }

    /// Sets per-band amplitude transmission after validating the material energy budget.
    ///
    /// For each band, absorbed energy fraction plus squared amplitude transmission must be at
    /// most `1.0`. Any remaining incident energy contributes to reflected energy.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::MaterialEnergyExceedsIncidentEnergy`] when any band exceeds the
    /// incident energy budget.
    pub fn try_with_transmission(mut self, transmission: BandGain) -> Result<Self, GeometryError> {
        // Both operands are dimensionless fractions; squaring amplitude converts it to an energy fraction.
        for (absorption, transmission) in [
            (self.absorption.low(), transmission.low()),
            (self.absorption.mid(), transmission.mid()),
            (self.absorption.high(), transmission.high()),
        ] {
            let transmission_amplitude_fraction = f64::from(transmission);
            let absorbed_energy_fraction = f64::from(absorption);
            let is_full_transmission = transmission.to_bits() == 1.0_f32.to_bits();
            let total_energy_fraction = transmission_amplitude_fraction
                .mul_add(transmission_amplitude_fraction, absorbed_energy_fraction);
            // If transmission is unity, reject every positive absorption value before rounded addition hides it.
            if total_energy_fraction > 1.0
                || (is_full_transmission && absorbed_energy_fraction > 0.0)
            {
                return Err(GeometryError::MaterialEnergyExceedsIncidentEnergy);
            }
        }

        self.transmission = transmission;
        Ok(self)
    }

    /// Returns the material's three-band absorption coefficients.
    #[must_use]
    pub const fn absorption(self) -> BandAbsorption {
        self.absorption
    }

    /// Returns the per-band amplitude transmitted through this surface.
    #[must_use]
    pub const fn transmission(self) -> BandGain {
        self.transmission
    }

    /// Returns each band's incident-energy fraction available for reflection.
    pub(crate) fn reflected_fraction(self) -> BandEnergy {
        // Transmission amplitudes are dimensionless; reflected energy subtracts their squares.
        let low_transmission = f64::from(self.transmission.low());
        let mid_transmission = f64::from(self.transmission.mid());
        let high_transmission = f64::from(self.transmission.high());
        // Each amplitude fraction is squared into energy with one fused operation.
        let low =
            low_transmission.mul_add(-low_transmission, 1.0 - f64::from(self.absorption.low()));
        let mid =
            mid_transmission.mul_add(-mid_transmission, 1.0 - f64::from(self.absorption.mid()));
        let high =
            high_transmission.mul_add(-high_transmission, 1.0 - f64::from(self.absorption.high()));
        BandEnergy::from_solver(low, mid, high)
    }
}

impl Default for AcousticMaterial {
    /// Returns a zero-absorption, non-transmitting surface material.
    fn default() -> Self {
        Self {
            absorption: BandAbsorption::default(),
            transmission: BandGain::ZERO,
        }
    }
}
