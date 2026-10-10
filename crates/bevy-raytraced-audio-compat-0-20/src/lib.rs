//! Shared 2D and 3D systems compiled against Bevy 0.20 release candidate 2.

#[path = "../../shared/processed_audio.rs"]
pub mod processed_audio;

#[path = "../../shared/rodio_0_22.rs"]
pub mod rodio_source;

#[path = "../../bevy-raytraced-audio-2d/src/adapter.rs"]
pub mod audio_2d;

#[path = "../../bevy-raytraced-audio-3d/src/adapter.rs"]
pub mod audio_3d;

#[cfg(test)]
mod test_support;
