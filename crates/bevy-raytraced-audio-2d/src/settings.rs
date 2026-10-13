//! Private settings shared by the 2d plugin and update system.

use bevy::prelude::Resource;
use core::num::NonZeroUsize;

/// Adapter settings shared by every opted-in source in the application.
#[derive(Resource, Clone, Copy, Debug)]
pub(super) struct RaytracedAudioSettings {
    /// Sink-volume fallback when accumulated direct transmission is zero in every band.
    pub(super) occluded_gain: f32,
    /// Most emitters whose response is recomputed per update.
    ///
    /// `None` refreshes every emitter when the scene has at most 64, and 64 per update after that.
    pub(super) emitter_budget: Option<NonZeroUsize>,
}
