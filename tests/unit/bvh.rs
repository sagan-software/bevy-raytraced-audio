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

/// Candidate traversal visits overlapping branch and leaf surfaces and honors skips in both.
#[test]
fn candidate_visitation_covers_all_overlapping_nodes() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );
    let query_bounds = Bounds::path_2d((-1.0, -1.0), (6.0, 1.0));

    for skipped_surface in [None, Some(0), Some(3)] {
        let mut candidates = Vec::new();
        let completed =
            hierarchy.visit_candidates_until(query_bounds, skipped_surface, |surface_index| {
                candidates.push(surface_index);
                true
            });
        candidates.sort_unstable();

        let expected = (0..6)
            .filter(|surface_index| Some(*surface_index) != skipped_surface)
            .collect::<Vec<_>>();
        assert!(completed);
        assert_eq!(candidates, expected);
    }
}

/// Candidate traversal stops as soon as its visitor reports that no more surfaces are needed.
#[test]
fn candidate_visitation_can_stop_early() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );
    let query_bounds = Bounds::path_2d((-1.0, -1.0), (6.0, 1.0));
    let mut visited = Vec::new();

    let completed = hierarchy.visit_candidates_until(query_bounds, None, |surface_index| {
        visited.push(surface_index);
        false
    });

    assert!(!completed);
    assert_eq!(visited.len(), 1);
}

/// A stop request from a nested child propagates to the root before sibling traversal.
#[test]
fn candidate_visitation_propagates_a_nested_stop() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );
    let query_bounds = Bounds::path_2d((-1.0, -1.0), (6.0, 1.0));
    let mut visited = Vec::new();

    let completed = hierarchy.visit_candidates_until(query_bounds, None, |surface_index| {
        visited.push(surface_index);
        surface_index != 0
    });

    assert!(!completed);
    assert!(visited.len() > 1, "the stop must originate below the root");
    assert_eq!(visited.last(), Some(&0));
    assert!(
        !visited.contains(&5),
        "the right sibling must not be visited after the stop"
    );
}

/// Invalid node indices complete candidate traversal without calling its visitor.
#[test]
fn candidate_visitation_ignores_an_invalid_node_index() {
    let hierarchy = BoundingVolumeHierarchy::build([(0, planar_bounds(0.0))], SceneDimensions::Two);
    let mut visitor_calls = 0;

    let completed =
        hierarchy.visit_candidate_node(usize::MAX, planar_bounds(0.0), None, &mut |_| {
            visitor_calls += 1;
            true
        });

    assert!(completed);
    assert_eq!(visitor_calls, 0);
}

/// An invalid private leaf range completes traversal without calling its visitor.
#[test]
fn candidate_visitation_ignores_an_invalid_leaf_range() {
    let mut hierarchy =
        BoundingVolumeHierarchy::build([(0, planar_bounds(0.0))], SceneDimensions::Two);
    let root = hierarchy.root.expect("one surface creates a root leaf");
    hierarchy.surface_indices.clear();
    let mut visitor_calls = 0;

    let completed = hierarchy.visit_candidate_node(root, planar_bounds(0.0), None, &mut |_| {
        visitor_calls += 1;
        true
    });

    assert!(completed);
    assert_eq!(visitor_calls, 0);
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

/// A visibility hit stops even when the ray origin lies inside every node bound.
#[test]
fn negative_ray_hit_stops_all_remaining_primitives() {
    use super::Ray;
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..64).map(|i| (i, planar_bounds(0.))),
        SceneDimensions::Two,
    );
    let mut calls = 0;
    hierarchy.visit_ray(Ray::new((0., 0., 0.), (1., 0., 0.)), 100., |_| {
        calls += 1;
        Some(-1.)
    });
    assert_eq!(calls, 1);
}
