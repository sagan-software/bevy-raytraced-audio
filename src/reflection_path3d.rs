//! First-order reflected paths in three-dimensional space.

use crate::{BandEnergy, ReflectionSurfaceIndex, SolverPoint3d};

/// One visible first-order reflection returned by a three-dimensional scene query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflectionPath3d {
    /// Scene-local index of the reflecting triangle.
    surface_index: ReflectionSurfaceIndex,
    /// Point where the image-source ray reaches the finite reflecting triangle.
    reflection_point: SolverPoint3d,
    /// Mirrored source position whose direct image ray forms this reflection.
    image_source: SolverPoint3d,
    /// Total source-to-reflector-to-listener path distance in meters.
    distance_m: f64,
    /// Squared direct-to-reflected distance ratio used by energy accumulation.
    distance_ratio_squared: f64,
    /// Incident-energy fraction available for reflection in each band.
    reflected_fraction: BandEnergy,
}

impl ReflectionPath3d {
    /// Returns the surface index assigned during insertion into the queried scene.
    #[must_use]
    pub const fn surface_index(self) -> ReflectionSurfaceIndex {
        self.surface_index
    }

    /// Returns the point where the first-order path reflects from its finite triangle.
    #[must_use]
    pub const fn reflection_point(self) -> SolverPoint3d {
        self.reflection_point
    }

    /// Returns the virtual source position mirrored across the reflecting triangle.
    #[must_use]
    pub const fn image_source(self) -> SolverPoint3d {
        self.image_source
    }

    /// Returns the total reflected path distance from emitter to listener in meters.
    #[must_use]
    pub const fn distance_m(self) -> f64 {
        self.distance_m
    }

    /// Returns per-band reflected energy relative to an unobstructed direct path.
    #[must_use]
    pub fn relative_energy(self) -> BandEnergy {
        // A unitless squared distance ratio scales each band's unitless reflected-energy fraction.
        let low = self.distance_ratio_squared * self.reflected_fraction.low();
        let mid = self.distance_ratio_squared * self.reflected_fraction.mid();
        let high = self.distance_ratio_squared * self.reflected_fraction.high();
        BandEnergy::from_solver(low, mid, high)
    }

    /// Returns the distance ratio and material reflectivity for aggregate accumulation.
    pub(crate) const fn accumulation_terms(self) -> (f64, BandEnergy) {
        (self.distance_ratio_squared, self.reflected_fraction)
    }

    /// Creates a path from geometry and validated scene material values.
    pub(crate) const fn new(
        surface_index: ReflectionSurfaceIndex,
        reflection_point: SolverPoint3d,
        image_source: SolverPoint3d,
        distance_m: f64,
        distance_ratio_squared: f64,
        reflected_fraction: BandEnergy,
    ) -> Self {
        Self {
            surface_index,
            reflection_point,
            image_source,
            distance_m,
            distance_ratio_squared,
            reflected_fraction,
        }
    }
}
