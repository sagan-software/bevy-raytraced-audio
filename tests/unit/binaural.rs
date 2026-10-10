//! End-to-end impulse checks distinguish elevation coloration from simple gain/pan.

use super::{BinauralParams, BinauralProcessor, DATA, Position, RECORD_BYTES, TAPS, integer, ring};
use std::sync::Arc;

/// Renders one impulse, including both ears and the full FIR response.
fn impulse(position: [f32; 3], rate: u32, enabled: bool) -> Vec<[f32; 2]> {
    let params = Arc::new(BinauralParams::new(1.0));
    params.set_position(position);
    params.set_enabled(enabled);
    let mut processor = BinauralProcessor::new(params, rate);
    (0..1024)
        .map(|i| processor.process_frame(if i == 0 { 1.0 } else { 0.0 }, 0.0))
        .collect()
}

/// Squared amplitude summed over time.
fn energy(samples: &[[f32; 2]], ear: usize) -> f32 {
    samples
        .iter()
        .map(|sample| sample.get(ear).unwrap().powi(2))
        .sum()
}

/// Normalized waveform correlation removes simple amplitude differences.
fn correlation(a: &[[f32; 2]], b: &[[f32; 2]]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(a, b)| a[0] * b[0]).sum();
    dot / (energy(a, 0) * energy(b, 0)).sqrt()
}

/// Every record must remain intact when distributed or packaged by Nix/Cargo.
#[test]
fn embedded_measurements_are_complete() {
    assert_eq!(DATA.len(), 368 * RECORD_BYTES);
}

/// Equal-distance elevation pairs sound different even though stereo pan cannot separate them.
#[test]
fn elevation_changes_spectrum_not_just_level() {
    for rate in [22_050, 44_100, 48_000, 96_000] {
        let above = impulse([0.0, 0.6, -0.8], rate, true);
        let below = impulse([0.0, -0.6, -0.8], rate, true);
        assert!(correlation(&above, &below).abs() < 0.95, "{rate} Hz");
        let above_stereo = impulse([0.0, 0.6, -0.8], rate, false);
        let below_stereo = impulse([0.0, -0.6, -0.8], rate, false);
        assert_eq!(above_stereo, below_stereo);
    }
}

/// Right sources reach the right ear earlier and louder; mirroring swaps exactly both ears.
#[test]
fn lateral_sources_have_interaural_level_and_timing() {
    let right = impulse([1.0, 0.0, 0.0], 44_100, true);
    let left = impulse([-1.0, 0.0, 0.0], 44_100, true);
    assert!(energy(&right, 1) > energy(&right, 0) * 2.0);
    let peak = |ear: usize| {
        right
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a[ear].abs().total_cmp(&b[ear].abs()))
            .unwrap()
            .0
    };
    assert!(peak(1) < peak(0));
    for (a, b) in right.iter().zip(&left) {
        assert!((a[0] - b[1]).abs() < 1.0e-6);
        assert!((a[1] - b[0]).abs() < 1.0e-6);
    }
}

/// Front and rear have different spectral cues at the same distance and pan.
#[test]
fn front_and_rear_are_distinct() {
    assert!(
        correlation(
            &impulse([0.0, 0.0, -1.0], 44_100, true),
            &impulse([0.0, 0.0, 1.0], 44_100, true)
        )
        .abs()
            < 0.95
    );
}

/// Distance rolloff is independent of headphone filtering and never boosts near sources.
#[test]
fn distance_rolloff_is_applied_once() {
    for enabled in [false, true] {
        let near = impulse([0.0, 0.0, -1.0], 48_000, enabled);
        let far = impulse([0.0, 0.0, -2.0], 48_000, enabled);
        assert!((energy(&near, 0) / energy(&far, 0) - 4.0).abs() < 1.0e-4);
    }
}

/// Poles, invalid poses, missing listeners and mode changes must not poison audio.
#[test]
fn moving_sources_and_invalid_listeners_remain_bounded() {
    for rate in [8_000, 44_100, 48_000, 192_000] {
        let params = Arc::new(BinauralParams::new(f32::NAN));
        let mut processor = BinauralProcessor::new(Arc::clone(&params), rate);
        assert_eq!(processor.process_frame(1.0, 1.0), [0.0; 2]);
        for pos in [
            [0.0; 3],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [f32::NAN, 0.0, 0.0],
            [f32::MAX; 3],
        ] {
            params.set_position(pos);
            for _ in 0..4096 {
                for sample in processor.process_frame(0.1, 0.0) {
                    assert!(sample.is_finite() && sample.abs() < 1.0);
                }
            }
        }
        params.set_position([0.0, 0.0, -1.0]);
        params.set_enabled(false);
        for _ in 0..rate / 2 {
            processor.process_frame(0.1, 0.0);
        }
        let [l, r] = processor.process_frame(0.1, 0.0);
        assert!(0.1_f32.mul_add(-core::f32::consts::FRAC_1_SQRT_2, l).abs() < 1.0e-4);
        assert!((l - r).abs() < 1.0e-4);
        params.clear_listener();
        for _ in 0..rate / 2 {
            processor.process_frame(0.1, 0.0);
        }
        assert!(
            processor
                .process_frame(0.1, 0.0)
                .iter()
                .all(|s| s.abs() < 1.0e-5)
        );
    }
}

/// Chunked accumulation matches the scalar FIR for tails and changing coefficients.
#[test]
#[expect(
    clippy::suboptimal_flops,
    reason = "reference intentionally matches the original separate arithmetic"
)]
fn convolution_matches_scalar_reference() {
    for length in [1_usize, 3, 4, 5, 127, 128, 140, 279, 558] {
        let number = |i: usize| f32::from(u16::try_from(i).expect("bounded fixture"));
        let initial: Vec<_> = (0..length)
            .map(|i| [number(i % 13) * 0.001, -number(i % 19) * 0.002])
            .collect();
        let goals: Vec<_> = initial.iter().map(|&[l, r]| [r, l]).collect();
        let input: Vec<_> = (0..length).map(|i| (number(i % 31) - 15.) * 0.02).collect();
        for smooth in [false, true] {
            let mut actual = initial.clone();
            let mut expected = initial.clone();
            for _ in 0..32 {
                let result = if smooth {
                    super::convolve::<true>(&mut actual, &goals, &input, 0.03)
                } else {
                    super::convolve::<false>(&mut actual, &goals, &input, 0.03)
                };
                let mut reference = [0.; 2];
                for ((coefficient, goal), sample) in expected.iter_mut().zip(&goals).zip(&input) {
                    for ((value, target), output) in
                        coefficient.iter_mut().zip(goal).zip(&mut reference)
                    {
                        if smooth {
                            *value += 0.03 * (*target - *value);
                        }
                        *output += *value * *sample;
                    }
                }
                assert_eq!(actual, expected);
                for (a, b) in result.into_iter().zip(reference) {
                    assert!((a - b).abs() < 1.0e-6, "length={length}: {a} vs {b}");
                }
            }
        }
    }
}

/// Sparse stereo fallback matches the full FIR, including non-finite input history and transitions.
#[test]
fn sparse_fallback_matches_full_filter_and_transitions() {
    let params = Arc::new(BinauralParams::new(1.));
    params.set_position([0.5, 0.2, -1.]);
    params.set_enabled(false);
    let mut fast = BinauralProcessor::new(params.clone(), 48_000);
    let mut reference = BinauralProcessor::new(params.clone(), 48_000);
    reference.smoothing_active = true;
    for step in 0..1024_u16 {
        if step == 512 {
            params.set_enabled(true);
        }
        let input = if step == 32 {
            f32::NAN
        } else {
            f32::from(step % 17) * 0.01
        };
        let actual = fast.process_frame(input, 0.1);
        let expected = reference.process_frame(input, 0.1);
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a.is_nan() && b.is_nan()) || (a - b).abs() < 1e-6);
        }
    }
}

/// Fused reference for coefficient interpolation, retained to bound numerical changes.
fn fused_response(position: Position) -> [[f32; 2]; TAPS] {
    let mut lower = -40;
    let mut upper = 90;
    for elevation in (-40..=90).step_by(10) {
        if f32::from(elevation) <= position.elevation {
            lower = elevation;
        }
        if f32::from(elevation) >= position.elevation {
            upper = elevation;
            break;
        }
    }
    let blend = if upper > lower {
        (position.elevation - f32::from(lower)) / f32::from(upper - lower)
    } else {
        0.0
    };
    let low = ring(lower, position.azimuth.abs(), 1.0 - blend);
    let high = ring(upper, position.azimuth.abs(), blend);
    let mut taps = [[0.0; 2]; TAPS];
    for record in low.into_iter().chain(high) {
        for (tap, output) in taps.iter_mut().enumerate() {
            for (ear, sample) in output.iter_mut().enumerate() {
                let measured_ear = if (position.azimuth < 0.0) == record.swap_ears {
                    ear
                } else {
                    1 - ear
                };
                let offset = record.index * RECORD_BYTES + 4 + tap * 4 + measured_ear * 2;
                *sample = f32::from(integer(offset)).mul_add(record.weight / 32_768.0, *sample);
            }
        }
    }
    taps
}

/// Coefficient error stays below one millionth across rates and listener-relative directions.
#[test]
fn unfused_interpolation_matches_fused_reference() {
    for rate in [22_050, 44_100, 48_000, 96_000] {
        for x in -4..=4_i16 {
            let params = Arc::new(BinauralParams::new(1.));
            params.set_position([f32::from(x) * 0.3, 0.6, -0.8]);
            let processor = BinauralProcessor::new(params, rate);
            let native = fused_response(processor.position);
            for (actual, resample) in processor.target.iter().zip(&processor.resampler) {
                let mut expected = [0_f32; 2];
                for &(index, weight) in &resample.weights {
                    for (sample, value) in expected.iter_mut().zip(native.get(index).unwrap()) {
                        *sample =
                            (value * weight).mul_add(core::f32::consts::FRAC_1_SQRT_2, *sample);
                    }
                }
                for (a, b) in actual.iter().zip(expected) {
                    assert!((*a - b).abs() < 1e-6);
                }
            }
        }
    }
}

/// Invalid resampler indices are ignored instead of indexing beyond the measurement table.
#[test]
fn resampler_ignores_an_out_of_range_weight() {
    let params = Arc::new(BinauralParams::new(1.));
    params.set_position([1., 0., -1.]);
    let mut processor = BinauralProcessor::new(params, 48_000);
    let expected = processor.target.clone();
    processor
        .resampler
        .first_mut()
        .unwrap()
        .weights
        .push((TAPS, 1.));
    processor.update_response();
    assert_eq!(processor.target, expected);
}
