//! The marked 2d emitter component.

use bevy::prelude::Component;

/// Marks an audio entity as an opted-in 2D acoustic source.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RaytracedAudioEmitter2d;
