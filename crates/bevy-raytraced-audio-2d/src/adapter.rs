//! Public facade for the Bevy 2D acoustic adapter.

#[cfg(feature = "debug_draw")]
mod debug_draw;
mod emitter;
mod listener;
mod plugin;
mod ray_tracing;
mod reflection_paths;
mod response;
mod settings;
mod surface;
mod systems;
mod volume;

pub use crate::processed_audio::{
    RaytracedAudioPlayer, RaytracedAudioPrepareSystems, RaytracedAudioPrepared,
    RaytracedAudioProcessingPlugin, RaytracedAudioSource,
};
#[cfg(feature = "debug_draw")]
pub use debug_draw::{RayKindMask, RaytracedAudioDebugDraw2d};
#[cfg(feature = "debug_draw")]
pub(crate) use debug_draw::{kind_alpha, visible_fractions};
pub use emitter::RaytracedAudioEmitter2d;
pub use listener::RaytracedAudioListener2d;
#[cfg(feature = "debug_draw")]
pub use plugin::RaytracedAudio2dDebugPlugin;
pub use plugin::RaytracedAudio2dPlugin;
pub use ray_tracing::{
    RaytracedAudioListenerTrace2d, RaytracedAudioRayResponse2d, RaytracedAudioTracing2d,
};
pub use reflection_paths::RaytracedAudioReflectionPaths2d;
pub use response::RaytracedAudioResponse2d;
pub use surface::RaytracedAudioSurface2d;

#[cfg(test)]
#[path = "../../../tests/unit/2d_adapter.rs"]
mod tests;
