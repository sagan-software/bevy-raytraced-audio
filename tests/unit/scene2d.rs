//! Private boundary coverage for line intersection and image-source edge cases.

use super::{
    AcousticScene2d, PARAMETER_EPSILON, Vector2, first_reflection, path_intersects_segment,
};
use crate::{AcousticMaterial, Emitter2d, Listener2d, Point2, Segment2d};

/// Splitting a straight wall into adjoining segments cannot double attenuation at the join.
#[test]
fn transmission_is_continuous_at_segment_seams() {
    use crate::{AcousticMaterial, BandGain, Point2, Segment2d};
    let gain = BandGain::try_new(0.5, 0.3, 0.1).unwrap();
    let material = AcousticMaterial::default()
        .try_with_transmission(gain)
        .unwrap();
    let mut scene = AcousticScene2d::default();
    for x in [0.0, 0.2] {
        for (a, b) in [(-1.0, 0.0), (0.0, 1.0)] {
            scene.add_segment(
                Segment2d::try_new(
                    Point2::try_new(x, a).unwrap(),
                    Point2::try_new(x, b).unwrap(),
                    material,
                )
                .unwrap(),
            );
        }
    }
    for y in [-0.01, 0.0, 0.01] {
        let (actual, _) = scene.direct_transmission(point(-1.0, y), point(1.0, y));
        assert_eq!(actual, gain.multiply(gain));
    }
}

/// Makes a double-precision test point.
fn point(x: f64, y: f64) -> Vector2 {
    Vector2 { x, y }
}

/// Captured paths match the response query and stale outputs clear without losing capacity.
#[test]
fn trace_with_paths_reuses_storage_and_preserves_response() {
    let mut scene = AcousticScene2d::default();
    let reflector = Segment2d::try_new(
        Point2::try_new(0.0, -2.0).expect("finite reflector start"),
        Point2::try_new(0.0, 2.0).expect("finite reflector end"),
        AcousticMaterial::default(),
    )
    .expect("nondegenerate reflector");
    scene.add_segment(reflector);
    let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0).expect("finite emitter"));
    let listener = Listener2d::new(Point2::try_new(-3.0, 0.0).expect("finite listener"));
    let mut paths = Vec::with_capacity(4);

    let expected = scene.trace(emitter, listener);
    let actual = scene.trace_with_reflection_paths(emitter, listener, &mut paths);

    assert_eq!(actual, expected);
    assert_eq!(paths.len(), 1);
    let capacity = paths.capacity();
    scene.clear();
    let expected_without_reflections = scene.trace(emitter, listener);
    let actual_without_reflections =
        scene.trace_with_reflection_paths(emitter, listener, &mut paths);
    assert_eq!(actual_without_reflections, expected_without_reflections);
    assert_eq!(paths.len(), 0);
    assert_eq!(paths.capacity(), capacity);
}

/// Collinear overlap blocks an open path, while endpoint touch and parallel offset do not.
#[test]
fn segment_intersection_handles_parallel_and_endpoint_cases() {
    assert!(path_intersects_segment(
        point(0.0, 0.0),
        point(2.0, 0.0),
        point(0.5, 0.0),
        point(1.5, 0.0),
    ));
    assert!(!path_intersects_segment(
        point(0.0, 0.0),
        point(2.0, 0.0),
        point(2.0, 0.0),
        point(3.0, 0.0),
    ));
    assert!(!path_intersects_segment(
        point(0.0, 0.0),
        point(2.0, 0.0),
        point(0.0, 1.0),
        point(2.0, 1.0),
    ));
    assert!(!path_intersects_segment(
        point(0.0, 0.0),
        point(0.0, 0.0),
        point(-1.0, 0.0),
        point(1.0, 0.0),
    ));
}

/// A valid wall hit remains accepted inside the tolerant segment-parameter boundary.
#[test]
fn segment_intersection_preserves_wall_endpoint_tolerance() {
    let y_offset = PARAMETER_EPSILON * 0.5;
    assert!(path_intersects_segment(
        point(-1.0, 1.0 + y_offset),
        point(1.0, 1.0 + y_offset),
        point(0.0, 0.0),
        point(0.0, 1.0),
    ));
}

/// A wall endpoint can block a crossing, and a reflection must meet the finite wall.
#[test]
fn image_reflection_checks_side_and_segment_bounds() {
    assert!(path_intersects_segment(
        point(0.0, 0.0),
        point(2.0, 0.0),
        point(1.0, 0.0),
        point(1.0, 1.0),
    ));
    assert!(
        first_reflection(
            point(0.0, 1.0),
            point(0.0, -1.0),
            point(-1.0, 0.0),
            point(1.0, 0.0),
        )
        .is_none()
    );
    assert!(
        first_reflection(
            point(0.0, 1.0),
            point(0.0, 0.0),
            point(-1.0, 0.0),
            point(1.0, 0.0),
        )
        .is_none()
    );
    assert!(
        first_reflection(
            point(0.0, 1.0),
            point(4.0, 1.0),
            point(3.0, 0.0),
            point(4.0, 0.0),
        )
        .is_none()
    );
    assert!(
        first_reflection(
            point(0.0, -1.0),
            point(1.0e15, -2.0),
            point(-1.0, 0.0),
            point(1.0, 0.0),
        )
        .is_none()
    );
}
