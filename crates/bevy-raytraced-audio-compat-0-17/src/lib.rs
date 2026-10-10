//! Shared 2D and 3D systems compiled against Bevy 0.17.

#[path = "../../shared/processed_audio.rs"]
pub mod processed_audio;

#[path = "../../shared/rodio_0_20.rs"]
pub mod rodio_source;

#[path = "../../bevy-raytraced-audio-2d/src/adapter.rs"]
pub mod audio_2d;

#[path = "../../bevy-raytraced-audio-3d/src/adapter.rs"]
pub mod audio_3d;

#[cfg(test)]
#[path = "../../../tests/unit/bevy_0_17_support.rs"]
mod test_support;
