//! Bevy plugin setup and configuration for this adapter.

use super::ray_tracing::{RaytracedAudioListenerTrace3d, RaytracedAudioTracing3d};
use super::settings::RaytracedAudioSettings;
use super::systems::update_raytraced_audio;
use crate::processed_audio::{RaytracedAudioPrepareSystems, RaytracedAudioProcessingPlugin};
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::{App, Plugin, PostUpdate, TransformSystems};
use bevy_raytraced_audio::{AudioBackendPreference, BandGain, GeometryError, RayTraceSettings};

/// Bevy plugin that traces explicit 3D acoustic surfaces after transform propagation.
#[derive(Clone, Copy, Debug)]
pub struct RaytracedAudio3dPlugin {
    /// Linear sink volume fallback for a direct path with zero transmission in every band.
    occluded_gain: f32,
    /// Preferred propagation backend policy.
    backend_preference: AudioBackendPreference,
    /// Initial listener ray-trace configuration.
    tracing: RaytracedAudioTracing3d,
}

impl Default for RaytracedAudio3dPlugin {
    /// Uses zero sink-volume fallback for fully opaque direct paths.
    fn default() -> Self {
        Self {
            occluded_gain: 0.0,
            backend_preference: AudioBackendPreference::Cpu,
            tracing: RaytracedAudioTracing3d::default(),
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

impl RaytracedAudio3dPlugin {
    /// Sets the initial listener ray-trace settings.
    #[must_use]
    pub const fn with_ray_tracing(mut self, settings: RayTraceSettings) -> Self {
        self.tracing.settings = settings;
        self.tracing.enabled = true;
        self
    }

    /// Disables the listener ray trace so sinks follow direct-path transmission only.
    #[must_use]
    pub const fn without_ray_tracing(mut self) -> Self {
        self.tracing.enabled = false;
        self
    }
}

impl Plugin for RaytracedAudio3dPlugin {
    /// Registers the post-transform acoustic query system and its configuration.
    fn build(&self, app: &mut App) {
        if self.backend_preference == AudioBackendPreference::Auto {
            log::warn!("GPU backend is unavailable in this release; using CPU tracing");
        }
        // Processed playback needs Bevy's asset server; headless apps can skip it.
        let has_asset_server = app.world().contains_resource::<bevy::asset::AssetServer>();
        if has_asset_server && !app.is_plugin_added::<RaytracedAudioProcessingPlugin>() {
            app.add_plugins(RaytracedAudioProcessingPlugin);
        }
        app.insert_resource(RaytracedAudioSettings {
            occluded_gain: self.occluded_gain,
        })
        .insert_resource(self.tracing)
        .init_resource::<RaytracedAudioListenerTrace3d>()
        .add_systems(
            PostUpdate,
            update_raytraced_audio
                .after(TransformSystems::Propagate)
                .before(RaytracedAudioPrepareSystems),
        );
    }
}

/// Draws the 3D listener's recorded rays as an animated wavefront with gizmos.
///
/// Add it after [`RaytracedAudio3dPlugin`]. Configure the drawing through the
/// [`super::RaytracedAudioDebugDraw3d`] resource.
#[cfg(feature = "debug_draw")]
#[derive(Clone, Copy, Debug, Default)]
pub struct RaytracedAudio3dDebugPlugin;

#[cfg(feature = "debug_draw")]
impl Plugin for RaytracedAudio3dDebugPlugin {
    /// Registers the drawing configuration and the recording and drawing systems.
    fn build(&self, app: &mut App) {
        use super::debug_draw::{RaytracedAudioDebugDraw3d, draw_rays, sync_recording};
        use bevy::prelude::Update;

        app.init_resource::<RaytracedAudioDebugDraw3d>()
            .add_systems(Update, (sync_recording, draw_rays).chain());
    }
}
