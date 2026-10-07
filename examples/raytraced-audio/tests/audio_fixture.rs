//! Checks that the example audio asset uses a format enabled by Bevy.

use bevy::audio::{AudioSource, Decodable};
use std::sync::Arc;

/// Decodes the shared WAV fixture through Bevy's configured decoder.
#[test]
fn bundled_chime_decodes_with_bevy_audio_features() {
    let bytes = include_bytes!("../assets/audio/bevy-raytraced-audio-chime.wav");
    let source = AudioSource {
        bytes: Arc::from(bytes.as_slice()),
    };

    assert!(source.decoder().next().is_some());
}

/// Keeps the browser chime above the audible peak level used by the demos.
#[test]
fn bundled_chime_has_a_clear_playback_level() {
    let bytes = include_bytes!("../assets/audio/bevy-raytraced-audio-chime.wav");
    let source = AudioSource {
        bytes: Arc::from(bytes.as_slice()),
    };
    let peak = source
        .decoder()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));

    assert!(peak >= 0.4, "the chime peak is {peak:.3}");
}
