//! Cached balanced bounds hierarchy used for exact segment and triangle visibility tests.
//!
//! Short 3D visibility queries over the small primitives can answer from a uniform grid. Long
//! overlaps, closest-hit queries, and every 2D scene keep the binary tree.

use core::ops::Range;

/// Maximum primitive count stored directly in one leaf.
const MAX_LEAF_SURFACES: usize = 4;

/// Fixed node stack for explicit any-hit traversal.
///
/// Median splits halve the primitive count per level and leaves hold at most
/// [`MAX_LEAF_SURFACES`], so the depth of any tree over allocatable memory is far below half
/// this capacity; the any-hit fallback recursion covers the theoretical remainder.
const MAX_TRAVERSAL_STACK: usize = 128;

/// A primitive longer than this multiple of the median extent is stored beside the tree.
///
/// The factor keeps a room shell out of every ancestor while a uniform set of walls stays in
/// the tree and keeps the parallel-wall reflection cull.
const LARGE_PRIMITIVE_EXTENT_FACTOR: f64 = 8.0;

/// Fewest small primitives that justify a uniform visibility grid.
///
/// Below this count the binary tree is only a few nodes deep, so the extra index does not
/// remove a walk.
const MIN_GRID_SURFACES: usize = 16;

/// Widest cell rectangle answered by a direct grid scan.
///
/// A segment that only crosses the prop slab covers a handful of cells. Longer overlaps stay
/// on the binary tree so a scan cannot walk the whole lattice.
const MAX_DIRECT_GRID_CELLS: usize = 8;

/// Largest lattice stored beside the tree.
///
/// A denser set keeps the binary tree. The dense listener fixture is about one thousand cells.
const MAX_GRID_CELLS: usize = 4_096;

/// Number of coordinate axes in the planar acoustic scene.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SceneDimensions {
    /// Surface and path coordinates use x and y.
    Two,
    /// Surface and path coordinates use x, y, and z.
    Three,
}

/// One coordinate axis used to split scene bounds.
#[derive(Clone, Copy, Debug)]
enum Axis {
    /// Horizontal x coordinate.
    X,
    /// Vertical y coordinate.
    Y,
    /// Depth z coordinate.
    Z,
}

/// Axis-aligned bounds for one surface or open path segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    /// Smallest x coordinate in the bounds.
    min_x: f64,
    /// Smallest y coordinate in the bounds.
    min_y: f64,
    /// Smallest z coordinate in the bounds.
    min_z: f64,
    /// Largest x coordinate in the bounds.
    max_x: f64,
    /// Largest y coordinate in the bounds.
    max_y: f64,
    /// Largest z coordinate in the bounds.
    max_z: f64,
}

impl Bounds {
    /// Builds bounds for a 2D surface with the same padding used by the exact segment test.
    pub(crate) fn segment_surface_2d(
        start: (f64, f64),
        end: (f64, f64),
        parameter_epsilon: f64,
    ) -> Self {
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;
        let padding_x = parameter_epsilon * (end_x - start_x).abs();
        let padding_y = parameter_epsilon * (end_y - start_y).abs();

        Self {
            min_x: start_x.min(end_x) - padding_x,
            min_y: start_y.min(end_y) - padding_y,
            min_z: 0.0,
            max_x: start_x.max(end_x) + padding_x,
            max_y: start_y.max(end_y) + padding_y,
            max_z: 0.0,
        }
    }

    /// Builds bounds for a 3D surface with the same padding used by the exact triangle test.
    pub(crate) fn triangle_surface_3d(
        vertices: [(f64, f64, f64); 3],
        parameter_epsilon: f64,
    ) -> Self {
        let [
            (first_x, first_y, first_z),
            (second_x, second_y, second_z),
            (third_x, third_y, third_z),
        ] = vertices;
        let to_second_x = second_x - first_x;
        let to_second_y = second_y - first_y;
        let to_second_z = second_z - first_z;
        let to_third_x = third_x - first_x;
        let to_third_y = third_y - first_y;
        let to_third_z = third_z - first_z;
        let padding_scale = 2.0 * parameter_epsilon;
        let padding_x = padding_scale * (to_second_x.abs() + to_third_x.abs());
        let padding_y = padding_scale * (to_second_y.abs() + to_third_y.abs());
        let padding_z = padding_scale * (to_second_z.abs() + to_third_z.abs());

        Self {
            min_x: first_x.min(second_x).min(third_x) - padding_x,
            min_y: first_y.min(second_y).min(third_y) - padding_y,
            min_z: first_z.min(second_z).min(third_z) - padding_z,
            max_x: first_x.max(second_x).max(third_x) + padding_x,
            max_y: first_y.max(second_y).max(third_y) + padding_y,
            max_z: first_z.max(second_z).max(third_z) + padding_z,
        }
    }

    /// Builds unpadded bounds for a 2D path's endpoints.
    pub(crate) const fn path_2d(start: (f64, f64), end: (f64, f64)) -> Self {
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;

        Self {
            min_x: start_x.min(end_x),
            min_y: start_y.min(end_y),
            min_z: 0.0,
            max_x: start_x.max(end_x),
            max_y: start_y.max(end_y),
            max_z: 0.0,
        }
    }

    /// Builds unpadded bounds for a 3D path's endpoints.
    pub(crate) const fn path_3d(start: (f64, f64, f64), end: (f64, f64, f64)) -> Self {
        let (start_x, start_y, start_z) = start;
        let (end_x, end_y, end_z) = end;

        Self {
            min_x: start_x.min(end_x),
            min_y: start_y.min(end_y),
            min_z: start_z.min(end_z),
            max_x: start_x.max(end_x),
            max_y: start_y.max(end_y),
            max_z: start_z.max(end_z),
        }
    }

    /// Returns the ray parameter where a ray enters these closed bounds, if before the limit.
    #[inline]
    fn ray_entry(self, ray: Ray, maximum_parameter: f64) -> Option<f64> {
        let mut entry = 0.0_f64;
        let mut exit = maximum_parameter;
        for (origin, inverse_direction, minimum, maximum) in [
            (
                ray.origin.0,
                ray.inverse_direction.0,
                self.min_x,
                self.max_x,
            ),
            (
                ray.origin.1,
                ray.inverse_direction.1,
                self.min_y,
                self.max_y,
            ),
            (
                ray.origin.2,
                ray.inverse_direction.2,
                self.min_z,
                self.max_z,
            ),
        ] {
            if inverse_direction.is_infinite() {
                // A ray parallel to this slab must already lie between its planes.
                if origin < minimum || origin > maximum {
                    return None;
                }
                continue;
            }
            let first = (minimum - origin) * inverse_direction;
            let second = (maximum - origin) * inverse_direction;
            // Plain comparisons avoid the NaN and signed-zero handling of `f64::min` and
            // `f64::max`; every operand here is finite, and this runs for each visited node.
            let (near, far) = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            if near > entry {
                entry = near;
            }
            if far < exit {
                exit = far;
            }
            if entry > exit {
                return None;
            }
        }
        Some(entry)
    }

    /// Returns the entry and exit parameters of the closed overlap, when the ray meets it.
    #[inline]
    fn ray_overlap(self, ray: Ray, maximum_parameter: f64) -> Option<(f64, f64)> {
        let mut entry = 0.0_f64;
        let mut exit = maximum_parameter;
        let axes = [
            (
                ray.origin.0,
                ray.inverse_direction.0,
                self.min_x,
                self.max_x,
            ),
            (
                ray.origin.1,
                ray.inverse_direction.1,
                self.min_y,
                self.max_y,
            ),
            (
                ray.origin.2,
                ray.inverse_direction.2,
                self.min_z,
                self.max_z,
            ),
        ];
        for (origin, inverse_direction, minimum, maximum) in axes {
            if !tighten_slab(
                &mut entry,
                &mut exit,
                origin,
                inverse_direction,
                minimum,
                maximum,
            ) {
                return None;
            }
        }
        Some((entry, exit))
    }

    /// Combines two bounds into the smallest bounds containing both.
    const fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            min_z: self.min_z.min(other.min_z),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
            max_z: self.max_z.max(other.max_z),
        }
    }

    /// Builds the cube that contains a sphere, for conservative candidate queries.
    pub(crate) fn sphere_3d(center: (f64, f64, f64), radius: f64) -> Self {
        let (x, y, z) = center;
        Self {
            min_x: x - radius,
            min_y: y - radius,
            min_z: z - radius,
            max_x: x + radius,
            max_y: y + radius,
            max_z: z + radius,
        }
    }

    /// Returns whether two closed bounds overlap on all three stored axes.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && other.min_x <= self.max_x
            && self.min_y <= other.max_y
            && other.min_y <= self.max_y
            && self.min_z <= other.max_z
            && other.min_z <= self.max_z
    }

    /// Returns whether these bounds completely contain `other` on all three stored axes.
    pub(crate) fn contains(self, other: Self) -> bool {
        self.min_x <= other.min_x
            && other.max_x <= self.max_x
            && self.min_y <= other.min_y
            && other.max_y <= self.max_y
            && self.min_z <= other.min_z
            && other.max_z <= self.max_z
    }

    /// Returns the midpoint coordinate on one split axis without overflowing on finite inputs.
    const fn center(self, axis: Axis) -> f64 {
        let (minimum, maximum) = match axis {
            Axis::X => (self.min_x, self.max_x),
            Axis::Y => (self.min_y, self.max_y),
            Axis::Z => (self.min_z, self.max_z),
        };
        f64::midpoint(minimum, maximum)
    }

    /// Returns the length of one stored axis.
    fn extent(self, axis: Axis) -> f64 {
        let (minimum, maximum) = match axis {
            Axis::X => (self.min_x, self.max_x),
            Axis::Y => (self.min_y, self.max_y),
            Axis::Z => (self.min_z, self.max_z),
        };
        maximum - minimum
    }

    /// Chooses the widest coordinate axis, resolving equal extents in x-y-z order.
    fn widest_axis(self, dimensions: SceneDimensions) -> Axis {
        let x_extent = self.extent(Axis::X);
        let y_extent = self.extent(Axis::Y);
        let z_extent = self.extent(Axis::Z);

        match dimensions {
            SceneDimensions::Two if x_extent >= y_extent => Axis::X,
            SceneDimensions::Two => Axis::Y,
            SceneDimensions::Three if x_extent >= y_extent && x_extent >= z_extent => Axis::X,
            SceneDimensions::Three if y_extent >= z_extent => Axis::Y,
            SceneDimensions::Three => Axis::Z,
        }
    }

    /// Returns the longest axis that this scene dimension stores.
    fn longest_extent(self, dimensions: SceneDimensions) -> f64 {
        let planar = self.extent(Axis::X).max(self.extent(Axis::Y));
        match dimensions {
            SceneDimensions::Two => planar,
            SceneDimensions::Three => planar.max(self.extent(Axis::Z)),
        }
    }
}

/// Bounds and primitive indices arranged into a balanced binary tree.
///
/// A primitive much larger than the median extent is stored beside the tree and tested on its
/// own. One room shell then does not enlarge every ancestor of the remaining props. Numerous
/// remaining props also get a uniform grid for short visibility queries.
#[derive(Clone, Debug, Default)]
pub(crate) struct BoundingVolumeHierarchy {
    /// Tree nodes stored in post-order so child bounds are already available during construction.
    nodes: Vec<Node>,
    /// Primitive indices stored contiguously for leaf ranges.
    surface_indices: Vec<usize>,
    /// Root node index, or `None` for an empty scene.
    root: Option<usize>,
    /// Common plane normal for parallel axis-aligned surfaces, if the scene has one.
    parallel_axis: Option<Axis>,
    /// Room-scale primitives kept out of the tree so their bounds do not inflate ancestors.
    large_surfaces: Vec<PrimitiveBounds>,
    /// Uniform grid over the small primitives, when a 3D lattice is cheaper than the tree.
    small_grid: Option<SmallGrid>,
}

impl BoundingVolumeHierarchy {
    /// Builds a balanced tree while preserving original surface indices in every node.
    ///
    /// Room-scale outliers are stored beside the tree. A uniform set, including a room made
    /// only of large walls, keeps the same partition order as a tree without that side list.
    #[expect(
        clippy::float_cmp,
        reason = "reflection rejection requires exactly planar bounds; a tolerance would admit tilted walls"
    )]
    pub(crate) fn build(
        surfaces: impl IntoIterator<Item = (usize, Bounds)>,
        dimensions: SceneDimensions,
    ) -> Self {
        let mut primitives: Vec<_> = surfaces
            .into_iter()
            .map(|(surface_index, bounds)| PrimitiveBounds {
                surface_index,
                bounds,
            })
            .collect();
        let parallel_axis = primitives.first().and_then(|first| {
            let bounds = first.bounds;
            let axis = if bounds.min_x == bounds.max_x {
                Axis::X
            } else if bounds.min_y == bounds.max_y {
                Axis::Y
            } else if matches!(dimensions, SceneDimensions::Three) && bounds.min_z == bounds.max_z {
                Axis::Z
            } else {
                return None;
            };
            primitives
                .iter()
                .all(|primitive| primitive.bounds.extent(axis) == 0.0)
                .then_some(axis)
        });
        let mut hierarchy = Self {
            parallel_axis,
            ..Self::default()
        };
        if primitives.is_empty() {
            return hierarchy;
        }
        hierarchy.large_surfaces = Self::partition_large_primitives(&mut primitives, dimensions);
        let root = Self::build_node(
            &mut primitives,
            dimensions,
            &mut hierarchy.nodes,
            &mut hierarchy.surface_indices,
        );
        hierarchy.root = Some(root);
        let grid = Self::props_grid(&hierarchy.nodes, root, &primitives, dimensions);
        hierarchy.small_grid = grid;
        hierarchy
    }

    /// Returns the bounds of every primitive in the hierarchy, or `None` for an empty scene.
    pub(crate) fn root_bounds(&self) -> Option<Bounds> {
        let mut bounds = self
            .root
            .and_then(|index| self.nodes.get(index))
            .map(|node| node.bounds);
        for surface in &self.large_surfaces {
            bounds = Some(bounds.map_or(surface.bounds, |existing| existing.union(surface.bounds)));
        }
        bounds
    }

    /// Rejects parallel wall groups whose tangential bounds cannot contain an image reflection.
    /// A valid image ray is a convex combination of the two tangential endpoint coordinates.
    /// Other orientations keep the exact per-surface path; padding permits arithmetic rounding.
    pub(crate) fn may_have_reflections(&self, path: Bounds) -> bool {
        if self.large_surfaces_may_reflect(path) {
            return true;
        }
        let Some(root) = self.root.and_then(|index| self.nodes.get(index)) else {
            return false;
        };
        // A few exact candidates cost less than a separate projected-bounds query.
        if self.surface_indices.len() <= MAX_LEAF_SURFACES {
            return true;
        }
        let Some(axis) = self.parallel_axis else {
            return true;
        };
        tangential_overlap(axis, path, root.bounds)
    }

    /// Visits candidates until the callback returns false and reports whether traversal completed.
    pub(crate) fn visit_candidates_until(
        &self,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        mut visit_surface: impl FnMut(usize) -> bool,
    ) -> bool {
        if !self.visit_large_candidates(path_bounds, skipped_surface, &mut visit_surface) {
            return false;
        }
        let Some(root) = self.root else {
            return true;
        };
        self.visit_candidate_node(root, path_bounds, skipped_surface, &mut visit_surface)
    }

    /// Visits an open path's conservative candidates, pruning by the line rather than its box.
    /// Small leaves use the cheaper box test; larger trees reject off-line subtrees with slabs.
    pub(crate) fn visit_segment_candidates(
        &self,
        ray: Ray,
        bounds: Bounds,
        skipped_surface: Option<usize>,
        mut visit_surface: impl FnMut(usize) -> bool,
    ) -> bool {
        if self.surface_indices.len() <= MAX_LEAF_SURFACES {
            return self.visit_candidates_until(bounds, skipped_surface, visit_surface);
        }
        let mut completed = true;
        self.visit_ray(ray, 1.0, |index| {
            if skipped_surface != Some(index) && !visit_surface(index) {
                completed = false;
                Some(-1.0)
            } else {
                None
            }
        });
        completed
    }

    /// Visits surfaces whose node bounds a ray reaches before the callback's current closest hit.
    ///
    /// The callback returns the ray parameter of an exact hit, which then prunes farther nodes.
    pub(crate) fn visit_ray(
        &self,
        ray: Ray,
        mut maximum_parameter: f64,
        mut hit_parameter: impl FnMut(usize) -> Option<f64>,
    ) {
        self.visit_large_rays(ray, &mut maximum_parameter, &mut hit_parameter);
        if maximum_parameter < 0.0 {
            return;
        }
        let Some(root) = self.root else {
            return;
        };
        let root_is_reached = self
            .nodes
            .get(root)
            .and_then(|node| node.bounds.ray_entry(ray, maximum_parameter))
            .is_some();
        if root_is_reached {
            self.visit_ray_node(root, ray, &mut maximum_parameter, &mut hit_parameter);
        }
    }

    /// Returns whether `test` accepts any primitive the ray corridor reaches before the limit.
    ///
    /// This is the visibility-query specialization of [`Self::visit_ray`]: the answer is a
    /// boolean, so children are visited without nearest-first ordering and traversal stops at
    /// the first accepted primitive. Visibility checks are the hottest traversal in a listener
    /// trace because every bounce casts one per source plus one toward the listener.
    /// Room-scale primitives are tested first. A short overlap of the remaining props is
    /// answered from a uniform grid; a long overlap keeps the binary tree. `path_bounds` is
    /// the corridor's conservative box, used only for tiny single-leaf scenes where the box
    /// test beats slab traversal setup.
    pub(crate) fn any_hit(
        &self,
        ray: Ray,
        path_bounds: Bounds,
        limit: f64,
        skipped_surface: Option<usize>,
        mut test: impl FnMut(usize) -> bool,
    ) -> bool {
        if self.surface_indices.len() <= MAX_LEAF_SURFACES {
            return !self
                .visit_candidates_until(path_bounds, skipped_surface, |index| !test(index));
        }
        // Room-scale primitives sit outside the root, so a root miss is not a scene miss.
        if self.large_ray_accepted(ray, path_bounds, limit, skipped_surface, &mut test) {
            return true;
        }
        if let Some(grid) = &self.small_grid {
            match grid.query(ray, limit, skipped_surface, &mut test) {
                GridAnswer::Hit => return true,
                GridAnswer::Clear => return false,
                GridAnswer::Wide => {}
            }
        }
        let Some(root) = self.root else {
            return false;
        };
        let root_is_reached = self
            .nodes
            .get(root)
            .is_some_and(|node| node.bounds.ray_entry(ray, limit).is_some());
        if !root_is_reached {
            return false;
        }
        self.any_hit_stacked::<MAX_TRAVERSAL_STACK>(root, ray, limit, skipped_surface, &mut test)
    }

    /// Walks a reached root with a fixed frontier of `STACK` nodes.
    ///
    /// `STACK` is at least one. When both children of a node are already waiting, the extra
    /// child continues through [`Self::any_hit_descendant`], which keeps the answer exact for a
    /// frontier smaller than the tree. [`MAX_TRAVERSAL_STACK`] is larger than any tree that fits
    /// in memory, so production calls do not use that continuation.
    fn any_hit_stacked<const STACK: usize>(
        &self,
        root: usize,
        ray: Ray,
        limit: f64,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let mut stack = [root; STACK];
        let mut depth = 1_usize;
        while depth > 0 {
            depth -= 1;
            let Some(node_index) = stack.get(depth).copied() else {
                break;
            };
            let Some(node) = self.nodes.get(node_index) else {
                continue;
            };
            match node.kind {
                NodeKind::Leaf { ref indices } => {
                    if self.leaf_hits(indices.clone(), skipped_surface, test) {
                        return true;
                    }
                }
                NodeKind::Branch { left, right } => {
                    for child in [left, right] {
                        let reached = self
                            .nodes
                            .get(child)
                            .is_some_and(|node| node.bounds.ray_entry(ray, limit).is_some());
                        if !reached {
                            continue;
                        }
                        match stack.get_mut(depth) {
                            Some(slot) => {
                                *slot = child;
                                depth += 1;
                            }
                            // A full frontier keeps the child exact without discarding it.
                            None => {
                                if self.any_hit_descendant(
                                    child,
                                    ray,
                                    limit,
                                    skipped_surface,
                                    &mut *test,
                                ) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Recursively answers one [`Self::any_hit`] query for a subtree without stack storage.
    fn any_hit_descendant(
        &self,
        node_index: usize,
        ray: Ray,
        limit: f64,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(node) = self.nodes.get(node_index) else {
            return false;
        };
        match node.kind {
            NodeKind::Leaf { ref indices } => {
                self.leaf_hits(indices.clone(), skipped_surface, test)
            }
            NodeKind::Branch { left, right } => {
                for child in [left, right] {
                    let reached = self
                        .nodes
                        .get(child)
                        .is_some_and(|node| node.bounds.ray_entry(ray, limit).is_some());
                    if reached && self.any_hit_descendant(child, ray, limit, skipped_surface, test)
                    {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Returns whether one reached leaf contains an accepted surface that passes `test`.
    fn leaf_hits(
        &self,
        indices: Range<usize>,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(surface_indices) = self.surface_indices.get(indices) else {
            return false;
        };
        for surface_index in surface_indices {
            if skipped_surface != Some(*surface_index) && test(*surface_index) {
                return true;
            }
        }
        false
    }

    /// Visits a reached leaf or descends into children nearest-first.
    ///
    /// Each child's slab entry is computed once here; a child is skipped when an earlier hit has
    /// moved the limit in front of its entry, which matches re-running the full slab test.
    fn visit_ray_node(
        &self,
        node_index: usize,
        ray: Ray,
        maximum_parameter: &mut f64,
        hit_parameter: &mut impl FnMut(usize) -> Option<f64>,
    ) {
        // A negative callback distance is the visibility-query termination sentinel.
        // No forward ray can reach another node, including one containing its origin.
        if *maximum_parameter < 0.0 {
            return;
        }
        let Some(node) = self.nodes.get(node_index) else {
            return;
        };
        let mut record = |surface_index: usize, maximum_parameter: &mut f64| {
            if let Some(parameter) = hit_parameter(surface_index)
                && parameter < *maximum_parameter
            {
                *maximum_parameter = parameter;
            }
        };

        match &node.kind {
            NodeKind::Leaf { indices } => {
                if let Some(surface_indices) = self.surface_indices.get(indices.clone()) {
                    for surface_index in surface_indices {
                        record(*surface_index, maximum_parameter);
                        if *maximum_parameter < 0.0 {
                            return;
                        }
                    }
                }
            }
            NodeKind::Branch { left, right } => {
                let left_entry = self
                    .nodes
                    .get(*left)
                    .and_then(|child| child.bounds.ray_entry(ray, *maximum_parameter));
                let right_entry = self
                    .nodes
                    .get(*right)
                    .and_then(|child| child.bounds.ray_entry(ray, *maximum_parameter));
                // Visiting the nearer child first lets its hits prune the farther child.
                let children = match (left_entry, right_entry) {
                    (Some(left_parameter), Some(right_parameter))
                        if right_parameter < left_parameter =>
                    {
                        [(*right, right_entry), (*left, left_entry)]
                    }
                    _ => [(*left, left_entry), (*right, right_entry)],
                };
                for (child, entry) in children {
                    if entry.is_some_and(|parameter| parameter <= *maximum_parameter) {
                        self.visit_ray_node(child, ray, maximum_parameter, hit_parameter);
                    }
                }
            }
        }
    }

    /// Moves room-scale outliers out of `primitives` when a tight majority remains.
    ///
    /// A leaf-sized set, a nonpositive median extent, and a split that would move every
    /// primitive stay in input order. The retained slice keeps that order, so later median
    /// splits of a uniform room match a tree built without this partition.
    fn partition_large_primitives(
        primitives: &mut Vec<PrimitiveBounds>,
        dimensions: SceneDimensions,
    ) -> Vec<PrimitiveBounds> {
        if primitives.len() <= MAX_LEAF_SURFACES {
            return Vec::new();
        }
        let mut extents = primitives
            .iter()
            .map(|primitive| primitive.bounds.longest_extent(dimensions))
            .collect::<Vec<_>>();
        let median_index = extents.len() / 2;
        extents.select_nth_unstable_by(median_index, f64::total_cmp);
        let median = extents.get(median_index).copied().unwrap_or(0.0);
        if median <= 0.0 {
            return Vec::new();
        }
        let limit = median * LARGE_PRIMITIVE_EXTENT_FACTOR;
        let large_count = extents.iter().filter(|extent| **extent > limit).count();
        if large_count == 0 || large_count == primitives.len() {
            return Vec::new();
        }
        let mut large_surfaces = Vec::with_capacity(large_count);
        primitives.retain(|primitive| {
            let stays = primitive.bounds.longest_extent(dimensions) <= limit;
            if !stays {
                large_surfaces.push(*primitive);
            }
            stays
        });
        large_surfaces
    }

    /// Returns whether a side-listed primitive can still reflect `path`.
    ///
    /// An empty side list leaves the tree rule unchanged. Without one shared plane, the
    /// tangential test cannot reject a side-listed primitive.
    fn large_surfaces_may_reflect(&self, path: Bounds) -> bool {
        if self.large_surfaces.is_empty() {
            return false;
        }
        let Some(axis) = self.parallel_axis else {
            return true;
        };
        self.large_surfaces
            .iter()
            .any(|surface| tangential_overlap(axis, path, surface.bounds))
    }

    /// Visits side-listed primitives whose bounds overlap `path_bounds`.
    ///
    /// Returns whether every overlapping primitive was accepted.
    fn visit_large_candidates(
        &self,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        visit_surface: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        for surface in &self.large_surfaces {
            let skipped = skipped_surface == Some(surface.surface_index);
            if !skipped
                && surface.bounds.overlaps(path_bounds)
                && !visit_surface(surface.surface_index)
            {
                return false;
            }
        }
        true
    }

    /// Tests side-listed primitives and tightens the closest-hit limit.
    fn visit_large_rays(
        &self,
        ray: Ray,
        maximum_parameter: &mut f64,
        hit_parameter: &mut impl FnMut(usize) -> Option<f64>,
    ) {
        for surface in &self.large_surfaces {
            if *maximum_parameter < 0.0 {
                return;
            }
            if surface.bounds.ray_entry(ray, *maximum_parameter).is_some()
                && let Some(parameter) = hit_parameter(surface.surface_index)
                && parameter < *maximum_parameter
            {
                *maximum_parameter = parameter;
            }
        }
    }

    /// Returns whether `test` accepts a side-listed primitive the ray reaches before `limit`.
    ///
    /// `path_bounds` boxes the ray's segment. Six comparisons reject a room surface the segment
    /// cannot reach before the slab test pays three multiplies per axis, and most visibility
    /// queries in a room stay far from every wall.
    fn large_ray_accepted(
        &self,
        ray: Ray,
        path_bounds: Bounds,
        limit: f64,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        self.large_surfaces.iter().any(|surface| {
            skipped_surface != Some(surface.surface_index)
                && surface.bounds.overlaps(path_bounds)
                && surface.bounds.ray_entry(ray, limit).is_some()
                && test(surface.surface_index)
        })
    }

    /// Returns how many primitives were stored beside the tree.
    #[cfg(test)]
    const fn large_surface_count(&self) -> usize {
        self.large_surfaces.len()
    }

    /// Indexes the small primitives with the same bounds the tree root stored.
    fn props_grid(
        nodes: &[Node],
        root: usize,
        primitives: &[PrimitiveBounds],
        dimensions: SceneDimensions,
    ) -> Option<SmallGrid> {
        let bounds = nodes.get(root)?.bounds;
        SmallGrid::build(primitives, dimensions, bounds)
    }

    /// Partitions one nonempty slice and appends its node after all child nodes.
    fn build_node(
        primitives: &mut [PrimitiveBounds],
        dimensions: SceneDimensions,
        nodes: &mut Vec<Node>,
        surface_indices: &mut Vec<usize>,
    ) -> usize {
        let first = primitives
            .first()
            .copied()
            .expect("only nonempty BVH partitions are built");
        let bounds = primitives
            .iter()
            .skip(1)
            .fold(first.bounds, |combined, primitive| {
                combined.union(primitive.bounds)
            });

        if primitives.len() <= MAX_LEAF_SURFACES {
            // Leaf order does not affect scene output; the original indices remain attached.
            let start = surface_indices.len();
            surface_indices.extend(primitives.iter().map(|primitive| primitive.surface_index));
            let end = surface_indices.len();
            let node_index = nodes.len();
            nodes.push(Node {
                bounds,
                kind: NodeKind::Leaf {
                    indices: start..end,
                },
            });
            return node_index;
        }

        // Split by centroid spread, not surface extent: tall/long walls must not force
        // every interior partition onto an axis where all primitive centers coincide.
        let centers = primitives
            .iter()
            .map(|primitive| {
                let center = (
                    primitive.bounds.center(Axis::X),
                    primitive.bounds.center(Axis::Y),
                    primitive.bounds.center(Axis::Z),
                );
                Bounds::path_3d(center, center)
            })
            .reduce(Bounds::union)
            .expect("nonempty node");
        let axis = centers.widest_axis(dimensions);
        let midpoint = primitives.len() / 2;
        primitives.select_nth_unstable_by(midpoint, |left, right| {
            left.bounds
                .center(axis)
                .total_cmp(&right.bounds.center(axis))
                .then_with(|| left.surface_index.cmp(&right.surface_index))
        });
        let (left_primitives, right_primitives) = primitives.split_at_mut(midpoint);
        let left_node = Self::build_node(left_primitives, dimensions, nodes, surface_indices);
        let right_node = Self::build_node(right_primitives, dimensions, nodes, surface_indices);
        let node_index = nodes.len();
        nodes.push(Node {
            bounds,
            kind: NodeKind::Branch {
                left: left_node,
                right: right_node,
            },
        });
        node_index
    }

    /// Visits overlapping branch and leaf candidates without allocating traversal storage.
    fn visit_candidate_node(
        &self,
        node_index: usize,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        visit_surface: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(node) = self.nodes.get(node_index) else {
            return true;
        };
        if !node.bounds.overlaps(path_bounds) {
            return true;
        }

        match &node.kind {
            NodeKind::Leaf { indices } => {
                if let Some(surface_indices) = self.surface_indices.get(indices.clone()) {
                    for surface_index in surface_indices {
                        if skipped_surface != Some(*surface_index) && !visit_surface(*surface_index)
                        {
                            return false;
                        }
                    }
                }
                true
            }
            NodeKind::Branch { left, right } => {
                if !self.visit_candidate_node(*left, path_bounds, skipped_surface, visit_surface) {
                    return false;
                }
                self.visit_candidate_node(*right, path_bounds, skipped_surface, visit_surface)
            }
        }
    }
}

/// Tightens one slab overlap. Returns false when the ray misses this axis.
///
/// Plain comparisons avoid the NaN and signed-zero handling of `f64::min` and `f64::max`.
/// Every operand on the hot path is finite, and this runs for each visited node.
#[inline]
fn tighten_slab(
    entry: &mut f64,
    exit: &mut f64,
    origin: f64,
    inverse_direction: f64,
    minimum: f64,
    maximum: f64,
) -> bool {
    if inverse_direction.is_infinite() {
        // A ray parallel to this slab must already lie between its planes.
        return !(origin < minimum || origin > maximum);
    }
    let first = (minimum - origin) * inverse_direction;
    let second = (maximum - origin) * inverse_direction;
    let (near, far) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    if near > *entry {
        *entry = near;
    }
    if far < *exit {
        *exit = far;
    }
    *entry <= *exit
}

/// Uniform x-z lattice over the small primitives of one 3D hierarchy.
#[derive(Clone, Debug)]
struct SmallGrid {
    /// Clip box. This is the small-primitive root, so a miss is a root miss.
    bounds: Bounds,
    /// Cell edge length, in meters.
    cell: f64,
    /// Cell columns along x.
    x_count: usize,
    /// Cell rows along z.
    z_count: usize,
    /// Exclusive prefix: cell `c` occupies `starts[c]..starts[c + 1]`.
    starts: Vec<u32>,
    /// Surface indices grouped by cell. One surface may occupy several cells.
    indices: Vec<u32>,
}

/// Result of a uniform-grid visibility query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridAnswer {
    /// The ray misses the prop box, or a short scan accepted nothing.
    Clear,
    /// A short scan accepted a surface.
    Hit,
    /// The overlap is wider than a direct scan, so the binary tree must answer.
    Wide,
}

/// Cell size and dimensions of one lattice.
#[derive(Clone, Copy, Debug)]
struct Lattice {
    /// Cell edge length, in meters.
    cell: f64,
    /// Cell columns along x.
    columns: usize,
    /// Cell rows along z.
    rows: usize,
}

impl SmallGrid {
    /// Builds a lattice for a 3D prop set large enough that short scans beat the tree.
    fn build(
        primitives: &[PrimitiveBounds],
        dimensions: SceneDimensions,
        bounds: Bounds,
    ) -> Option<Self> {
        if !matches!(dimensions, SceneDimensions::Three) || primitives.len() < MIN_GRID_SURFACES {
            return None;
        }
        let lattice = Lattice::from_bounds(bounds, primitives.len())?;
        let starts = cell_starts(primitives, bounds, lattice)?;
        let indices = scatter_cells(primitives, bounds, lattice, &starts)?;
        Some(Self {
            bounds,
            cell: lattice.cell,
            x_count: lattice.columns,
            z_count: lattice.rows,
            starts,
            indices,
        })
    }

    /// Answers a short visibility query. A wide footprint returns [`GridAnswer::Wide`].
    #[inline]
    fn query(
        &self,
        ray: Ray,
        limit: f64,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> GridAnswer {
        let Some((entry, exit)) = self.bounds.ray_overlap(ray, limit) else {
            return GridAnswer::Clear;
        };
        let Some((x0, x1, z0, z1)) = self.footprint(ray, entry, exit) else {
            return GridAnswer::Wide;
        };
        let width = x0.abs_diff(x1).saturating_add(1);
        let depth = z0.abs_diff(z1).saturating_add(1);
        let Some(covered) = width.checked_mul(depth) else {
            return GridAnswer::Wide;
        };
        if covered > MAX_DIRECT_GRID_CELLS {
            return GridAnswer::Wide;
        }
        if self.rectangle_hits(x0, x1, z0, z1, skipped_surface, test) {
            GridAnswer::Hit
        } else {
            GridAnswer::Clear
        }
    }

    /// Maps the clipped segment's x-z endpoints into lattice slots.
    #[inline]
    fn footprint(&self, ray: Ray, entry: f64, exit: f64) -> Option<(usize, usize, usize, usize)> {
        let start_x = axis_position(ray.origin.0, ray.inverse_direction.0, entry)?;
        let end_x = axis_position(ray.origin.0, ray.inverse_direction.0, exit)?;
        let start_z = axis_position(ray.origin.2, ray.inverse_direction.2, entry)?;
        let end_z = axis_position(ray.origin.2, ray.inverse_direction.2, exit)?;
        let x0 = lattice_index(start_x, self.bounds.min_x, self.cell, self.x_count)?;
        let x1 = lattice_index(end_x, self.bounds.min_x, self.cell, self.x_count)?;
        let z0 = lattice_index(start_z, self.bounds.min_z, self.cell, self.z_count)?;
        let z1 = lattice_index(end_z, self.bounds.min_z, self.cell, self.z_count)?;
        Some((x0, x1, z0, z1))
    }

    /// Returns whether any surface in the inclusive cell rectangle passes `test`.
    #[inline]
    fn rectangle_hits(
        &self,
        x0: usize,
        x1: usize,
        z0: usize,
        z1: usize,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let x_low = x0.min(x1);
        let x_high = x0.max(x1);
        let z_low = z0.min(z1);
        let z_high = z0.max(z1);
        for column in x_low..=x_high {
            for row in z_low..=z_high {
                if self.cell_hits(column, row, skipped_surface, test) {
                    return true;
                }
            }
        }
        false
    }

    /// Returns whether one cell contains an accepted surface other than `skipped_surface`.
    #[inline]
    fn cell_hits(
        &self,
        column: usize,
        row: usize,
        skipped_surface: Option<usize>,
        test: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(slot) = column
            .checked_mul(self.z_count)
            .and_then(|value| value.checked_add(row))
        else {
            return false;
        };
        let Some(start) = self.starts.get(slot).copied() else {
            return false;
        };
        let Some(end) = self.starts.get(slot.saturating_add(1)).copied() else {
            return false;
        };
        let (Some(start_index), Some(end_index)) =
            (usize::try_from(start).ok(), usize::try_from(end).ok())
        else {
            return false;
        };
        let Some(surfaces) = self.indices.get(start_index..end_index) else {
            return false;
        };
        for surface in surfaces {
            let Some(surface_index) = usize::try_from(*surface).ok() else {
                continue;
            };
            if skipped_surface != Some(surface_index) && test(surface_index) {
                return true;
            }
        }
        false
    }
}

impl Lattice {
    /// Chooses a near-square lattice that covers `bounds` and stays under the cell cap.
    fn from_bounds(bounds: Bounds, count: usize) -> Option<Self> {
        let span_x = bounds.max_x - bounds.min_x;
        let span_z = bounds.max_z - bounds.min_z;
        if !span_x.is_finite() || !span_z.is_finite() || span_x < 0.0 || span_z < 0.0 {
            return None;
        }
        let count_f = f64::from(u32::try_from(count).ok()?);
        let span = span_x.max(span_z).max(1.0e-3);
        let side = count_f.sqrt().ceil().max(1.0);
        let cell = span / side;
        if !cell.is_finite() || cell <= 0.0 {
            return None;
        }
        let columns = axis_cells(span_x, cell)?;
        let rows = axis_cells(span_z, cell)?;
        let cells = columns.checked_mul(rows)?;
        if cells > MAX_GRID_CELLS {
            return None;
        }
        Some(Self {
            cell,
            columns,
            rows,
        })
    }
}

/// Builds the exclusive cell prefix from every primitive reference.
fn cell_starts(
    primitives: &[PrimitiveBounds],
    bounds: Bounds,
    lattice: Lattice,
) -> Option<Vec<u32>> {
    let cells = lattice.columns.checked_mul(lattice.rows)?;
    let mut counts = vec![0_u32; cells];
    for primitive in primitives {
        visit_cells(primitive.bounds, bounds, lattice, |slot| {
            let count = counts.get_mut(slot)?;
            *count = count.checked_add(1)?;
            Some(())
        })?;
    }
    prefix_counts(&counts)
}

/// Turns per-cell counts into an exclusive prefix of length `counts + 1`.
fn prefix_counts(counts: &[u32]) -> Option<Vec<u32>> {
    let mut starts = Vec::with_capacity(counts.len().saturating_add(1));
    let mut running = 0_u32;
    for count in counts {
        starts.push(running);
        running = running.checked_add(*count)?;
    }
    starts.push(running);
    Some(starts)
}

/// Writes surface indices into the ranges described by `starts`.
fn scatter_cells(
    primitives: &[PrimitiveBounds],
    bounds: Bounds,
    lattice: Lattice,
    starts: &[u32],
) -> Option<Vec<u32>> {
    let total = starts.last().copied()?;
    let mut cursor = starts.to_vec();
    let mut indices = vec![0_u32; usize::try_from(total).ok()?];
    for primitive in primitives {
        let index = u32::try_from(primitive.surface_index).ok()?;
        visit_cells(primitive.bounds, bounds, lattice, |slot| {
            let place = cursor.get_mut(slot)?;
            let position = usize::try_from(*place).ok()?;
            *indices.get_mut(position)? = index;
            *place = place.checked_add(1)?;
            Some(())
        })?;
    }
    Some(indices)
}

/// Visits every lattice slot overlapped by `primitive`.
fn visit_cells(
    primitive: Bounds,
    root: Bounds,
    lattice: Lattice,
    mut visit: impl FnMut(usize) -> Option<()>,
) -> Option<()> {
    let x0 = lattice_index(primitive.min_x, root.min_x, lattice.cell, lattice.columns)?;
    let x1 = lattice_index(primitive.max_x, root.min_x, lattice.cell, lattice.columns)?;
    let z0 = lattice_index(primitive.min_z, root.min_z, lattice.cell, lattice.rows)?;
    let z1 = lattice_index(primitive.max_z, root.min_z, lattice.cell, lattice.rows)?;
    for column in x0.min(x1)..=x0.max(x1) {
        for row in z0.min(z1)..=z0.max(z1) {
            let slot = column.checked_mul(lattice.rows)?.checked_add(row)?;
            visit(slot)?;
        }
    }
    Some(())
}

/// Column or row count that keeps the far boundary inside the lattice.
fn axis_cells(span: f64, cell: f64) -> Option<usize> {
    let scaled = (span / cell).ceil().max(1.0);
    if !scaled.is_finite() {
        return None;
    }
    lattice_slot(scaled).checked_add(1)
}

/// Clamps a world coordinate into a lattice axis.
fn lattice_index(position: f64, origin: f64, cell: f64, count: usize) -> Option<usize> {
    if count == 0 || !position.is_finite() || !cell.is_finite() || cell <= 0.0 {
        return None;
    }
    let scaled = ((position - origin) / cell).floor();
    if !scaled.is_finite() {
        return None;
    }
    if scaled <= 0.0 {
        return Some(0);
    }
    let last = count - 1;
    let last_u32 = u32::try_from(last).ok()?;
    if scaled >= f64::from(last_u32) {
        return Some(last);
    }
    Some(lattice_slot(scaled))
}

/// Converts a finite, non-negative lattice coordinate that already lies below the axis length.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers reject non-finite and negative coordinates and clamp to the axis length first"
)]
const fn lattice_slot(scaled: f64) -> usize {
    scaled as usize
}

/// Reconstructs one world coordinate from a reciprocal direction and a ray parameter.
fn axis_position(origin: f64, inverse: f64, parameter: f64) -> Option<f64> {
    let position = if inverse.is_finite() {
        let scaled = parameter * inverse.recip();
        origin + scaled
    } else {
        origin
    };
    position.is_finite().then_some(position)
}

/// Returns whether `path` meets `bounds` on the axes perpendicular to `axis`.
///
/// Padding covers the same rounding the exact reflection test allows.
fn tangential_overlap(axis: Axis, path: Bounds, bounds: Bounds) -> bool {
    let overlaps = |minimum: f64, maximum: f64, low: f64, high: f64| {
        let padding = 64.0
            * f64::EPSILON
            * minimum
                .abs()
                .max(maximum.abs())
                .max(low.abs())
                .max(high.abs());
        minimum - padding <= high && low <= maximum + padding
    };
    match axis {
        Axis::X => {
            overlaps(path.min_y, path.max_y, bounds.min_y, bounds.max_y)
                && overlaps(path.min_z, path.max_z, bounds.min_z, bounds.max_z)
        }
        Axis::Y => {
            overlaps(path.min_x, path.max_x, bounds.min_x, bounds.max_x)
                && overlaps(path.min_z, path.max_z, bounds.min_z, bounds.max_z)
        }
        Axis::Z => {
            overlaps(path.min_x, path.max_x, bounds.min_x, bounds.max_x)
                && overlaps(path.min_y, path.max_y, bounds.min_y, bounds.max_y)
        }
    }
}

/// A half-line with a precomputed reciprocal direction for slab tests.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ray {
    /// Ray origin in scene coordinates; 2D scenes use zero depth.
    origin: (f64, f64, f64),
    /// Componentwise reciprocal of the ray direction; zero components become infinite.
    inverse_direction: (f64, f64, f64),
}

impl Ray {
    /// Builds a ray from its origin and nonzero direction.
    pub(crate) const fn new(origin: (f64, f64, f64), direction: (f64, f64, f64)) -> Self {
        Self {
            origin,
            inverse_direction: (
                direction.0.recip(),
                direction.1.recip(),
                direction.2.recip(),
            ),
        }
    }
}

/// Original surface identifier and its conservative padded bounds.
#[derive(Clone, Copy, Debug)]
struct PrimitiveBounds {
    /// Insertion-order index used by exact tests and reflection output.
    surface_index: usize,
    /// Bounds including the exact geometry predicate's tolerance.
    bounds: Bounds,
}

/// One branch or leaf in the balanced scene tree.
#[derive(Clone, Debug)]
struct Node {
    /// Combined bounds for this node and all descendants.
    bounds: Bounds,
    /// Closed node shape, with primitives stored only in leaves.
    kind: NodeKind,
}

/// Legal tree node shapes stored in the acceleration structure.
#[derive(Clone, Debug)]
enum NodeKind {
    /// A small contiguous range of original surface indices.
    Leaf {
        /// Range in `BoundingVolumeHierarchy::surface_indices`.
        indices: Range<usize>,
    },
    /// Two recursively balanced children; exact tests occur only in reached leaves.
    Branch {
        /// Left child node index.
        left: usize,
        /// Right child node index.
        right: usize,
    },
}

#[cfg(test)]
#[path = "../tests/unit/bvh.rs"]
mod tests;
