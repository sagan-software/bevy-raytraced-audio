//! Device-free sink fixtures for Bevy 0.19 tests.

/// Creates a rodio player without opening an audio device.
pub(crate) fn test_audio_sink() -> bevy::audio::AudioSink {
    bevy::audio::AudioSink::new(rodio::Player::new().0)
}
