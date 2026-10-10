//! Private boundary coverage for triangle intersection and image-source handling.

use super::{
    AcousticScene3d, Bounds, PARAMETER_EPSILON, TriangleGeometry, Vector3, first_reflection,
    path_intersects_triangle, point_in_triangle,
};
use crate::{AcousticMaterial, Emitter3d, Listener3d, Point3, Triangle3d};

/// Makes a double-precision test vector.
fn vector(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 { x, y, z }
}

/// Adjacent triangles form one wall crossing, even exactly on their shared diagonal.
#[test]
fn transmission_does_not_double_attenuate_mesh_seams() {
    let gain = crate::BandGain::try_new(0.5, 0.3, 0.1).unwrap();
    let material = AcousticMaterial::default()
        .try_with_transmission(gain)
        .unwrap();
    let mut scene = AcousticScene3d::default();
    for x in [0.0, 0.2] {
        let corners = [
            Point3::try_new(x, -1.0, -1.0).unwrap(),
            Point3::try_new(x, 1.0, -1.0).unwrap(),
            Point3::try_new(x, 1.0, 1.0).unwrap(),
            Point3::try_new(x, -1.0, 1.0).unwrap(),
        ];
        for indices in [[0, 1, 2], [0, 2, 3]] {
            scene.add_triangle(Triangle3d::try_new(indices.map(|i| corners[i]), material).unwrap());
        }
    }
    for offset in [-0.01, 0.0, 0.01] {
        let (actual, occluded) =
            scene.direct_transmission(vector(-1.0, offset, 0.0), vector(1.0, offset, 0.0));
        assert!(occluded);
        assert_eq!(actual, gain.multiply(gain), "offset={offset}");
    }
}

/// Captured paths match the response query and stale outputs clear without losing capacity.
#[test]
fn trace_with_paths_reuses_storage_and_preserves_response() {
    let mut scene = AcousticScene3d::default();
    let reflector = Triangle3d::try_new(
        [
            Point3::try_new(0.0, -1.0, -1.0).expect("finite reflector vertex"),
            Point3::try_new(0.0, 1.0, -1.0).expect("finite reflector vertex"),
            Point3::try_new(0.0, 0.0, 1.0).expect("finite reflector vertex"),
        ],
        AcousticMaterial::default(),
    )
    .expect("nondegenerate reflector");
    scene.add_triangle(reflector);
    let emitter = Emitter3d::new(Point3::try_new(-1.0, 0.0, 0.0).expect("finite emitter"));
    let listener = Listener3d::new(Point3::try_new(-3.0, 0.0, 0.0).expect("finite listener"));
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

/// Makes finite test vertices in world-space meters.
fn vertices() -> [Point3; 3] {
    [
        Point3::try_new(0.0, -1.0, -1.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 1.0, -1.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 0.0, 1.0).expect("finite triangle vertex"),
    ]
}

/// Builds cached geometry from validated test vertices.
fn geometry(vertices: [Point3; 3]) -> TriangleGeometry {
    TriangleGeometry::new(
        Triangle3d::try_new(vertices, AcousticMaterial::default())
            .expect("test triangle is nondegenerate"),
    )
}

/// Triangle intersection distinguishes interior, parallel, outside, and endpoint paths.
#[test]
fn path_intersection_checks_triangle_and_open_segment() {
    let triangle = geometry(vertices());
    assert!(path_intersects_triangle(
        vector(-1.0, 0.0, 0.0),
        vector(1.0, 0.0, 0.0),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(-1.0, -2.0, 0.0),
        vector(-1.0, 2.0, 0.0),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(-1.0, 2.0, 0.0),
        vector(1.0, 2.0, 0.0),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(-1.0, 1.1, 0.4),
        vector(1.0, 1.1, 0.4),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(0.0, 0.0, 0.0),
        vector(1.0, 0.0, 0.0),
        &triangle,
    ));
}

/// Overlapping bounds exercise the plane-parallel and barycentric rejection paths.
#[test]
fn path_intersection_checks_parallel_and_outside_barycentric_paths() {
    let triangle = geometry(vertices());
    assert!(!path_intersects_triangle(
        vector(0.0, -2.0, 0.0),
        vector(0.0, 2.0, 0.0),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(-1.0, 0.0, -1.0),
        vector(1.0, 2.2, -1.0),
        &triangle,
    ));
    assert!(!path_intersects_triangle(
        vector(-1.0, 0.0, 0.0),
        vector(1.0, 2.2, 0.0),
        &triangle,
    ));
}

/// A valid triangle hit remains accepted inside the tolerant barycentric boundary.
#[test]
fn path_intersection_preserves_triangle_boundary_tolerance() {
    let y_offset = PARAMETER_EPSILON * 0.5;
    assert!(path_intersects_triangle(
        vector(-1.0, 1.0 + y_offset, -1.0),
        vector(1.0, 1.0 + y_offset, -1.0),
        &geometry(vertices()),
    ));
}

/// Triangle membership accepts a boundary point and rejects outside and degenerate bases.
#[test]
fn barycentric_membership_checks_edges_and_degenerate_basis() {
    let origin = vector(0.0, 0.0, 0.0);
    let edge_a = vector(1.0, 0.0, 0.0);
    let edge_b = vector(0.0, 1.0, 0.0);
    assert!(point_in_triangle(
        vector(0.5, 0.5, 0.0),
        origin,
        edge_a,
        edge_b
    ));
    assert!(!point_in_triangle(
        vector(1.1, 0.1, 0.0),
        origin,
        edge_a,
        edge_b
    ));
    assert!(!point_in_triangle(
        vector(0.0, 0.0, 0.0),
        origin,
        vector(0.0, 0.0, 0.0),
        edge_b,
    ));
}

/// Image-source reflection rejects paths whose point misses the finite triangle.
#[test]
fn image_reflection_rejects_a_point_outside_the_triangle() {
    let far_vertices = [
        Point3::try_new(0.0, 10.0, 0.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 12.0, 0.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 10.0, 2.0).expect("finite triangle vertex"),
    ];
    assert!(
        first_reflection(
            vector(-1.0, 0.0, 0.0),
            vector(-1.0, 0.0, 2.0),
            &geometry(far_vertices),
        )
        .is_none()
    );
}

/// The bounds fast path reaches barycentric rejection inside the triangle's padded box.
#[test]
fn image_reflection_rejects_a_point_inside_bounds_but_outside_triangle() {
    let triangle = geometry([
        Point3::try_new(0.0, 10.0, 0.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 12.0, 0.0).expect("finite triangle vertex"),
        Point3::try_new(0.0, 10.0, 2.0).expect("finite triangle vertex"),
    ]);
    let reflection_point = vector(0.0, 11.8, 1.8);
    let point_bounds = Bounds::path_3d(
        (reflection_point.x, reflection_point.y, reflection_point.z),
        (reflection_point.x, reflection_point.y, reflection_point.z),
    );

    assert!(triangle.bounds.overlaps(point_bounds));
    assert!(!triangle.contains_point(reflection_point));
    assert!(
        first_reflection(vector(-1.0, 11.8, 1.8), vector(-3.0, 11.8, 1.8), &triangle,).is_none()
    );
}

/// A near-parallel image ray cannot produce a stable finite-triangle reflection.
#[test]
fn image_reflection_rejects_a_near_parallel_ray() {
    assert!(
        first_reflection(
            vector(1.0, 0.0, 0.0),
            vector(1.0, 1.0e15, 2.0),
            &geometry(vertices()),
        )
        .is_none()
    );
}

/// A nonfinite private image-source intermediate cannot produce a reflection path.
#[test]
fn image_reflection_rejects_nonfinite_parameter() {
    let triangle = Triangle3d::try_new(vertices(), AcousticMaterial::default()).unwrap();
    assert!(
        first_reflection(
            vector(f64::NAN, 0.0, 0.0),
            vector(1.0, 0.0, 0.0),
            &TriangleGeometry::new(triangle)
        )
        .is_none()
    );
}

/// Every planar orientation's broad phase retains each exact image-source hit.
#[test]
fn reflection_broad_phase_has_no_false_negatives() {
    for axis in 0..3 {
        let mut scene = AcousticScene3d::default();
        for normal in [-5., -3., -1., 1., 3., 5.] {
            let points = match axis {
                0 => [(normal, -2., -2.), (normal, 2., -2.), (normal, 0., 2.)],
                1 => [(-2., normal, -2.), (2., normal, -2.), (0., normal, 2.)],
                _ => [(-2., -2., normal), (2., -2., normal), (0., 2., normal)],
            };
            scene.add_triangle(
                Triangle3d::try_new(
                    points.map(|(x, y, z)| Point3::try_new(x, y, z).unwrap()),
                    AcousticMaterial::default(),
                )
                .unwrap(),
            );
        }
        let acceleration = scene.acceleration();
        for i in -8..=8 {
            for j in -8..=8 {
                let source = vector(f64::from(i), f64::from(j), f64::from(i + j));
                let receiver = vector(f64::from(j) + 0.25, f64::from(i) - 0.25, f64::from(i - j));
                if !acceleration.hierarchy.may_have_reflections(Bounds::path_3d(
                    (source.x, source.y, source.z),
                    (receiver.x, receiver.y, receiver.z),
                )) {
                    assert!(
                        acceleration
                            .triangles
                            .iter()
                            .all(|wall| first_reflection(source, receiver, wall).is_none())
                    );
                }
            }
        }
    }
}

/// A full triangle query rejects distant parallel groups and preserves the direct response.
#[test]
fn distant_parallel_walls_have_no_reflection_output() {
    let mut scene = AcousticScene3d::default();
    for x in 0..8 {
        let x = f32::from(u16::try_from(x).unwrap());
        scene.add_triangle(
            Triangle3d::try_new(
                [
                    Point3::try_new(x, 100., 0.).unwrap(),
                    Point3::try_new(x, 101., 0.).unwrap(),
                    Point3::try_new(x, 100., 1.).unwrap(),
                ],
                AcousticMaterial::default(),
            )
            .unwrap(),
        );
    }
    let emitter = Emitter3d::new(Point3::try_new(-1., -1., 0.).unwrap());
    let listener = Listener3d::new(Point3::try_new(-2., 1., 0.).unwrap());
    assert_eq!(scene.reflection_paths(emitter, listener).count(), 0);
    assert_eq!(
        scene.trace(emitter, listener),
        AcousticScene3d::default().trace(emitter, listener)
    );
}
