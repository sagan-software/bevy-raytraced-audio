//! Decoder metadata checks.
use super::{AudioSource, Decodable, RaytracedAudioSource, Source};

/// Metadata stays open-ended because live processing can loop and extend the source tail.
#[test]
fn decoder_metadata_is_open_ended() {
    let processed = RaytracedAudioSource {
        source: AudioSource {
            bytes: std::sync::Arc::from(
                include_bytes!("../../assets/audio/bevy-raytraced-audio-chime.wav").as_slice(),
            ),
        },
        params: std::sync::Arc::new(bevy_raytraced_audio::AcousticDspParams::default()),
        looping: false,
        binaural: None,
    };
    assert_eq!(processed.decoder().current_span_len(), None);
}
