//! Mapping and validation checks for shared ray-trace values.

use super::{MuffleFilter, RayTraceSettings, SourceRayResponse, TraceRng, muffle_strength};
use crate::BandGain;

/// Discovering no rays fully muffles; a quarter of rays is nearly clear.
#[test]
fn muffle_curve_falls_quickly() {
    assert!((muffle_strength(0.0) - 1.0).abs() < f32::EPSILON);
    assert!(muffle_strength(0.5) < 0.06);
    assert!(muffle_strength(0.1) > muffle_strength(0.2));
}

/// Direct visibility is always clear, while a hidden source loses treble before bass.
#[test]
fn hidden_sources_lose_high_frequencies_first() {
    let visible = SourceRayResponse::new(true, 0.0, BandGain::ZERO);
    assert_eq!(visible.filter(), MuffleFilter::CLEAR);

    let hidden = SourceRayResponse::new(false, 0.08, BandGain::ZERO);
    assert!(hidden.filter().gain_hf() < hidden.filter().gain_lf());
    assert!(hidden.filter().gain_lf() < 1.0);

    let leaking = SourceRayResponse::new(false, 0.0, BandGain::try_new(0.4, 0.2, 0.05).unwrap());
    assert!((leaking.filter().gain_lf() - 0.4).abs() < 1.0e-6);
    assert!((leaking.filter().gain_hf() - 0.05).abs() < 1.0e-6);
}

/// Builders reject invalid counts and coefficients.
#[test]
fn settings_validate_inputs() {
    assert!(RayTraceSettings::default().try_with_ray_count(0).is_err());
    assert!(
        RayTraceSettings::default()
            .try_with_scattering(1.5)
            .is_err()
    );
    assert!(
        RayTraceSettings::default()
            .try_with_escape_distance(0.0)
            .is_err()
    );
    assert!(
        RayTraceSettings::default()
            .try_with_escape_distance(f64::NAN)
            .is_err()
    );
    assert!(MuffleFilter::try_new(0.5, 1.5).is_err());
}

/// The generator stays in the unit interval and is deterministic.
#[test]
fn rng_is_deterministic_and_bounded() {
    let mut first = TraceRng::new(7);
    let mut second = TraceRng::new(7);
    for _ in 0..1000 {
        let value = first.next_unit();
        assert!((0.0..1.0).contains(&value));
        assert_eq!(value.to_bits(), second.next_unit().to_bits());
    }
    assert!(TraceRng::new(0).next_unit() > 0.0);
}
