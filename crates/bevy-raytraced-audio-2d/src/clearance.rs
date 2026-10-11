//! The optional source clearance component for 2D emitters.

use bevy::prelude::Component;
use bevy_raytraced_audio::{Emitter2d, GeometryError, Point2};

/// Ignores surfaces within a radius of an emitter, for sources placed inside or against geometry.
///
/// Add it next to [`super::RaytracedAudioEmitter2d`]. Emitters without it use no clearance.
/// See [`Emitter2d::try_with_clearance`] for the propagation rules.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RaytracedAudioClearance2d {
    /// Validated clearance radius in meters.
    clearance_m: f32,
}

impl RaytracedAudioClearance2d {
    /// Creates a clearance of `clearance_m` meters around the emitter.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Emitter2d::try_with_clearance`].
    pub fn try_new(clearance_m: f32) -> Result<Self, GeometryError> {
        let origin = Point2::try_new(0.0, 0.0)?;
        let clearance_m = Emitter2d::new(origin)
            .try_with_clearance(clearance_m)?
            .clearance_m();
        Ok(Self { clearance_m })
    }

    /// Returns the clearance radius in meters.
    #[must_use]
    pub const fn clearance_m(self) -> f32 {
        self.clearance_m
    }

    /// Builds the core emitter at `position` with this optional clearance.
    pub(super) fn emitter(clearance: Option<&Self>, position: Point2) -> Emitter2d {
        let emitter = Emitter2d::new(position);
        // The component holds a validated value, so the fallback is never taken.
        clearance.map_or(emitter, |clearance| {
            emitter
                .try_with_clearance(clearance.clearance_m)
                .unwrap_or(emitter)
        })
    }
}
