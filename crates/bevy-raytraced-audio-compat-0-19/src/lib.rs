//! Shared 2D and 3D systems compiled against Bevy 0.19.

#[path = "../../shared/processed_audio.rs"]
pub mod processed_audio;

#[path = "../../shared/rodio_0_22.rs"]
pub mod rodio_source;

#[path = "../../bevy-raytraced-audio-2d/src/adapter.rs"]
pub mod audio_2d;

#[path = "../../bevy-raytraced-audio-3d/src/adapter.rs"]
pub mod audio_3d;

#[cfg(test)]
#[path = "../../../tests/unit/gpu_runtime.rs"]
pub(crate) mod gpu_runtime;

#[cfg(test)]
#[path = "../../../tests/unit/bevy_0_19_support.rs"]
mod test_support;
