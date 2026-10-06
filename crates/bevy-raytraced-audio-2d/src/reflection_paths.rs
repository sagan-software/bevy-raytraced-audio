//! Opt-in per-emitter output containing the latest 2D reflection paths.

use bevy::prelude::Component;
use bevy_raytraced_audio::ReflectionPath2d;

/// Stores first-order reflection paths for one emitter when explicitly attached.
#[derive(Component, Debug, Default)]
pub struct RaytracedAudioReflectionPaths2d {
    /// Reused path storage populated by the 2D acoustic adapter each frame.
    pub(super) paths: Vec<ReflectionPath2d>,
}

impl RaytracedAudioReflectionPaths2d {
    /// Returns this emitter's current reflection paths in surface insertion order.
    #[must_use]
    pub fn paths(&self) -> &[ReflectionPath2d] {
        &self.paths
    }
}
