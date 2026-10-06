//! Device-free sink fixtures for Bevy 0.18 tests.

/// Creates a rodio sink without opening an audio device.
pub(crate) fn test_audio_sink() -> bevy::audio::AudioSink {
    bevy::audio::AudioSink::new(rodio::Sink::new_idle().0)
}
