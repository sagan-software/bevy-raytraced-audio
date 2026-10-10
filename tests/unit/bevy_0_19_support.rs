//! Device-free sink fixtures for Bevy 0.19 tests.

/// Creates a rodio player without opening an audio device.
pub(crate) fn test_audio_sink() -> bevy::audio::AudioSink {
    bevy::audio::AudioSink::new(rodio::Player::new().0)
}

/// Validates system parameter access in newer Bevy tests.
///
/// # Panics
/// Required test resources must exist.
#[cfg(feature = "debug_draw")]
pub(crate) fn test_system_param<T>(value: Result<T, impl core::fmt::Debug>) -> T {
    value.expect("test system parameters are registered")
}

/// Runs spatial controls with a device-free mixer.
pub(crate) fn with_spatial_audio_sink(test: impl FnOnce(bevy::audio::SpatialAudioSink)) {
    let (mixer, _source) = rodio::mixer::mixer(
        core::num::NonZeroU16::new(2).unwrap(),
        core::num::NonZeroU32::new(48_000).unwrap(),
    );
    let player =
        rodio::SpatialPlayer::connect_new(&mixer, [0.0; 3], [-0.1, 0.0, 0.0], [0.1, 0.0, 0.0]);
    test(bevy::audio::SpatialAudioSink::new(player));
}
