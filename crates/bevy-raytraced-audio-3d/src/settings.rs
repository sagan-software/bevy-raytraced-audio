//! Private settings shared by the 3d plugin and update system.

use bevy::prelude::Resource;

/// Adapter settings shared by every opted-in source in the application.
#[derive(Resource, Clone, Copy, Debug)]
pub(super) struct RaytracedAudioSettings {
    /// Sink-volume fallback when accumulated direct transmission is zero in every band.
    pub(super) occluded_gain: f32,
}
