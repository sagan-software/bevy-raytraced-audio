//! Checks that the example audio assets decode with Bevy and that processing audibly muffles them.

use bevy::audio::{AudioSource, Decodable, Source};
use bevy_raytraced_audio::{AcousticDspParams, AcousticDspProcessor, MuffleFilter};
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

/// Retains the original WAV fixture's signal-level regression check.
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

/// Every shipped recording must decode completely to finite, unclipped mono audio.
#[test]
fn bundled_recordings_decode_with_bevy_audio_features() -> std::io::Result<()> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio");
    let mut recordings = 0;
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "ogg") {
            continue;
        }
        let source = AudioSource {
            bytes: Arc::from(std::fs::read(&path)?),
        };
        let mut decoder = source.decoder();
        assert_eq!(decoder.channels().get(), 1, "{}", path.display());
        assert_eq!(decoder.sample_rate().get(), 44_100, "{}", path.display());
        let mut peak = 0.0_f32;
        for sample in &mut decoder {
            assert!(
                sample.is_finite() && sample.abs() <= 1.0,
                "{}: {sample}",
                path.display()
            );
            peak = peak.max(sample.abs());
        }
        assert!(peak > 0.01, "{} contains no audible signal", path.display());
        recordings += 1;
    }
    assert_eq!(recordings, 20, "all bundled Ogg fixtures must be present");
    Ok(())
}

/// Seconds of the music recording compared in the muffling test.
const MUFFLE_TEST_SECONDS: usize = 4;

/// Samples averaged by the crude low-pass used to measure bass energy.
const MOVING_AVERAGE_LENGTH: u8 = 64;

/// Runs decoded samples through an acoustic processor with a fixed filter and no reverb.
fn process(samples: &[f32], channels: u16, sample_rate_hz: u32, filter: MuffleFilter) -> Vec<f32> {
    let params = Arc::new(AcousticDspParams::default());
    params.set_filter(filter);
    let mut processor = AcousticDspProcessor::new(params, channels, sample_rate_hz);
    samples
        .iter()
        .map(|sample| processor.process_sample(*sample))
        .collect()
}

/// Returns the energy of the first difference, a crude high-pass measure of treble.
fn treble_energy(samples: &[f32]) -> f64 {
    samples
        .windows(2)
        .map(|pair| match pair {
            [previous, current] => f64::from(current - previous).powi(2),
            _ => 0.0,
        })
        .sum()
}

/// Returns the energy of a moving average, a crude low-pass measure of bass.
fn bass_energy(samples: &[f32]) -> f64 {
    samples
        .windows(usize::from(MOVING_AVERAGE_LENGTH))
        .map(|window| {
            let sum: f64 = window.iter().copied().map(f64::from).sum();
            (sum / f64::from(MOVING_AVERAGE_LENGTH)).powi(2)
        })
        .sum()
}

/// A strong muffle filter removes most of a real recording's treble but keeps its bass.
#[test]
fn processed_audio_muffles_real_recording() -> Result<(), bevy_raytraced_audio::GeometryError> {
    let bytes = include_bytes!("../assets/audio/music_loop.ogg");
    let source = AudioSource {
        bytes: Arc::from(bytes.as_slice()),
    };
    let decoder = source.decoder();
    let channels = decoder.channels().get();
    let sample_rate_hz = decoder.sample_rate().get();
    let frames = usize::try_from(sample_rate_hz).unwrap_or(usize::MAX) * MUFFLE_TEST_SECONDS;
    let samples: Vec<f32> = decoder.take(frames * usize::from(channels)).collect();
    assert!(!samples.is_empty(), "the music recording decodes");

    let clear = process(&samples, channels, sample_rate_hz, MuffleFilter::CLEAR);
    let muffled = process(
        &samples,
        channels,
        sample_rate_hz,
        MuffleFilter::try_new(1.0, 0.05)?,
    );

    let treble_ratio = treble_energy(&muffled) / treble_energy(&clear);
    let bass_ratio = bass_energy(&muffled) / bass_energy(&clear);
    assert!(
        treble_ratio < 0.25,
        "muffling keeps {:.1}% of the treble energy",
        treble_ratio * 100.0
    );
    assert!(
        bass_ratio > 0.5,
        "muffling keeps only {:.1}% of the bass energy",
        bass_ratio * 100.0
    );
    Ok(())
}

/// Spatial downmix preserves level instead of adding stereo channels at twice the amplitude.
#[test]
fn spatial_downmix_averages_input_channels() {
    use rodio::{buffer::SamplesBuffer, source::ChannelVolume};
    use std::num::{NonZeroU16, NonZeroU32};
    for channels in [1, 2, 6] {
        let input = SamplesBuffer::new(
            NonZeroU16::new(channels).unwrap(),
            NonZeroU32::new(48_000).unwrap(),
            vec![0.75; usize::from(channels) * 4],
        );
        let actual: Vec<_> = ChannelVolume::new(input, vec![1.0, 0.5]).collect();
        assert_eq!(actual, [0.75, 0.375].repeat(4));
    }
}

/// Actual muffled footsteps retain different above/below signals, and impulses retain headroom.
#[test]
fn binaural_recordings_have_elevation_cues_and_headroom() {
    use bevy_raytraced_audio::{BinauralParams, BinauralProcessor};
    let bytes = include_bytes!("../assets/audio/footsteps_walk_loop.ogg");
    let source = AudioSource {
        bytes: Arc::from(bytes.as_slice()),
    };
    let samples: Vec<f32> = source.decoder().take(88_200).collect();
    let render = |height| {
        let params = Arc::new(BinauralParams::new(1.0));
        params.set_position([0.0, height, -0.8]);
        let mut head = BinauralProcessor::new(params, 44_100);
        let filtered = process(
            &samples,
            1,
            44_100,
            MuffleFilter::try_new(0.45, 0.045).unwrap(),
        );
        filtered
            .into_iter()
            .map(|sample| head.process_frame(sample, 0.0)[0])
            .collect::<Vec<_>>()
    };
    let above = render(0.6);
    let below = render(-0.6);
    let dot: f64 = above
        .iter()
        .zip(&below)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum();
    let energy = |signal: &[f32]| signal.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    let correlation = dot / (energy(&above) * energy(&below)).sqrt();
    assert!(
        correlation.abs() < 0.99,
        "muffled footstep correlation: {correlation}"
    );
    let shot = AudioSource {
        bytes: Arc::from(include_bytes!("../assets/audio/gunshot_1.ogg").as_slice()),
    };
    let params = Arc::new(BinauralParams::new(0.22));
    params.set_position([0.0, 0.0, -0.5]);
    let mut head = BinauralProcessor::new(params, 44_100);
    for sample in shot.decoder() {
        for output in head.process_frame(sample * 0.16 * 0.55, 0.0) {
            assert!(output.is_finite() && output.abs() < 1.0);
        }
    }
}
