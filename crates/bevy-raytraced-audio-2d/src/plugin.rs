//! Bevy plugin setup and configuration for this adapter.

use super::settings::RaytracedAudioSettings;
use super::systems::update_raytraced_audio;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::{App, Plugin, PostUpdate, TransformSystems};
use bevy_raytraced_audio::{BandGain, GeometryError};

/// Bevy plugin that traces explicit 2D acoustic surfaces after transform propagation.
#[derive(Clone, Copy, Debug)]
pub struct RaytracedAudio2dPlugin {
    /// Linear sink volume multiplier applied while a direct path is occluded.
    occluded_gain: f32,
}

impl Default for RaytracedAudio2dPlugin {
    /// Uses zero direct-path gain when a surface blocks the source.
    fn default() -> Self {
        Self { occluded_gain: 0.0 }
    }
}

impl RaytracedAudio2dPlugin {
    /// Sets the sink volume multiplier used for an occluded direct path.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError::NonFiniteValue`] for a non-finite gain or
    /// [`GeometryError::CoefficientOutOfRange`] when the gain falls outside `[0, 1]`.
    pub fn with_occluded_gain(mut self, gain: f32) -> Result<Self, GeometryError> {
        BandGain::try_new(gain, gain, gain)?;
        self.occluded_gain = gain;
        Ok(self)
    }
}

impl Plugin for RaytracedAudio2dPlugin {
    /// Registers the post-transform acoustic query system and its configuration.
    fn build(&self, app: &mut App) {
        app.insert_resource(RaytracedAudioSettings {
            occluded_gain: self.occluded_gain,
        })
        .add_systems(
            PostUpdate,
            update_raytraced_audio.after(TransformSystems::Propagate),
        );
    }
}
