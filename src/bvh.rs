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

    /// Excludes the reflector itself before invoking the exact intersection predicate.
    fn surface_intersects(
        surface_index: usize,
        skipped_surface: Option<usize>,
        intersects_surface: &mut impl FnMut(usize) -> bool,
    ) -> bool {
        skipped_surface != Some(surface_index) && intersects_surface(surface_index)
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
