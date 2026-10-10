//! Public facade for the Bevy 3D acoustic adapter.

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

// The ray-kind mask is shared with the 2D drawing so both adapters accept the same value.
#[cfg(feature = "debug_draw")]
pub use crate::audio_2d::RayKindMask;
pub use crate::processed_audio::{
    RaytracedAudioPlayer, RaytracedAudioPrepareSystems, RaytracedAudioPrepared,
    RaytracedAudioProcessingPlugin, RaytracedAudioSource,
};
#[cfg(feature = "debug_draw")]
pub use debug_draw::RaytracedAudioDebugDraw3d;
pub use emitter::RaytracedAudioEmitter3d;
pub use listener::RaytracedAudioListener3d;
#[cfg(feature = "debug_draw")]
pub use plugin::RaytracedAudio3dDebugPlugin;
pub use plugin::RaytracedAudio3dPlugin;
pub use ray_tracing::{
    RaytracedAudioListenerTrace3d, RaytracedAudioRayResponse3d, RaytracedAudioTracing3d,
};
pub use reflection_paths::RaytracedAudioReflectionPaths3d;
pub use response::RaytracedAudioResponse3d;
pub use surface::RaytracedAudioSurface3d;

#[cfg(test)]
#[path = "../../../tests/unit/3d_adapter.rs"]
mod tests;
