//! Bevy plugin setup and configuration for this adapter.

use super::settings::RaytracedAudioSettings;
use super::systems::update_raytraced_audio;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::{App, Plugin, PostUpdate, TransformSystems};
use bevy_raytraced_audio::{AudioBackendPreference, BandGain, GeometryError};

/// Bevy plugin that traces explicit 3D acoustic surfaces after transform propagation.
#[derive(Clone, Copy, Debug)]
pub struct RaytracedAudio3dPlugin {
    /// Linear sink volume fallback for a direct path with zero transmission in every band.
    occluded_gain: f32,
    /// Preferred propagation backend policy.
    backend_preference: AudioBackendPreference,
}

impl Default for RaytracedAudio3dPlugin {
    /// Uses zero sink-volume fallback for fully opaque direct paths.
    fn default() -> Self {
        Self {
            occluded_gain: 0.0,
            backend_preference: AudioBackendPreference::Cpu,
        }
    }
}

impl RaytracedAudio3dPlugin {
    /// Sets the sink-volume fallback for a direct path with zero transmission in every band.
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

    /// Sets the propagation backend preference used by this plugin.
    ///
    /// CPU tracing is the default. `Auto` currently logs that GPU compute is
    /// unavailable and keeps CPU tracing active because this release has no GPU
    /// backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_raytraced_audio::AudioBackendPreference;
    ///
    /// let preference = AudioBackendPreference::Auto;
    /// assert_eq!(preference, AudioBackendPreference::Auto);
    /// ```
    #[must_use]
    pub const fn with_backend_preference(mut self, preference: AudioBackendPreference) -> Self {
        self.backend_preference = preference;
        self
    }
}

impl Plugin for RaytracedAudio3dPlugin {
    /// Registers the post-transform acoustic query system and its configuration.
    fn build(&self, app: &mut App) {
        if self.backend_preference == AudioBackendPreference::Auto {
            log::warn!("GPU backend is unavailable in this release; using CPU tracing");
        }
        app.insert_resource(RaytracedAudioSettings {
            occluded_gain: self.occluded_gain,
        })
        .add_systems(
            PostUpdate,
            update_raytraced_audio.after(TransformSystems::Propagate),
        );
    }
}
