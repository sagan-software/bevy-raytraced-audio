//! The unique listener marker used by the 3d adapter.

use bevy::prelude::Component;

/// Marks the unique 3D listener used by this adapter.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RaytracedAudioListener3d;
