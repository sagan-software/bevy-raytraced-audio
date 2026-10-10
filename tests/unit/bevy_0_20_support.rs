//! Device-free sink fixtures for Bevy 0.20 release-candidate tests.

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
