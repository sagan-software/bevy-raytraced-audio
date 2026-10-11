//! The optional source clearance component for 3D emitters.

use bevy::prelude::Component;
use bevy_raytraced_audio::{Emitter3d, GeometryError, Point3};

/// Ignores surfaces within a radius of an emitter, for sources placed inside or against geometry.
///
/// Add it next to [`super::RaytracedAudioEmitter3d`]. Emitters without it use no clearance.
/// See [`Emitter3d::try_with_clearance`] for the propagation rules.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct RaytracedAudioClearance3d {
    /// Validated clearance radius in meters.
    clearance_m: f32,
}

impl RaytracedAudioClearance3d {
    /// Creates a clearance of `clearance_m` meters around the emitter.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Emitter3d::try_with_clearance`].
    pub fn try_new(clearance_m: f32) -> Result<Self, GeometryError> {
        let origin = Point3::try_new(0.0, 0.0, 0.0)?;
        let clearance_m = Emitter3d::new(origin)
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
    pub(super) fn emitter(clearance: Option<&Self>, position: Point3) -> Emitter3d {
        let emitter = Emitter3d::new(position);
        // The component holds a validated value, so the fallback is never taken.
        clearance.map_or(emitter, |clearance| {
            emitter
                .try_with_clearance(clearance.clearance_m)
                .unwrap_or(emitter)
        })
    }
}
