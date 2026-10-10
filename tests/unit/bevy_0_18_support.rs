//! Device-free sink fixtures for Bevy 0.18 tests.

/// Creates a rodio sink without opening an audio device.
pub(crate) fn test_audio_sink() -> bevy::audio::AudioSink {
    bevy::audio::AudioSink::new(rodio::Sink::new_idle().0)
}

/// Normalizes infallible system parameter access in older Bevy tests.
#[cfg(feature = "debug_draw")]
pub(crate) const fn test_system_param<T>(value: T) -> T {
    value
}

/// Runs spatial controls against an explicitly enabled virtual audio device.
/// The stream stays alive until the test finishes; normal tests never open hardware.
pub(crate) fn with_spatial_audio_sink(test: impl FnOnce(bevy::audio::SpatialAudioSink)) {
    if std::env::var_os("ACOUSTIC_TEST_AUDIO_DEVICE").is_none() {
        return;
    }
    let (_stream, handle) =
        rodio::OutputStream::try_default().expect("configured virtual audio device");
    let sink = rodio::SpatialSink::try_new(&handle, [0.0; 3], [-0.1, 0.0, 0.0], [0.1, 0.0, 0.0])
        .expect("virtual spatial sink");
    test(bevy::audio::SpatialAudioSink::new(sink));
}
