//! The unique listener marker used by the 2d adapter.

use bevy::prelude::Component;

/// Marks the unique 2D listener used by this adapter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RaytracedAudioListener2d;
