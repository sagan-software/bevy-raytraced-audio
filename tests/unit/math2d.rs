//! Reference-norm regressions for extreme finite vectors.

use super::Vector2;
/// The fast norm agrees with scaled hypot, including overflow/underflow fallbacks.
#[test]
fn norm_preserves_extreme_ranges() {
    for scale in [
        0., 1.0e-300, 1.0e-160, 1.0e-30, 1., 1.0e30, 1.0e160, 1.0e300,
    ] {
        let v = Vector2 {
            x: 3. * scale,
            y: -4. * scale,
        };
        let reference = v.x.hypot(v.y);
        assert!((v.length() - reference).abs() <= reference * 4. * f64::EPSILON);
    }
}
