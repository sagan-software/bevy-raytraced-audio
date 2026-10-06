//! First-order reflected paths in the XY plane.

use crate::{BandAbsorption, BandEnergy, ReflectionSurfaceIndex, SolverPoint2d};

/// One visible first-order reflection returned by a two-dimensional scene query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflectionPath2d {
    /// Scene-local index of the reflecting segment.
    surface_index: ReflectionSurfaceIndex,
    /// Point where the image-source ray reaches the finite reflecting segment.
    reflection_point: SolverPoint2d,
    /// Mirrored source position whose direct image ray forms this reflection.
    image_source: SolverPoint2d,
    /// Total source-to-reflector-to-listener path distance in meters.
    distance_m: f64,
    /// Squared direct-to-reflected distance ratio used by energy accumulation.
    distance_ratio_squared: f64,
    /// Absorbed energy fraction for the reflecting segment in each frequency band.
    absorption: BandAbsorption,
}

impl ReflectionPath2d {
    /// Returns the surface index assigned during insertion into the queried scene.
    #[must_use]
    pub const fn surface_index(self) -> ReflectionSurfaceIndex {
        self.surface_index
    }

    /// Returns the point where the first-order path reflects from its finite segment.
    #[must_use]
    pub const fn reflection_point(self) -> SolverPoint2d {
        self.reflection_point
    }

    /// Returns the virtual source position mirrored across the reflecting segment.
    #[must_use]
    pub const fn image_source(self) -> SolverPoint2d {
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
        let low = self.distance_ratio_squared * (1.0 - f64::from(self.absorption.low()));
        let mid = self.distance_ratio_squared * (1.0 - f64::from(self.absorption.mid()));
        let high = self.distance_ratio_squared * (1.0 - f64::from(self.absorption.high()));
        BandEnergy::from_solver(low, mid, high)
    }

    /// Returns the distance ratio and absorption used to preserve fused aggregate accumulation.
    pub(crate) const fn accumulation_terms(self) -> (f64, BandAbsorption) {
        (self.distance_ratio_squared, self.absorption)
    }

    /// Creates a path from geometry and validated scene material values.
    pub(crate) const fn new(
        surface_index: ReflectionSurfaceIndex,
        reflection_point: SolverPoint2d,
        image_source: SolverPoint2d,
        distance_m: f64,
        distance_ratio_squared: f64,
        absorption: BandAbsorption,
    ) -> Self {
        Self {
            surface_index,
            reflection_point,
            image_source,
            distance_m,
            distance_ratio_squared,
            absorption,
        }
    }
}
