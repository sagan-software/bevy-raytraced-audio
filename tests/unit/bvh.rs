//! Boundary tests for hierarchy construction and candidate traversal.

use super::{BoundingVolumeHierarchy, Bounds, SceneDimensions};

/// Predicate view over the production candidate visitor used by visibility assertions.
impl BoundingVolumeHierarchy {
    /// Returns whether an exact predicate accepts any overlapping candidate.
    fn any_intersection(
        &self,
        bounds: Bounds,
        skip: Option<usize>,
        mut test: impl FnMut(usize) -> bool,
    ) -> bool {
        !self.visit_candidates_until(bounds, skip, |index| !test(index))
    }
}

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
    let hierarchy = BoundingVolumeHierarchy::build(Vec::new(), SceneDimensions::Two);
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
        surface_index != 2
    });

    assert!(!completed);
    assert!(visited.len() > 1, "the stop must originate below the root");
    assert_eq!(visited.last(), Some(&2));
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

/// Tall surfaces with coincident y centers retain exact nearest hits after centroid partitioning.
#[test]
fn ray_queries_match_exhaustive_bounds_on_tall_surfaces() {
    use super::Ray;
    let surfaces: Vec<_> = (0..127)
        .map(|i| {
            let x = fixture_x(i % 17) - 8.;
            let z = fixture_x(i / 17) - 3.;
            Bounds::path_3d((x, -100., z), (x + 0.25, 100., z + 0.25))
        })
        .collect();
    let hierarchy = BoundingVolumeHierarchy::build(
        surfaces.iter().copied().enumerate(),
        SceneDimensions::Three,
    );
    for x in -12..12 {
        for z in -6..6 {
            for direction in [(1., 0., 0.), (-1., 0., 0.), (0., 0., 1.), (0.3, 0.7, -0.2)] {
                let ray = Ray::new((f64::from(x), 0., f64::from(z)), direction);
                let expected = surfaces
                    .iter()
                    .filter_map(|b| b.ray_entry(ray, 1000.))
                    .min_by(f64::total_cmp);
                let mut nearest: Option<f64> = None;
                hierarchy.visit_ray(ray, 1000., |index| {
                    let distance = surfaces.get(index).unwrap().ray_entry(ray, 1000.)?;
                    nearest = Some(nearest.map_or(distance, |old| old.min(distance)));
                    Some(distance)
                });
                assert_eq!(nearest, expected);
            }
        }
    }
}

/// Invalid ray traversal state and a terminated ray cannot invoke exact surface predicates.
#[test]
fn invalid_and_terminated_ray_visits_do_no_work() {
    let hierarchy = BoundingVolumeHierarchy::build([(0, planar_bounds(0.))], SceneDimensions::Two);
    let ray = super::Ray::new((0., 0., 0.), (1., 0., 0.));
    let mut callback = |_| panic!("no valid surface visit");
    hierarchy.visit_ray_node(usize::MAX, ray, &mut 1., &mut callback);
    hierarchy.visit_ray_node(hierarchy.root.unwrap(), ray, &mut -1., &mut callback);
    let mut broken = hierarchy.clone();
    broken.surface_indices.clear();
    broken.visit_ray_node(broken.root.unwrap(), ray, &mut 1., &mut callback);
}

/// One callback interface handles normal branches, early termination, and invalid storage.
#[test]
fn traversal_callbacks_cover_valid_and_invalid_nodes() {
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..16).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );
    let root = hierarchy.root.unwrap();
    let bounds = Bounds::path_2d((-1., 0.), (20., 0.));
    let ray = super::Ray::new((-1., 0., 0.), (1., 0., 0.));
    let mut ray_callback: fn(usize) -> Option<f64> = |_| None;
    hierarchy.visit_ray_node(usize::MAX, ray, &mut 100., &mut ray_callback);
    hierarchy.visit_ray_node(root, ray, &mut -1., &mut ray_callback);
    hierarchy.visit_ray_node(root, ray, &mut 100., &mut ray_callback);
    hierarchy.visit_ray_node(
        root,
        super::Ray::new((20., 0., 0.), (-1., 0., 0.)),
        &mut 100.,
        &mut ray_callback,
    );
    let mut broken = hierarchy.clone();
    broken.surface_indices.clear();
    broken.visit_ray_node(root, ray, &mut 100., &mut ray_callback);
    ray_callback = |_| Some(-1.);
    let mut maximum = 100.;
    hierarchy.visit_ray_node(root, ray, &mut maximum, &mut ray_callback);
    assert_eq!(maximum, -1.);

    let mut candidate_callback: fn(usize) -> bool = |_| true;
    assert!(hierarchy.visit_candidate_node(usize::MAX, bounds, None, &mut candidate_callback));
    assert!(hierarchy.visit_candidate_node(
        root,
        planar_bounds(100.),
        None,
        &mut candidate_callback
    ));
    assert!(hierarchy.visit_candidate_node(root, bounds, Some(0), &mut candidate_callback));
    assert!(broken.visit_candidate_node(root, bounds, None, &mut candidate_callback));
    candidate_callback = |_| false;
    assert!(!hierarchy.visit_candidate_node(root, bounds, None, &mut candidate_callback));
    candidate_callback = |index| index < 15;
    assert!(!hierarchy.visit_candidate_node(root, bounds, None, &mut candidate_callback));
}

/// Line traversal retains every exact bounds hit and propagates a visitor's early stop.
#[test]
fn segment_candidates_preserve_hits_and_early_exit() {
    for count in [1, 4, 5, 127] {
        let surfaces: Vec<_> = (0..count).map(|i| planar_bounds(fixture_x(i))).collect();
        let hierarchy = BoundingVolumeHierarchy::build(
            surfaces.iter().copied().enumerate(),
            SceneDimensions::Two,
        );
        let bounds = Bounds::path_2d((-1., 0.), (200., 0.));
        let ray = super::Ray::new((-1., 0., 0.), (201., 0., 0.));
        let mut visited = Vec::new();
        assert!(
            hierarchy.visit_segment_candidates(ray, bounds, Some(0), |i| {
                visited.push(i);
                true
            })
        );
        visited.sort_unstable();
        assert_eq!(visited, (1..count).collect::<Vec<_>>());
        let mut calls = 0;
        assert!(!hierarchy.visit_segment_candidates(ray, bounds, None, |_| {
            calls += 1;
            false
        }));
        assert_eq!(calls, 1);
    }
}

/// Parallel planar groups reject only tangential misses; mixed and oblique planes fall back.
#[test]
fn reflection_bounds_handle_every_axis_and_fallback() {
    assert!(
        !BoundingVolumeHierarchy::default()
            .may_have_reflections(Bounds::path_2d((0., 0.), (1., 1.)))
    );
    for axis in 0..3 {
        let make_bounds = |normal: f64| match axis {
            0 => Bounds::path_3d((normal, -2., -2.), (normal, 2., 2.)),
            1 => Bounds::path_3d((-2., normal, -2.), (2., normal, 2.)),
            _ => Bounds::path_3d((-2., -2., normal), (2., 2., normal)),
        };
        let hierarchy = BoundingVolumeHierarchy::build(
            (0_u32..8).map(|index| {
                (
                    usize::try_from(index).unwrap(),
                    make_bounds(f64::from(index) + 10.0),
                )
            }),
            SceneDimensions::Three,
        );
        assert!(hierarchy.may_have_reflections(Bounds::path_3d((-1., -1., -1.), (1., 1., 1.))));
        assert!(
            !hierarchy
                .may_have_reflections(Bounds::path_3d((100., 100., 100.), (101., 101., 101.)))
        );
        let mixed = BoundingVolumeHierarchy::build(
            (0_u32..8).map(|index| {
                (
                    usize::try_from(index).unwrap(),
                    if index == 0 {
                        make_bounds(10.)
                    } else {
                        Bounds::path_3d((-1., -1., -1.), (1., 1., 1.))
                    },
                )
            }),
            SceneDimensions::Three,
        );
        assert!(
            mixed.may_have_reflections(Bounds::path_3d((100., 100., 100.), (101., 101., 101.)))
        );
    }
    let oblique = BoundingVolumeHierarchy::build(
        (0..8).map(|index| (index, Bounds::path_3d((-1., -1., -1.), (1., 1., 1.)))),
        SceneDimensions::Three,
    );
    assert!(oblique.may_have_reflections(Bounds::path_3d((100., 100., 100.), (101., 101., 101.))));
}

/// The same dynamic input type supports empty, planar, oblique and mixed geometry.
#[test]
fn dynamic_surface_lists_cover_orientation_changes() {
    for (bounds, dimensions) in [
        (
            Bounds::path_3d((0., -1., -1.), (0., 1., 1.)),
            SceneDimensions::Three,
        ),
        (
            Bounds::path_3d((-1., 0., -1.), (1., 0., 1.)),
            SceneDimensions::Three,
        ),
        (
            Bounds::path_3d((-1., -1., 0.), (1., 1., 0.)),
            SceneDimensions::Three,
        ),
        (
            Bounds::path_3d((-1., -1., -1.), (1., 1., 1.)),
            SceneDimensions::Three,
        ),
        (Bounds::path_2d((-1., -1.), (1., 1.)), SceneDimensions::Two),
    ] {
        let surfaces: Vec<_> = (0..8).map(|index| (index, bounds)).collect();
        let tree = BoundingVolumeHierarchy::build(surfaces, dimensions);
        let mut visited = Vec::new();
        assert!(tree.visit_candidates_until(bounds, None, |index| {
            visited.push(index);
            true
        }));
        visited.sort_unstable();
        assert_eq!(visited, (0..8).collect::<Vec<_>>());
    }
}

/// A one-slot frontier and a direct descendant walk stop at the first accepted surface.
#[test]
fn any_hit_descendant_matches_a_one_slot_frontier() {
    use super::{NodeKind, Ray};
    let hierarchy = BoundingVolumeHierarchy::build(
        (0..6).map(|index| (index, planar_bounds(fixture_x(index)))),
        SceneDimensions::Two,
    );
    let ray = Ray::new((-1.0, 0.0, 0.0), (1.0, 0.0, 0.0));
    let root = hierarchy.root.expect("six surfaces form a branch");
    assert!(hierarchy.any_hit_stacked::<1>(root, ray, 100.0, None, &mut |_| true));
    assert!(!hierarchy.any_hit_stacked::<1>(root, ray, 100.0, None, &mut |_| false));
    assert!(!hierarchy.any_hit_stacked::<1>(usize::MAX, ray, 100.0, None, &mut |_| true));
    let leaf = hierarchy
        .nodes
        .iter()
        .position(|node| matches!(node.kind, NodeKind::Leaf { .. }))
        .expect("leaf");
    assert!(hierarchy.any_hit_descendant(leaf, ray, 100.0, None, &mut |_| true));
    assert!(!hierarchy.any_hit_descendant(leaf, ray, 100.0, None, &mut |_| false));
    let surfaces = match &hierarchy.nodes[leaf].kind {
        NodeKind::Leaf { indices } => hierarchy.surface_indices[indices.clone()].to_vec(),
        NodeKind::Branch { .. } => panic!("expected a leaf"),
    };
    assert!(
        !hierarchy.any_hit_descendant(leaf, ray, 100.0, Some(surfaces[0]), &mut |index| index
            == surfaces[0])
    );
    assert!(!hierarchy.any_hit_descendant(usize::MAX, ray, 100.0, None, &mut |_| true));
    let miss = Ray::new((-1.0, 50.0, 0.0), (1.0, 0.0, 0.0));
    assert!(!hierarchy.any_hit_descendant(root, miss, 100.0, None, &mut |_| true));
    assert!(hierarchy.any_hit_descendant(root, ray, 100.0, None, &mut |_| true));
    let mut broken = hierarchy;
    broken.surface_indices.clear();
    assert!(!broken.any_hit_descendant(leaf, ray, 100.0, None, &mut |_| true));
    assert!(!broken.any_hit_stacked::<1>(root, ray, 100.0, None, &mut |_| true));
}

/// A 0.2 m box placed on a 0.3 m stride so neighboring fixtures do not overlap.
fn compact_bounds(index: u32) -> Bounds {
    let x = f64::from(index) * 0.3;
    Bounds::path_3d((x, 0.0, 0.0), (x + 0.2, 0.2, 0.2))
}

/// Room-scale shells leave the tree. Uniform, degenerate, and leaf-sized sets do not.
#[test]
fn only_extent_outliers_leave_the_tree() {
    let wall = Bounds::path_3d((-10.0, -10.0, -10.0), (10.0, -10.0, 10.0));
    let mut mixed = vec![(0_usize, wall)];
    mixed.extend((1..17_u32).map(|index| (usize::try_from(index).unwrap(), compact_bounds(index))));
    assert_eq!(
        BoundingVolumeHierarchy::build(mixed, SceneDimensions::Three).large_surface_count(),
        1
    );

    let uniform = (0..16_u32).map(|index| (usize::try_from(index).unwrap(), compact_bounds(index)));
    assert_eq!(
        BoundingVolumeHierarchy::build(uniform, SceneDimensions::Three).large_surface_count(),
        0
    );

    let few = [
        (0_usize, wall),
        (1, compact_bounds(1)),
        (2, compact_bounds(2)),
    ];
    assert_eq!(
        BoundingVolumeHierarchy::build(few, SceneDimensions::Three).large_surface_count(),
        0
    );

    let points = (0..8_u32).map(|index| {
        let x = f64::from(index);
        (
            usize::try_from(index).unwrap(),
            Bounds::path_3d((x, 0.0, 0.0), (x, 0.0, 0.0)),
        )
    });
    assert_eq!(
        BoundingVolumeHierarchy::build(points, SceneDimensions::Three).large_surface_count(),
        0
    );
}

/// Depth counts only in a three-dimensional scene.
#[test]
fn extent_outliers_follow_the_scene_axes() {
    let long_depth = Bounds::path_3d((0.0, 0.0, -10.0), (0.2, 0.2, 10.0));
    let surfaces: Vec<_> = (0..16_usize)
        .map(|index| {
            let bounds = if index == 0 {
                long_depth
            } else {
                compact_bounds(u32::try_from(index).unwrap())
            };
            (index, bounds)
        })
        .collect();
    let planar = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Two);
    let spatial = BoundingVolumeHierarchy::build(surfaces, SceneDimensions::Three);

    assert_eq!(planar.large_surface_count(), 0);
    assert_eq!(spatial.large_surface_count(), 1);
}

/// A side-listed wall and a prop inside the tree both stay visible to a ray.
#[test]
fn large_and_small_surfaces_both_remain_visible() {
    use super::Ray;
    let wall = Bounds::path_3d((-10.0, -10.0, -10.0), (10.0, -10.0, 10.0));
    let mut surfaces = vec![(0_usize, wall)];
    surfaces
        .extend((1..17_u32).map(|index| (usize::try_from(index).unwrap(), compact_bounds(index))));
    let hierarchy = BoundingVolumeHierarchy::build(surfaces, SceneDimensions::Three);
    let wall_ray = Ray::new((0.0, -5.0, 5.0), (0.0, -1.0, 0.0));
    let wall_path = Bounds::path_3d((0.0, -5.0, 5.0), (0.0, -10.0, 5.0));

    assert!(hierarchy.any_hit(wall_ray, wall_path, 20.0, None, |index| index == 0));
    assert!(!hierarchy.any_hit(wall_ray, wall_path, 20.0, Some(0), |_| true));
    let prop_ray = Ray::new((-1.0, 0.1, 0.1), (1.0, 0.0, 0.0));
    let prop_path = Bounds::path_3d((-1.0, 0.1, 0.1), (0.5, 0.1, 0.1));
    assert!(hierarchy.any_hit(prop_ray, prop_path, 20.0, None, |index| index == 1));

    let mut seen = Vec::new();
    hierarchy.visit_ray(wall_ray, 20.0, |index| {
        seen.push(index);
        None
    });
    assert_eq!(seen, vec![0]);
    assert!(
        hierarchy
            .root_bounds()
            .is_some_and(|root| root.contains(wall))
    );
}

/// A visibility segment only consults a side-listed wall its bounding box reaches.
#[test]
fn visibility_skips_a_wall_the_path_box_stops_short_of() {
    use super::Ray;
    let wall = Bounds::path_3d((-10.0, -10.0, -10.0), (10.0, -10.0, 10.0));
    let mut surfaces = vec![(0_usize, wall)];
    surfaces
        .extend((1..17_u32).map(|index| (usize::try_from(index).unwrap(), compact_bounds(index))));
    let hierarchy = BoundingVolumeHierarchy::build(surfaces, SceneDimensions::Three);
    let ray = Ray::new((0.0, -5.0, 5.0), (0.0, -1.0, 0.0));
    let wall_tests = core::cell::Cell::new(0_u32);
    let test = |index: usize| {
        wall_tests.set(wall_tests.get() + u32::from(index == 0));
        index == 0
    };
    let short = Bounds::path_3d((0.0, -5.0, 5.0), (0.0, -9.0, 5.0));
    assert!(!hierarchy.any_hit(ray, short, 20.0, None, test));
    assert_eq!(wall_tests.get(), 0);
    // A box that touches the wall plane still tests the wall.
    let touching = Bounds::path_3d((0.0, -5.0, 5.0), (0.0, -10.0, 5.0));
    assert!(hierarchy.any_hit(ray, touching, 5.0, None, test));
    assert_eq!(wall_tests.get(), 1);
}

/// A side-listed parallel wall still reflects outside the prop cluster and rejects a far miss.
#[test]
fn a_large_parallel_wall_still_reflects_outside_the_prop_cluster() {
    let wall = Bounds::path_3d((-10.0, 0.0, -10.0), (10.0, 0.0, 10.0));
    let mut surfaces = vec![(0_usize, wall)];
    surfaces.extend((1..17_u32).map(|index| {
        let x = f64::from(index) * 0.3;
        (
            usize::try_from(index).unwrap(),
            Bounds::path_3d((x, 0.0, 0.0), (x + 0.2, 0.0, 0.2)),
        )
    }));
    let hierarchy = BoundingVolumeHierarchy::build(surfaces, SceneDimensions::Three);
    let off_plane = Bounds::path_3d((9.0, 1.0, 9.0), (9.5, 1.0, 9.5));
    let on_plane = Bounds::path_3d((9.0, 0.0, 9.0), (9.5, 0.0, 9.5));
    let far_miss = Bounds::path_3d((100.0, 0.0, 100.0), (101.0, 0.0, 101.0));

    assert_eq!(hierarchy.large_surface_count(), 1);
    assert!(hierarchy.may_have_reflections(off_plane));
    assert!(!hierarchy.may_have_reflections(far_miss));
    let mut seen = Vec::new();
    assert!(hierarchy.visit_candidates_until(on_plane, None, |index| {
        seen.push(index);
        true
    }));
    assert_eq!(seen, vec![0]);
}

/// Thin axis-aligned flags on a 0.25 m x-z lattice, indexed as `column * rows + row`.
fn flag_field(columns: u32, rows: u32) -> Vec<(usize, Bounds)> {
    let mut surfaces = Vec::new();
    for column in 0..columns {
        for row in 0..rows {
            let x = f64::from(column) * 0.25;
            let z = f64::from(row) * 0.25;
            surfaces.push((
                surfaces.len(),
                Bounds::path_3d((x, -0.12, z), (x, 0.12, z + 0.24)),
            ));
        }
    }
    surfaces
}

/// Returns whether `index` is a surface whose own bounds meet the ray before `limit`.
fn bounds_accept(surfaces: &[(usize, Bounds)], ray: super::Ray, limit: f64, index: usize) -> bool {
    surfaces
        .iter()
        .any(|(candidate, bounds)| *candidate == index && bounds.ray_entry(ray, limit).is_some())
}

/// Brute-force any-hit over primitive bounds, ignoring leaf padding.
fn bounds_oracle(
    surfaces: &[(usize, Bounds)],
    ray: super::Ray,
    limit: f64,
    skip: Option<usize>,
) -> bool {
    surfaces
        .iter()
        .any(|(index, bounds)| skip != Some(*index) && bounds.ray_entry(ray, limit).is_some())
}

/// Runs the production visibility query with the bounds oracle as its exact test.
fn query_bounds(
    hierarchy: &BoundingVolumeHierarchy,
    surfaces: &[(usize, Bounds)],
    ray: super::Ray,
    limit: f64,
    skip: Option<usize>,
) -> bool {
    // Large trees ignore this box; the grid and the slabs decide.
    let path = Bounds::path_3d((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    hierarchy.any_hit(ray, path, limit, skip, |index| {
        bounds_accept(surfaces, ray, limit, index)
    })
}

/// A uniform grid exists only for a large 3D prop set. Planar scenes keep the tree.
#[test]
fn uniform_grid_is_only_built_for_a_large_three_dimensional_prop_set() {
    let surfaces = flag_field(8, 8);
    let spatial = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Three);
    let planar = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Two);
    let few = BoundingVolumeHierarchy::build(flag_field(3, 3), SceneDimensions::Three);
    let ray = super::Ray::new((0.1, 0.0, 0.1), (1.0, 0.0, 0.0));
    let path = Bounds::path_3d((0.1, 0.0, 0.1), (0.4, 0.0, 0.1));

    assert!(spatial.small_grid.is_some());
    assert!(planar.small_grid.is_none());
    assert!(few.small_grid.is_none());
    assert!(planar.any_hit(ray, path, 0.3, None, |index| {
        bounds_accept(&surfaces, ray, 0.3, index)
    }));
}

/// Short overlaps agree with a per-primitive bounds test, including the lattice edges.
#[test]
fn narrow_grid_queries_match_primitive_bounds() {
    let surfaces = flag_field(8, 8);
    let hierarchy = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Three);
    let grid = hierarchy
        .small_grid
        .as_ref()
        .expect("64 flags build a grid");
    let rays = [
        super::Ray::new((0.1, 0.0, 0.1), (1.0, 0.0, 0.0)),
        super::Ray::new((1.5, 0.0, 1.8), (1.0, 0.0, 0.0)),
        super::Ray::new((-0.5, 0.0, 0.1), (1.0, 0.0, 0.0)),
        super::Ray::new((0.125, 0.0, -0.2), (0.0, 0.0, 1.0)),
        super::Ray::new((0.1, 0.0, 0.1), (1.0, 0.0, 1.0)),
    ];
    let limits = [0.3_f64, 0.4, 1.0, 0.5, 0.2];

    for (ray, limit) in rays.into_iter().zip(limits) {
        let mut accepts = |index| bounds_accept(&surfaces, ray, limit, index);
        let answer = grid.query(ray, limit, None, &mut accepts);
        assert_ne!(
            answer,
            super::GridAnswer::Wide,
            "fixture rays stay inside eight cells"
        );
        assert_eq!(
            query_bounds(&hierarchy, &surfaces, ray, limit, None),
            bounds_oracle(&surfaces, ray, limit, None)
        );
    }
    let interior = super::Ray::new((0.1, 0.0, 0.1), (1.0, 0.0, 0.0));
    assert!(bounds_oracle(&surfaces, interior, 0.3, None));
    assert!(!query_bounds(&hierarchy, &surfaces, interior, 0.3, Some(8)));
}

/// A long in-slab segment is handed back to the tree and still finds the blocker.
#[test]
fn wide_grid_queries_fall_back_to_the_tree() {
    let surfaces = flag_field(16, 16);
    let hierarchy = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Three);
    let grid = hierarchy
        .small_grid
        .as_ref()
        .expect("256 flags build a grid");
    let ray = super::Ray::new((-1.0, 0.0, 0.1), (1.0, 0.0, 0.0));
    let mut accepts = |index| bounds_accept(&surfaces, ray, 20.0, index);

    assert_eq!(
        grid.query(ray, 20.0, None, &mut accepts),
        super::GridAnswer::Wide
    );
    assert_eq!(
        query_bounds(&hierarchy, &surfaces, ray, 20.0, None),
        bounds_oracle(&surfaces, ray, 20.0, None)
    );
    assert!(bounds_oracle(&surfaces, ray, 20.0, None));
}

/// A ray outside the prop slab is a miss and does not inspect prop surfaces.
#[test]
fn a_ray_outside_the_prop_slab_does_not_scan_props() {
    let surfaces = flag_field(8, 8);
    let hierarchy = BoundingVolumeHierarchy::build(surfaces, SceneDimensions::Three);
    let grid = hierarchy
        .small_grid
        .as_ref()
        .expect("64 flags build a grid");
    let ray = super::Ray::new((0.1, 1.0, 0.1), (1.0, 0.0, 0.0));
    let path = Bounds::path_3d((0.1, 1.0, 0.1), (1.1, 1.0, 0.1));

    assert_eq!(
        grid.query(ray, 1.0, None, &mut |_| panic!(
            "a slab miss must not test a prop"
        )),
        super::GridAnswer::Clear
    );
    assert!(!hierarchy.any_hit(ray, path, 1.0, None, |_| {
        panic!("a slab miss must not test a prop")
    }));
}

/// A side-listed wall is tested before the grid, and a narrow prop hit still resolves.
#[test]
fn side_listed_walls_stay_ahead_of_the_prop_grid() {
    let wall = Bounds::path_3d((-10.0, -10.0, -10.0), (10.0, -10.0, 10.0));
    let mut surfaces = vec![(0_usize, wall)];
    surfaces.extend(
        flag_field(8, 8)
            .into_iter()
            .map(|(index, bounds)| (index + 1, bounds)),
    );
    let hierarchy = BoundingVolumeHierarchy::build(surfaces.clone(), SceneDimensions::Three);
    let wall_ray = super::Ray::new((0.0, -5.0, 0.0), (0.0, -1.0, 0.0));
    let wall_path = Bounds::path_3d((0.0, -5.0, 0.0), (0.0, -10.0, 0.0));
    let prop_ray = super::Ray::new((0.1, 0.0, 0.1), (1.0, 0.0, 0.0));

    assert_eq!(hierarchy.large_surface_count(), 1);
    assert!(hierarchy.small_grid.is_some());
    assert!(hierarchy.any_hit(wall_ray, wall_path, 20.0, None, |index| index == 0));
    assert_eq!(
        query_bounds(&hierarchy, &surfaces, prop_ray, 0.3, None),
        bounds_oracle(&surfaces, prop_ray, 0.3, None)
    );
    assert!(bounds_oracle(&surfaces, prop_ray, 0.3, None));
}
