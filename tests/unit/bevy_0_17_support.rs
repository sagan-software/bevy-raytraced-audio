//! Device-free sink fixtures for Bevy 0.17 tests.

/// Creates a rodio sink without opening an audio device.
pub(crate) fn test_audio_sink() -> bevy::audio::AudioSink {
    bevy::audio::AudioSink::new(rodio::Sink::new_idle().0)
}

/// Normalizes infallible system parameter access in older Bevy tests.
#[cfg(feature = "debug_draw")]
pub(crate) const fn test_system_param<T>(value: T) -> T {
    value
}
