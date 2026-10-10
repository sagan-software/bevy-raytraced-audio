//! Listener ray-tracing configuration, shared results, and per-emitter responses for 3D.

use bevy::prelude::{Component, Resource};
use bevy_raytraced_audio::{
    ListenerTrace3d, RayTraceSettings, SourceRayResponse, TraceScheduleStatistics,
};

/// Runtime configuration for the 3D listener ray trace.
///
/// Change it at runtime to adjust ray counts, toggle ray recording for visualization, or fall
/// back to direct-path transmission only.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RaytracedAudioTracing3d {
    /// Ray counts, bounce limits, and recording options.
    pub settings: RayTraceSettings,
    /// Whether the listener trace runs; disabled tracing uses direct transmission only.
    pub enabled: bool,
    /// Minimum real seconds between changed-input traces; `0` permits updates every frame.
    ///
    /// Identical inputs reuse their deterministic result unless
    /// [`RayTraceSettings::with_result_reuse`] disables reuse. That option forces computation
    /// on each eligible interval; it does not remove the interval limit. Surface changes and newly traced
    /// emitters always trigger an immediate trace.
    /// Direct/image-source response components share this cadence when tracing is enabled.
    /// Sink volumes and DSP controls still update every frame.
    pub interval_s: f32,
}

impl Default for RaytracedAudioTracing3d {
    /// Enables tracing at up to 30 traces per second with default settings.
    fn default() -> Self {
        Self {
            settings: RayTraceSettings::default(),
            enabled: true,
            interval_s: 1.0 / 30.0,
        }
    }
}

/// The latest 3D listener trace, including recorded rays when recording is enabled.
#[derive(Resource, Debug, Default)]
pub struct RaytracedAudioListenerTrace3d {
    /// Reused trace output.
    pub(super) trace: ListenerTrace3d,
    /// Whether `trace` holds results for the current listener.
    pub(super) valid: bool,
    /// Cumulative adapter scheduling decisions.
    pub(super) scheduling: TraceScheduleStatistics,
}

impl RaytracedAudioListenerTrace3d {
    /// Returns scheduling and scene-refresh counts independently of trace-cache hits.
    #[must_use]
    pub const fn scheduling_statistics(&self) -> TraceScheduleStatistics {
        self.scheduling
    }

    /// Returns the latest trace, or `None` when tracing is disabled or no unique listener exists.
    #[must_use]
    pub const fn trace(&self) -> Option<&ListenerTrace3d> {
        if self.valid { Some(&self.trace) } else { None }
    }
}

/// How one 3D emitter sounds from the listener after the latest ray trace.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaytracedAudioRayResponse3d {
    /// Latest traced muffling response.
    pub(super) response: SourceRayResponse,
}

impl RaytracedAudioRayResponse3d {
    /// Returns the latest traced muffling response.
    #[must_use]
    pub const fn response(self) -> SourceRayResponse {
        self.response
    }
}
