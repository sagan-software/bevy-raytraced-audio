//! Per-emitter propagation output from the 2d adapter.

use bevy::prelude::Component;
use bevy_raytraced_audio::AcousticResponse;

/// The latest direct and reflected response written for one emitter.
#[derive(Component, Clone, Copy, Debug)]
pub struct RaytracedAudioResponse2d {
    /// Computed response in meter units.
    pub(super) response: AcousticResponse,
}

impl RaytracedAudioResponse2d {
    /// Returns the latest computed propagation response.
    #[must_use]
    pub const fn response(self) -> AcousticResponse {
        self.response
    }
}
