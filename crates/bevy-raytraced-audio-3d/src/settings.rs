//! Private settings shared by the 3d plugin and update system.

use bevy::prelude::{Resource, Vec3};

/// Adapter settings shared by every opted-in source in the application.
#[derive(Resource, Clone, Copy, Debug)]
pub(super) struct RaytracedAudioSettings {
    /// Sink-volume fallback when accumulated direct transmission is zero in every band.
    pub(super) occluded_gain: f32,
    /// Bevy world units per meter; positions are divided by it before tracing.
    pub(super) units_per_meter: f32,
    /// Whether the adapter writes Bevy sink volumes; otherwise it only publishes responses.
    pub(super) controls_sink_volume: bool,
    /// Whether per-emitter responses include first-order reflections.
    pub(super) reflections: bool,
}

impl RaytracedAudioSettings {
    /// Converts a Bevy world position to meters.
    pub(super) fn to_meters(self, world: Vec3) -> Vec3 {
        world / self.units_per_meter
    }
}
