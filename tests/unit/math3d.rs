//! Reference-norm regressions for extreme finite vectors.

use super::Vector3;
/// Finite large and tiny vectors retain the range guarantees of the reference norm.
#[test]
fn norm_preserves_extreme_ranges() {
    for scale in [
        0., 1.0e-300, 1.0e-160, 1.0e-30, 1., 1.0e30, 1.0e160, 1.0e300,
    ] {
        let v = Vector3 {
            x: 2. * scale,
            y: -3. * scale,
            z: 6. * scale,
        };
        let reference = v.x.hypot(v.y).hypot(v.z);
        assert!((v.length() - reference).abs() <= reference * 4. * f64::EPSILON);
    }
}
