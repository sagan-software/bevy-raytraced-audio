//! Cached balanced bounds hierarchy used for exact segment and triangle visibility tests.

use core::ops::Range;

/// Maximum primitive count stored directly in one leaf.
const MAX_LEAF_SURFACES: usize = 4;

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

    /// Returns whether two closed bounds overlap on all three stored axes.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && other.min_x <= self.max_x
            && self.min_y <= other.max_y
            && other.min_y <= self.max_y
            && self.min_z <= other.max_z
            && other.min_z <= self.max_z
    }

    /// Returns the midpoint coordinate on one split axis without overflowing on finite inputs.
    fn center(self, axis: Axis) -> f64 {
        let (minimum, maximum) = match axis {
            Axis::X => (self.min_x, self.max_x),
            Axis::Y => (self.min_y, self.max_y),
            Axis::Z => (self.min_z, self.max_z),
        };
        maximum.mul_add(0.5, minimum * 0.5)
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
}

/// Bounds and primitive indices arranged into a balanced binary tree.
#[derive(Clone, Debug, Default)]
pub(crate) struct BoundingVolumeHierarchy {
    /// Tree nodes stored in post-order so child bounds are already available during construction.
    nodes: Vec<Node>,
    /// Primitive indices stored contiguously for leaf ranges.
    surface_indices: Vec<usize>,
    /// Root node index, or `None` for an empty scene.
    root: Option<usize>,
}

impl BoundingVolumeHierarchy {
    /// Builds a balanced tree while preserving original surface indices in every node.
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
        let mut hierarchy = Self::default();
        let root = Self::build_node(
            &mut primitives,
            dimensions,
            &mut hierarchy.nodes,
            &mut hierarchy.surface_indices,
        );
        hierarchy.root = root;
        hierarchy
    }

    /// Checks overlapping leaves with the exact geometry predicate and an optional skipped surface.
    pub(crate) fn any_intersection(
        &self,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        mut intersects_surface: impl FnMut(usize) -> bool,
    ) -> bool {
        self.root.is_some_and(|root| {
            self.intersects_node(root, path_bounds, skipped_surface, &mut intersects_surface)
        })
    }

    /// Visits candidates until the callback returns false and reports whether traversal completed.
    pub(crate) fn visit_candidates_until(
        &self,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        mut visit_surface: impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(root) = self.root else {
            return true;
        };
        self.visit_candidate_node(root, path_bounds, skipped_surface, &mut visit_surface)
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

    /// Visits one node the ray already reaches: its stored primitives, then children nearest-first.
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
                    }
                }
            }
            NodeKind::Branch {
                surface_index,
                left,
                right,
            } => {
                record(*surface_index, maximum_parameter);
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

    /// Partitions one nonempty slice and appends its node after all child nodes.
    fn build_node(
        primitives: &mut [PrimitiveBounds],
        dimensions: SceneDimensions,
        nodes: &mut Vec<Node>,
        surface_indices: &mut Vec<usize>,
    ) -> Option<usize> {
        let first = primitives.first().copied()?;
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
            return Some(node_index);
        }

        // A total comparator makes equal-centroid partitioning deterministic across queries.
        let axis = bounds.widest_axis(dimensions);
        let midpoint = primitives.len() / 2;
        let (_, median, _) = primitives.select_nth_unstable_by(midpoint, |left, right| {
            left.bounds
                .center(axis)
                .total_cmp(&right.bounds.center(axis))
                .then_with(|| left.surface_index.cmp(&right.surface_index))
        });
        let surface_index = median.surface_index;
        let (left_primitives, right_primitives) = primitives.split_at_mut(midpoint);
        let (_, right_primitives) = right_primitives.split_first_mut()?;
        let left_node = Self::build_node(left_primitives, dimensions, nodes, surface_indices)?;
        let right_node = Self::build_node(right_primitives, dimensions, nodes, surface_indices)?;
        let node_index = nodes.len();
        nodes.push(Node {
            bounds,
            kind: NodeKind::Branch {
                surface_index,
                left: left_node,
                right: right_node,
            },
        });
        Some(node_index)
    }

    /// Tests one node and visits only children whose bounds overlap the path.
    fn intersects_node(
        &self,
        node_index: usize,
        path_bounds: Bounds,
        skipped_surface: Option<usize>,
        intersects_surface: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        let Some(node) = self.nodes.get(node_index) else {
            return false;
        };
        if !node.bounds.overlaps(path_bounds) {
            return false;
        }

        match &node.kind {
            NodeKind::Leaf { indices } => {
                self.surface_indices
                    .get(indices.clone())
                    .is_some_and(|surface_indices| {
                        surface_indices.iter().copied().any(|surface_index| {
                            Self::surface_intersects(
                                surface_index,
                                skipped_surface,
                                intersects_surface,
                            )
                        })
                    })
            }
            NodeKind::Branch {
                surface_index,
                left,
                right,
            } => {
                Self::surface_intersects(*surface_index, skipped_surface, intersects_surface)
                    || self.intersects_node(*left, path_bounds, skipped_surface, intersects_surface)
                    || self.intersects_node(
                        *right,
                        path_bounds,
                        skipped_surface,
                        intersects_surface,
                    )
            }
        }
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
            NodeKind::Branch {
                surface_index,
                left,
                right,
            } => {
                if skipped_surface != Some(*surface_index) && !visit_surface(*surface_index) {
                    return false;
                }
                if !self.visit_candidate_node(*left, path_bounds, skipped_surface, visit_surface) {
                    return false;
                }
                self.visit_candidate_node(*right, path_bounds, skipped_surface, visit_surface)
            }
        }
    }

    /// Excludes the reflector itself before invoking the exact intersection predicate.
    fn surface_intersects(
        surface_index: usize,
        skipped_surface: Option<usize>,
        intersects_surface: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        skipped_surface != Some(surface_index) && intersects_surface(surface_index)
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
    /// Closed node shape, with one primitive stored at each branch median.
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
    /// A median surface and its two recursively balanced children.
    Branch {
        /// Original surface index stored at the median.
        surface_index: usize,
        /// Left child node index.
        left: usize,
        /// Right child node index.
        right: usize,
    },
}

#[cfg(test)]
#[path = "../tests/unit/bvh.rs"]
mod tests;
