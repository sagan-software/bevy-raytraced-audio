//! Private settings shared by the 3d plugin and update system.

use bevy::prelude::Resource;

/// Adapter settings shared by every opted-in source in the application.
#[derive(Resource, Clone, Copy, Debug)]
pub(super) struct RaytracedAudioSettings {
    /// Direct-path gain used when geometry blocks a source.
    pub(super) occluded_gain: f32,
}
