//! Private boundary coverage for line intersection and image-source edge cases.

use super::{
    AcousticScene2d, PARALLEL_EPSILON, PARAMETER_EPSILON, SegmentGeometry, Vector2,
    first_reflection_geometry, path_intersects_geometry,
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

/// Exercises the cached kernel through the original four-endpoint test interface.
fn first_reflection(
    source: Vector2,
    receiver: Vector2,
    start: Vector2,
    end: Vector2,
) -> Option<(Vector2, f64, Vector2)> {
    first_reflection_geometry(
        source,
        receiver,
        &SegmentGeometry::new(start, end, AcousticMaterial::default()),
    )
}
/// Exercises the cached kernel through the original four-endpoint test interface.
fn path_intersects_segment(
    source: Vector2,
    receiver: Vector2,
    start: Vector2,
    end: Vector2,
) -> bool {
    path_intersects_geometry(
        source,
        receiver,
        &SegmentGeometry::new(start, end, AcousticMaterial::default()),
    )
}
/// Original uncached image-source calculation used as a numerical reference.
fn reference_first_reflection(
    source: Vector2,
    receiver: Vector2,
    wall_start: Vector2,
    wall_end: Vector2,
) -> Option<(Vector2, f64, Vector2)> {
    let wall = wall_end.subtract(wall_start);
    let normal = Vector2 {
        x: -wall.y,
        y: wall.x,
    };
    let normal_squared = normal.dot(normal);
    let source_offset = source.subtract(wall_start).dot(normal);
    let receiver_offset = receiver.subtract(wall_start).dot(normal);

    // If either endpoint lies on the surface or they lie on opposite sides, no image-source reflection exists.
    if source_offset == 0.0
        || receiver_offset == 0.0
        || source_offset.is_sign_positive() != receiver_offset.is_sign_positive()
    {
        return None;
    }

    let image_source = source.subtract(normal.scale(2.0 * source_offset / normal_squared));
    let image_to_receiver = receiver.subtract(image_source);
    let denominator = image_to_receiver.cross(wall);
    let scale = image_to_receiver.length() * wall.length();
    if denominator.abs() <= PARALLEL_EPSILON * scale {
        return None;
    }

    // If the image ray misses the finite wall segment, the candidate cannot reflect from this surface.
    let image_to_wall = wall_start.subtract(image_source);
    let ray_parameter = image_to_wall.cross(wall) / denominator;
    let wall_parameter = image_to_wall.cross(image_to_receiver) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter)
        || !(-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&wall_parameter)
    {
        return None;
    }

    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    let reflected_distance = image_to_receiver.length();
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance, image_source))
}

/// Tests an open path against a finite 2D segment, including wall endpoints.
fn reference_path_intersects_segment(
    path_start: Vector2,
    path_end: Vector2,
    wall_start: Vector2,
    wall_end: Vector2,
) -> bool {
    let path_min_x = path_start.x.min(path_end.x);
    let path_max_x = path_start.x.max(path_end.x);
    let path_min_y = path_start.y.min(path_end.y);
    let path_max_y = path_start.y.max(path_end.y);
    let surface_padding_x = PARAMETER_EPSILON * (wall_end.x - wall_start.x).abs();
    let surface_padding_y = PARAMETER_EPSILON * (wall_end.y - wall_start.y).abs();
    let surface_min_x = wall_start.x.min(wall_end.x) - surface_padding_x;
    let surface_max_x = wall_start.x.max(wall_end.x) + surface_padding_x;
    let surface_min_y = wall_start.y.min(wall_end.y) - surface_padding_y;
    let surface_max_y = wall_start.y.max(wall_end.y) + surface_padding_y;

    // If the path and tolerant surface bounds do not overlap, they cannot intersect.
    if path_max_x < surface_min_x
        || surface_max_x < path_min_x
        || path_max_y < surface_min_y
        || surface_max_y < path_min_y
    {
        return false;
    }

    let path = path_end.subtract(path_start);
    let wall = wall_end.subtract(wall_start);
    let path_length = path.length();
    let wall_length = wall.length();
    if path_length == 0.0 || wall_length == 0.0 {
        return false;
    }

    let start_delta = wall_start.subtract(path_start);
    let denominator = path.cross(wall);
    if denominator.abs() <= PARALLEL_EPSILON * path_length * wall_length {
        // Collinear overlap blocks only when it reaches the open path interior.
        if start_delta.cross(path).abs() > PARALLEL_EPSILON * path_length * start_delta.length() {
            return false;
        }

        let path_length_squared = path.dot(path);
        let first_parameter = start_delta.dot(path) / path_length_squared;
        let second_parameter = wall_end.subtract(path_start).dot(path) / path_length_squared;
        let overlap_start = first_parameter.min(second_parameter);
        let overlap_end = first_parameter.max(second_parameter);
        return overlap_end > PARAMETER_EPSILON && overlap_start < 1.0 - PARAMETER_EPSILON;
    }

    // Parallelism was rejected above, so these parameters identify one segment crossing.
    let path_parameter = start_delta.cross(wall) / denominator;
    let wall_parameter = start_delta.cross(path) / denominator;
    path_parameter > PARAMETER_EPSILON
        && path_parameter < 1.0 - PARAMETER_EPSILON
        && (-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&wall_parameter)
}

/// Cached geometry preserves the original scalar results over skewed walls and arbitrary paths.
#[test]
fn cached_geometry_matches_scalar_reference() {
    let mut rng = crate::ray_trace::TraceRng::new(37);
    for _ in 0..2048 {
        let mut vector = || Vector2 {
            x: rng.next_unit().mul_add(20., -10.),
            y: rng.next_unit().mul_add(20., -10.),
        };
        let source = vector();
        let receiver = vector();
        let start = vector();
        let end = vector();
        assert_eq!(
            first_reflection(source, receiver, start, end),
            reference_first_reflection(source, receiver, start, end)
        );
        assert_eq!(
            path_intersects_segment(source, receiver, start, end),
            reference_path_intersects_segment(source, receiver, start, end)
        );
    }
}

/// Near-parallel image rays and zero-length paths inside surface bounds remain rejected.
#[test]
fn cached_geometry_parallel_and_zero_length_boundaries() {
    let start = Vector2 { x: 0., y: -2. };
    let end = Vector2 { x: 0., y: 2. };
    let source = Vector2 { x: 1e-15, y: -1. };
    let receiver = Vector2 { x: 1e-15, y: 1. };
    assert_eq!(first_reflection(source, receiver, start, end), None);
    assert!(!path_intersects_segment(
        Vector2::default(),
        Vector2::default(),
        start,
        end
    ));
}

/// Parallel diagonals can have overlapping boxes without crossing each other.
#[test]
fn parallel_offset_diagonals_do_not_intersect() {
    assert!(!path_intersects_segment(
        Vector2 { x: -1.0, y: -1.0 },
        Vector2 { x: 1.0, y: 1.0 },
        Vector2 { x: -1.0, y: -0.75 },
        Vector2 { x: 1.0, y: 1.25 },
    ));
}

/// Tangential rejection agrees with exact reflections for both wall orientations and many queries.
#[test]
fn reflection_broad_phase_has_no_false_negatives() {
    for horizontal in [false, true] {
        let mut scene = AcousticScene2d::default();
        for offset in [-5., -3., -1., 1., 3., 5.] {
            let points = if horizontal {
                [(-2., offset), (2., offset)]
            } else {
                [(offset, -2.), (offset, 2.)]
            };
            scene.add_segment(
                Segment2d::try_new(
                    Point2::try_new(points[0].0, points[0].1).unwrap(),
                    Point2::try_new(points[1].0, points[1].1).unwrap(),
                    AcousticMaterial::default(),
                )
                .unwrap(),
            );
        }
        let acceleration = scene.acceleration();
        for i in -8..=8 {
            for j in -8..=8 {
                let source = Vector2 {
                    x: f64::from(i),
                    y: f64::from(j),
                };
                let receiver = Vector2 {
                    x: f64::from(j) + 0.25,
                    y: f64::from(i) - 0.25,
                };
                if !acceleration
                    .hierarchy
                    .may_have_reflections(super::Bounds::path_2d(
                        (source.x, source.y),
                        (receiver.x, receiver.y),
                    ))
                {
                    assert!(acceleration.segments.iter().all(|wall| first_reflection_geometry(source, receiver, wall).is_none()));
                }
            }
        }
    }
}

/// A complete query returns the direct path without scanning distant parallel reflectors.
#[test]
fn distant_parallel_walls_have_no_reflection_output() {
    let mut scene = AcousticScene2d::default();
    for x in 0..8 {
        let x = f32::from(u16::try_from(x).unwrap());
        scene.add_segment(
            Segment2d::try_new(
                Point2::try_new(x, 100.).unwrap(),
                Point2::try_new(x, 101.).unwrap(),
                AcousticMaterial::default(),
            )
            .unwrap(),
        );
    }
    let emitter = Emitter2d::new(Point2::try_new(-1., -1.).unwrap());
    let listener = Listener2d::new(Point2::try_new(-2., 1.).unwrap());
    assert_eq!(scene.reflection_paths(emitter, listener).count(), 0);
    assert_eq!(
        scene.trace(emitter, listener),
        AcousticScene2d::default().trace(emitter, listener)
    );
}
