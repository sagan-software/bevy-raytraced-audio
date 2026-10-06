//! CPU acoustic propagation over explicit triangular surfaces in 3D.

use crate::math3d::Vector3;
use crate::{
    AcousticResponse, BandEnergy, BandGain, Emitter3d, Listener3d, PathResponse, Point3,
    ReflectionPath3d, ReflectionSurfaceIndex, SolverPoint3d, Triangle3d,
};

/// Tolerance for unitless segment and triangle coordinates.
const PARAMETER_EPSILON: f64 = 1.0e-9;

/// Relative tolerance used to classify a ray as parallel to a triangle.
const PARALLEL_EPSILON: f64 = 1.0e-12;

/// A three-dimensional acoustic scene using meter coordinates.
#[derive(Clone, Debug, Default)]
pub struct AcousticScene3d {
    /// Explicitly registered opaque and reflective triangles.
    triangles: Vec<Triangle3d>,
}

impl AcousticScene3d {
    /// Adds one validated surface triangle to the scene.
    pub fn add_triangle(&mut self, triangle: Triangle3d) {
        self.triangles.push(triangle);
    }

    /// Removes every surface triangle from the scene.
    pub fn clear(&mut self) {
        self.triangles.clear();
    }

    /// Returns the number of registered surface triangles.
    #[must_use]
    pub const fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Traces direct visibility and first-order reflections between two points.
    #[must_use]
    pub fn trace(&self, emitter: Emitter3d, listener: Listener3d) -> AcousticResponse {
        let source = Vector3::from_point(emitter.position());
        let receiver = Vector3::from_point(listener.position());
        let direct_distance = receiver.subtract(source).length();

        // If source and listener coincide, the path has no interior where an opaque surface can block it.
        let direct_occluded = direct_distance > 0.0
            && self
                .triangles
                .iter()
                .any(|triangle| path_intersects_triangle(source, receiver, triangle.vertices()));
        let direct_gain = if direct_occluded {
            BandGain::ZERO
        } else {
            BandGain::UNITY
        };
        let direct = PathResponse::new(direct_distance, direct_gain, direct_occluded);

        // Fused accumulation preserves the existing aggregate result for every visible path.
        let mut reflected_low = 0.0;
        let mut reflected_mid = 0.0;
        let mut reflected_high = 0.0;
        for path in self.reflection_paths_between(source, receiver, direct_distance) {
            let (distance_ratio_squared, absorption) = path.accumulation_terms();
            // Fused accumulation adds each reflected path with one rounding per band.
            reflected_low =
                distance_ratio_squared.mul_add(1.0 - f64::from(absorption.low()), reflected_low);
            reflected_mid =
                distance_ratio_squared.mul_add(1.0 - f64::from(absorption.mid()), reflected_mid);
            reflected_high =
                distance_ratio_squared.mul_add(1.0 - f64::from(absorption.high()), reflected_high);
        }

        AcousticResponse::new(
            direct,
            BandEnergy::from_solver(reflected_low, reflected_mid, reflected_high),
        )
    }

    /// Returns geometrically visible first-order reflections lazily in triangle insertion order; indices refer to this scene's current registration sequence.
    pub fn reflection_paths(
        &self,
        emitter: Emitter3d,
        listener: Listener3d,
    ) -> impl Iterator<Item = ReflectionPath3d> + '_ {
        let source = Vector3::from_point(emitter.position());
        let receiver = Vector3::from_point(listener.position());
        let direct_distance = receiver.subtract(source).length();
        self.reflection_paths_between(source, receiver, direct_distance)
    }

    /// Builds a lazy path iterator from the query's already-converted endpoints and distance.
    fn reflection_paths_between(
        &self,
        source: Vector3,
        receiver: Vector3,
        direct_distance: f64,
    ) -> impl Iterator<Item = ReflectionPath3d> + '_ {
        self.triangles
            .iter()
            .enumerate()
            .filter_map(move |(index, reflector)| {
                // If source and listener coincide, this triangle cannot form a positive-length reflected path.
                if direct_distance == 0.0 {
                    return None;
                }

                let (reflection_point, reflected_distance, image_source) =
                    first_reflection(source, receiver, reflector.vertices())?;

                // If another triangle crosses either leg, this reflected candidate is omitted.
                if self.segment_is_occluded(source, reflection_point, Some(index))
                    || self.segment_is_occluded(reflection_point, receiver, Some(index))
                {
                    return None;
                }

                let distance_ratio = direct_distance / reflected_distance;
                Some(ReflectionPath3d::new(
                    ReflectionSurfaceIndex::new(index),
                    SolverPoint3d::new(reflection_point.x, reflection_point.y, reflection_point.z),
                    SolverPoint3d::new(image_source.x, image_source.y, image_source.z),
                    reflected_distance,
                    distance_ratio * distance_ratio,
                    reflector.material().absorption(),
                ))
            })
    }

    /// Checks whether another registered triangle intersects an open path interior.
    fn segment_is_occluded(
        &self,
        start: Vector3,
        end: Vector3,
        skipped_index: Option<usize>,
    ) -> bool {
        self.triangles.iter().enumerate().any(|(index, triangle)| {
            if skipped_index == Some(index) {
                return false;
            }

            path_intersects_triangle(start, end, triangle.vertices())
        })
    }
}

/// Finds one valid image-source reflection and returns its point and path length.
fn first_reflection(
    source: Vector3,
    receiver: Vector3,
    vertices: [Point3; 3],
) -> Option<(Vector3, f64, Vector3)> {
    let [first, second, third] = vertices;
    let triangle_origin = Vector3::from_point(first);
    let edge_a = Vector3::from_point(second).subtract(triangle_origin);
    let edge_b = Vector3::from_point(third).subtract(triangle_origin);
    let normal = edge_a.cross(edge_b);
    let normal_squared = normal.dot(normal);
    let source_offset = source.subtract(triangle_origin).dot(normal);
    let receiver_offset = receiver.subtract(triangle_origin).dot(normal);

    // If either endpoint lies on the surface or they lie on opposite sides, no image-source reflection exists.
    if source_offset == 0.0
        || receiver_offset == 0.0
        || source_offset.is_sign_positive() != receiver_offset.is_sign_positive()
    {
        return None;
    }

    let image_source = source.subtract(normal.scale(2.0 * source_offset / normal_squared));
    let image_to_receiver = receiver.subtract(image_source);
    let denominator = image_to_receiver.dot(normal);
    let denominator_scale = image_to_receiver.length() * normal.length();
    if denominator.abs() <= PARALLEL_EPSILON * denominator_scale {
        return None;
    }

    // If the image ray misses the finite triangle, the candidate cannot reflect from this surface.
    let ray_parameter = triangle_origin.subtract(image_source).dot(normal) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter) {
        return None;
    }
    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    if !point_in_triangle(reflection_point, triangle_origin, edge_a, edge_b) {
        return None;
    }

    let reflected_distance = image_to_receiver.length();
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance, image_source))
}

/// Tests whether a point lies inside or on the boundary of one triangle.
fn point_in_triangle(point: Vector3, origin: Vector3, edge_a: Vector3, edge_b: Vector3) -> bool {
    let relative = point.subtract(origin);
    let first_edge_length_squared = edge_a.dot(edge_a);
    let edge_dot = edge_a.dot(edge_b);
    let second_edge_length_squared = edge_b.dot(edge_b);
    let relative_dot_a = relative.dot(edge_a);
    let relative_dot_b = relative.dot(edge_b);
    // This Gram determinant is zero exactly when the triangle basis is degenerate.
    let denominator = edge_dot.mul_add(
        -edge_dot,
        first_edge_length_squared * second_edge_length_squared,
    );
    if denominator == 0.0 {
        return false;
    }

    let coordinate_a = edge_dot
        .mul_add(-relative_dot_b, second_edge_length_squared * relative_dot_a)
        / denominator;
    let coordinate_b =
        edge_dot.mul_add(-relative_dot_a, first_edge_length_squared * relative_dot_b) / denominator;
    coordinate_a >= -PARAMETER_EPSILON
        && coordinate_b >= -PARAMETER_EPSILON
        && coordinate_a + coordinate_b <= 1.0 + PARAMETER_EPSILON
}

/// Tests an open path against one finite triangle using a double-precision ray test.
fn path_intersects_triangle(path_start: Vector3, path_end: Vector3, vertices: [Point3; 3]) -> bool {
    let [first, second, third] = vertices;
    let first = Vector3::from_point(first);
    let second = Vector3::from_point(second);
    let third = Vector3::from_point(third);
    let edge_a = second.subtract(first);
    let edge_b = third.subtract(first);
    let path_min_x = path_start.x.min(path_end.x);
    let path_max_x = path_start.x.max(path_end.x);
    let path_min_y = path_start.y.min(path_end.y);
    let path_max_y = path_start.y.max(path_end.y);
    let path_min_z = path_start.z.min(path_end.z);
    let path_max_z = path_start.z.max(path_end.z);
    let surface_padding_x = 2.0 * PARAMETER_EPSILON * (edge_a.x.abs() + edge_b.x.abs());
    let surface_padding_y = 2.0 * PARAMETER_EPSILON * (edge_a.y.abs() + edge_b.y.abs());
    let surface_padding_z = 2.0 * PARAMETER_EPSILON * (edge_a.z.abs() + edge_b.z.abs());
    let surface_min_x = first.x.min(second.x).min(third.x) - surface_padding_x;
    let surface_max_x = first.x.max(second.x).max(third.x) + surface_padding_x;
    let surface_min_y = first.y.min(second.y).min(third.y) - surface_padding_y;
    let surface_max_y = first.y.max(second.y).max(third.y) + surface_padding_y;
    let surface_min_z = first.z.min(second.z).min(third.z) - surface_padding_z;
    let surface_max_z = first.z.max(second.z).max(third.z) + surface_padding_z;

    // If the path and tolerant triangle bounds do not overlap, they cannot intersect.
    if path_max_x < surface_min_x
        || surface_max_x < path_min_x
        || path_max_y < surface_min_y
        || surface_max_y < path_min_y
        || path_max_z < surface_min_z
        || surface_max_z < path_min_z
    {
        return false;
    }

    let direction = path_end.subtract(path_start);
    let determinant_vector = direction.cross(edge_b);
    let determinant = edge_a.dot(determinant_vector);
    let determinant_scale = direction.length() * edge_a.length() * edge_b.length();
    if determinant.abs() <= PARALLEL_EPSILON * determinant_scale {
        // A path parallel to the plane does not cross the triangle surface.
        return false;
    }

    let inverse_determinant = 1.0 / determinant;
    let origin_delta = path_start.subtract(first);
    let coordinate_a = origin_delta.dot(determinant_vector) * inverse_determinant;
    if !(-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&coordinate_a) {
        return false;
    }

    let cross_vector = origin_delta.cross(edge_a);
    let coordinate_b = direction.dot(cross_vector) * inverse_determinant;
    if coordinate_b < -PARAMETER_EPSILON || coordinate_a + coordinate_b > 1.0 + PARAMETER_EPSILON {
        return false;
    }

    // Endpoints do not block; the path must cross the triangle interior.
    let path_parameter = edge_b.dot(cross_vector) * inverse_determinant;
    path_parameter > PARAMETER_EPSILON && path_parameter < 1.0 - PARAMETER_EPSILON
}

#[cfg(test)]
mod tests {
    //! Private boundary coverage for triangle intersection and image-source handling.

    use super::{
        PARAMETER_EPSILON, Vector3, first_reflection, path_intersects_triangle, point_in_triangle,
    };
    use crate::Point3;

    /// Makes a double-precision test vector.
    fn vector(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3 { x, y, z }
    }

    /// Makes finite test vertices in world-space meters.
    fn vertices() -> [Point3; 3] {
        [
            Point3::try_new(0.0, -1.0, -1.0).expect("finite triangle vertex"),
            Point3::try_new(0.0, 1.0, -1.0).expect("finite triangle vertex"),
            Point3::try_new(0.0, 0.0, 1.0).expect("finite triangle vertex"),
        ]
    }

    /// Triangle intersection distinguishes interior, parallel, outside, and endpoint paths.
    #[test]
    fn path_intersection_checks_triangle_and_open_segment() {
        let triangle = vertices();
        assert!(path_intersects_triangle(
            vector(-1.0, 0.0, 0.0),
            vector(1.0, 0.0, 0.0),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(-1.0, -2.0, 0.0),
            vector(-1.0, 2.0, 0.0),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(-1.0, 2.0, 0.0),
            vector(1.0, 2.0, 0.0),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(-1.0, 1.1, 0.4),
            vector(1.0, 1.1, 0.4),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(0.0, 0.0, 0.0),
            vector(1.0, 0.0, 0.0),
            triangle,
        ));
    }

    /// Overlapping bounds exercise the plane-parallel and barycentric rejection paths.
    #[test]
    fn path_intersection_checks_parallel_and_outside_barycentric_paths() {
        let triangle = vertices();
        assert!(!path_intersects_triangle(
            vector(0.0, -2.0, 0.0),
            vector(0.0, 2.0, 0.0),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(-1.0, 0.0, -1.0),
            vector(1.0, 2.2, -1.0),
            triangle,
        ));
        assert!(!path_intersects_triangle(
            vector(-1.0, 0.0, 0.0),
            vector(1.0, 2.2, 0.0),
            triangle,
        ));
    }

    /// A valid triangle hit remains accepted inside the tolerant barycentric boundary.
    #[test]
    fn path_intersection_preserves_triangle_boundary_tolerance() {
        let y_offset = PARAMETER_EPSILON * 0.5;
        assert!(path_intersects_triangle(
            vector(-1.0, 1.0 + y_offset, -1.0),
            vector(1.0, 1.0 + y_offset, -1.0),
            vertices(),
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
            first_reflection(vector(-1.0, 0.0, 0.0), vector(-1.0, 0.0, 2.0), far_vertices,)
                .is_none()
        );
    }

    /// A near-parallel image ray cannot produce a stable finite-triangle reflection.
    #[test]
    fn image_reflection_rejects_a_near_parallel_ray() {
        assert!(
            first_reflection(vector(1.0, 0.0, 0.0), vector(1.0, 1.0e15, 2.0), vertices(),)
                .is_none()
        );
    }
}
