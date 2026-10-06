//! Private state used to restore the user's Bevy sink volume.

use bevy::prelude::Component;

/// Remembers the user's unoccluded volume and last adapter output.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct RaytracedAudioBaseVolume {
    /// Last volume requested outside the acoustic adapter.
    pub(super) base_linear: f32,
    /// Last volume written by the acoustic adapter.
    pub(super) last_output_linear: f32,
}
