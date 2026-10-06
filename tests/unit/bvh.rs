//! Boundary tests for hierarchy construction and candidate traversal.

use super::{BoundingVolumeHierarchy, Bounds, SceneDimensions};

/// A unit-sized planar bounds fixture centered at the requested x coordinate.
fn planar_bounds(center_x: f64) -> Bounds {
    Bounds::path_2d((center_x - 0.25, -0.25), (center_x + 0.25, 0.25))
}

/// Converts a small fixture identifier to a coordinate without a precision-losing cast.
fn fixture_x(surface_index: usize) -> f64 {
    f64::from(u32::try_from(surface_index).expect("fixture surface indices fit in u32"))
}

/// An empty hierarchy has no candidate surfaces.
#[test]
fn empty_hierarchy_has_no_intersections() {
    let hierarchy = BoundingVolumeHierarchy::build([], SceneDimensions::Two);
    let mut predicate_calls = 0;

    let intersects =
        hierarchy.any_intersection(Bounds::path_2d((0.0, 0.0), (1.0, 0.0)), None, |_| {
            predicate_calls += 1;
            true
        });

    assert!(!intersects);
    assert_eq!(predicate_calls, 0);
}

/// A balanced branch tests its median and only overlapping left or right children.
#[test]
fn branches_find_surfaces_at_the_median_and_in_both_children() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );

    for surface_index in [0, 3, 5] {
        let intersects = hierarchy.any_intersection(
            planar_bounds(fixture_x(surface_index)),
            None,
            |candidate| candidate == surface_index,
        );
        assert!(intersects);
    }
}

/// Disjoint node bounds and exact misses both return false.
#[test]
fn bounds_and_exact_predicate_reject_misses() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );

    let disjoint = hierarchy.any_intersection(planar_bounds(20.0), None, |_| {
        panic!("a disjoint tree node cannot invoke the exact predicate")
    });
    let exact_miss = hierarchy.any_intersection(planar_bounds(2.0), None, |_| false);

    assert!(!disjoint);
    assert!(!exact_miss);
}

/// Skipping one reflector prevents its exact intersection from blocking either path leg.
#[test]
fn skipped_surface_does_not_reach_exact_predicate() {
    let hierarchy = BoundingVolumeHierarchy::build([(9, planar_bounds(0.0))], SceneDimensions::Two);
    let mut predicate_calls = 0;

    let intersects = hierarchy.any_intersection(planar_bounds(0.0), Some(9), |_| {
        predicate_calls += 1;
        true
    });

    assert!(!intersects);
    assert_eq!(predicate_calls, 0);
}

/// The widest z axis is included only for a three-dimensional tree.
#[test]
fn split_axis_respects_scene_dimension() {
    let bounds = Bounds::path_3d((0.0, 0.0, -10.0), (1.0, 1.0, 10.0));

    assert!(matches!(
        bounds.widest_axis(SceneDimensions::Two),
        super::Axis::X
    ));
    assert!(matches!(
        bounds.widest_axis(SceneDimensions::Three),
        super::Axis::Z
    ));
}

/// Three-dimensional partitions sort primitive centers on the z coordinate.
#[test]
fn three_dimensional_build_partitions_on_z() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| {
            let z = fixture_x(index);
            (index, Bounds::path_3d((0.0, 0.0, z), (1.0, 1.0, z + 0.5)))
        }),
        SceneDimensions::Three,
    );

    assert!(hierarchy.any_intersection(
        Bounds::path_3d((0.0, 0.0, 2.0), (1.0, 1.0, 2.5)),
        None,
        |surface_index| surface_index == 2,
    ));
}

/// Invalid internal node indices return false without invoking the surface predicate.
#[test]
fn invalid_node_index_is_not_an_intersection() {
    let hierarchy = BoundingVolumeHierarchy::build([(0, planar_bounds(0.0))], SceneDimensions::Two);

    assert!(
        !hierarchy.intersects_node(usize::MAX, planar_bounds(0.0), None, &mut |_| panic!(
            "an invalid node index has no surface candidates"
        ),)
    );
}
