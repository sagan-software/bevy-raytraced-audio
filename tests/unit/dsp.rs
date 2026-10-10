//! Signal checks for the muffle filter and reverb tail.

use super::{AcousticDspParams, AcousticDspProcessor};

use crate::{MuffleFilter, ReverbEstimate};
use std::sync::Arc;

/// Measures the RMS level of a mono sine after processing.
fn sine_rms(params: &Arc<AcousticDspParams>, frequency_hz: f32) -> f32 {
    let mut processor = AcousticDspProcessor::new(Arc::clone(params), 1, 48_000);
    let mut sum = 0.0;
    let mut count = 0_u16;
    for index in 0_u16..48_000 {
        let phase = core::f32::consts::TAU * frequency_hz * f32::from(index) / 48_000.0;
        let output = processor.process_sample(phase.sin());
        if index > 24_000 {
            sum = output.mul_add(output, sum);
            count += 1;
        }
    }
    (sum / f32::from(count)).sqrt()
}

/// A muffled filter keeps bass while removing treble; a clear filter passes both.
#[test]
fn muffling_attenuates_high_frequencies() {
    let params = Arc::new(AcousticDspParams::default());
    let clear_high = sine_rms(&params, 6_000.0);
    params.set_filter(MuffleFilter::try_new(1.0, 0.05).unwrap());
    let muffled_low = sine_rms(&params, 100.0);
    let muffled_high = sine_rms(&params, 6_000.0);
    assert!(clear_high > 0.65, "clear treble rms {clear_high}");
    assert!(muffled_low > 0.6, "muffled bass rms {muffled_low}");
    assert!(muffled_high < 0.12, "muffled treble rms {muffled_high}");
}

/// Bypassed processing returns input samples unchanged after smoothing settles.
#[test]
fn bypass_passes_audio_through() {
    let params = Arc::new(AcousticDspParams::default());
    params.set_filter(MuffleFilter::SILENT);
    params.set_enabled(false);
    let mut processor = AcousticDspProcessor::new(Arc::clone(&params), 2, 44_100);
    for index in 0_u16..200 {
        let input = (f32::from(index) * 0.1).sin();
        let output = processor.process_sample(input);
        assert!((output - input).abs() < 1.0e-4);
    }
}

/// An impulse produces a decaying tail only when the reverb send is open.
#[test]
fn reverb_produces_a_tail() {
    let params = Arc::new(AcousticDspParams::default());
    let tail_energy = |params: &Arc<AcousticDspParams>| {
        let mut processor = AcousticDspProcessor::new(Arc::clone(params), 1, 44_100);
        let mut energy = 0.0;
        for index in 0..44_100 {
            let output = processor.process_sample(if index == 0 { 1.0 } else { 0.0 });
            if index > 4_410 {
                energy = output.mul_add(output, energy);
            }
            assert!(output.is_finite() && output.abs() < 4.0);
        }
        energy
    };
    let dry = tail_energy(&params);
    params.set_reverb(ReverbEstimate::from_parameters(1.0, 1.5, 0.02), 1.0);
    let wet = tail_energy(&params);
    assert!(dry < 1.0e-9, "dry tail {dry}");
    assert!(wet > 1.0e-3, "wet tail {wet}");
}

/// Invalid caller-provided sends must not poison the audio stream with NaN.
#[test]
fn invalid_reverb_send_keeps_samples_finite() {
    for send in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 3.0] {
        let params = Arc::new(AcousticDspParams::default());
        params.set_reverb(ReverbEstimate::from_parameters(1.0, 1.5, 0.02), send);
        assert!((0.0..=1.0).contains(&params.wet_gain()));
        let mut processor = AcousticDspProcessor::new(params, 1, 8_000);
        for index in 0..8_000 {
            let output = processor.process_sample(if index == 0 { 1.0 } else { 0.0 });
            assert!(output.is_finite());
        }
    }
}
/// Surface-dependent high-band decay audibly shortens the treble tail without changing dry sound.
#[test]
fn absorbent_material_shortens_high_frequency_tail() {
    let tail = |frequency: f32, high_decay: f32| {
        let params = Arc::new(AcousticDspParams::default());
        params.set_reverb(
            ReverbEstimate::from_parameters(1.0, 2.0, 0.015).with_band_decay(2.0, high_decay),
            1.0,
        );
        let mut processor = AcousticDspProcessor::new(params, 1, 48_000);
        let mut energy = 0.0;
        for frame in 0_u16..48_000 {
            let input = if frame < 960 {
                (core::f32::consts::TAU * frequency * f32::from(frame) / 48_000.0).sin() * 0.1
            } else {
                0.0
            };
            let output = processor.process_sample(input);
            if frame > 12_000 {
                energy = output.mul_add(output, energy);
            }
            assert!(output.is_finite());
        }
        energy
    };
    let hard = tail(8_000.0, 2.0);
    let soft = tail(8_000.0, 0.12);
    assert!(soft < hard * 0.3, "hard={hard}, soft={soft}");
    let bass = tail(100.0, 0.12);
    assert!(bass > soft * 4.0, "bass={bass}, treble={soft}");
}

/// The division-free delay ring matches a modulo reference at every wrap and supported delay.
#[test]
fn delay_ring_matches_modulo_reference() {
    for length in [1, 2, 7, 31] {
        for delay in 0..length {
            let mut processor =
                AcousticDspProcessor::new(Arc::new(AcousticDspParams::default()), 1, 8_000);
            processor.pre_delay = vec![0.; length];
            processor.sample_rate = 8.;
            processor.current.pre_delay_s = f32::from(u16::try_from(delay).unwrap()) / 8.;
            processor.current.wet_gain = 0.;
            processor.current.early_gain = 1.;
            let mut reference = vec![0.; length];
            for step in 0..length * 3 {
                let position = step % length;
                let input = f32::from(u16::try_from(step).unwrap());
                *reference.get_mut(position).unwrap() = input;
                let expected = reference
                    .get((position + length - delay) % length)
                    .copied()
                    .unwrap();
                assert_eq!(processor.reverb_frame(input), expected);
            }
        }
    }
}

/// Defensive scalar kernels handle unavailable storage and non-finite sample counts safely.
#[test]
fn defensive_buffer_boundaries_and_live_decay_change() {
    assert_eq!(super::samples_to_count(f32::NAN), 0);
    assert_eq!(super::samples_to_count(f32::INFINITY), 0);
    let mut comb = super::Comb {
        buffer: Vec::new(),
        position: 0,
        filter_state: 0.,
    };
    assert_eq!(comb.process(1., [0.5; 2], 0.1), 0.);
    let mut allpass = super::AllPass {
        buffer: Vec::new(),
        position: 0,
    };
    assert_eq!(allpass.process(1.), 1.);
    let params = Arc::new(AcousticDspParams::default());
    let mut processor = AcousticDspProcessor::new(params.clone(), 1, 48_000);
    let original = processor.feedback_decay_s;
    params.set_reverb(
        ReverbEstimate::from_parameters(0.5, 4., 0.).with_band_decay(6., 2.),
        1.,
    );
    for _ in 0..4096 {
        processor.process_sample(0.);
    }
    assert!(processor.feedback_decay_s[0] > original[0]);
    processor.low_first.clear();
    assert_eq!(processor.process_sample_split(0.25), (0.25, 0.));
    processor.low_first.push(0.);
    processor.low_second.clear();
    assert_eq!(processor.process_sample_split(0.25), (0.25, 0.));
}
