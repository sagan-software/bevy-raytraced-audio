//! Shared 2D and 3D systems compiled against Bevy 0.17.

#[path = "../../bevy-raytraced-audio-2d/src/adapter.rs"]
pub mod audio_2d;

#[path = "../../bevy-raytraced-audio-3d/src/adapter.rs"]
pub mod audio_3d;

#[cfg(test)]
mod test_support;
