//! Decoder metadata checks.
use super::{AudioSource, Decodable, RaytracedAudioSource, Source, restart};

/// Metadata stays open-ended because live processing can loop and extend the source tail.
#[test]
fn decoder_metadata_and_float_wrapper_debug() {
    let source = AudioSource {
        bytes: std::sync::Arc::from(
            include_bytes!("../../assets/audio/bevy-raytraced-audio-chime.wav").as_slice(),
        ),
    };
    assert!(format!("{:?}", restart(&source)).contains("FloatSamples"));
    let processed = RaytracedAudioSource {
        source,
        params: std::sync::Arc::new(bevy_raytraced_audio::AcousticDspParams::default()),
        looping: false,
        binaural: None,
    };
    assert_eq!(processed.decoder().current_frame_len(), None);
}
