//! The marked 3d emitter component.

use bevy::prelude::Component;

/// Marks an audio entity as an opted-in 3D acoustic source.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RaytracedAudioEmitter3d;
