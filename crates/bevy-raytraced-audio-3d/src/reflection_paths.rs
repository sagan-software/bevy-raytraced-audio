//! Opt-in per-emitter output containing the latest 3D reflection paths.

use bevy::prelude::Component;
use bevy_raytraced_audio::ReflectionPath3d;

/// Stores first-order reflection paths for one emitter when explicitly attached.
#[derive(Component, Debug, Default)]
pub struct RaytracedAudioReflectionPaths3d {
    /// Reused path storage populated by the 3D acoustic adapter each frame.
    pub(super) paths: Vec<ReflectionPath3d>,
}

impl RaytracedAudioReflectionPaths3d {
    /// Returns this emitter's current reflection paths in surface insertion order.
    #[must_use]
    pub fn paths(&self) -> &[ReflectionPath3d] {
        &self.paths
    }
}
