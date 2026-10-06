//! CPU acoustic propagation over explicit triangular surfaces in 3D.

use crate::math3d::Vector3;
use crate::{
    AcousticResponse, BandEnergy, BandGain, Emitter3d, Listener3d, PathResponse, Triangle3d,
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

        // A zero-length path has no interior at which an opaque surface can block it.
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

        // Each valid image-source path contributes energy after visibility and absorption checks.
        let mut reflected_low = 0.0;
        let mut reflected_mid = 0.0;
        let mut reflected_high = 0.0;
        if direct_distance > 0.0 {
            for (reflector_index, reflector) in self.triangles.iter().enumerate() {
                let Some((reflection_point, reflected_distance)) =
                    first_reflection(source, receiver, reflector.vertices())
                else {
                    continue;
                };

                // Other triangles can block either leg, but the selected reflector ends both legs.
                if self.segment_is_occluded(source, reflection_point, Some(reflector_index))
                    || self.segment_is_occluded(reflection_point, receiver, Some(reflector_index))
                {
                    continue;
                }

                let distance_ratio = direct_distance / reflected_distance;
                let relative_energy = distance_ratio * distance_ratio;
                let absorption = reflector.material().absorption();
                // Fused accumulation adds each reflected path with one rounding per band.
                reflected_low =
                    relative_energy.mul_add(1.0 - f64::from(absorption.low()), reflected_low);
                reflected_mid =
                    relative_energy.mul_add(1.0 - f64::from(absorption.mid()), reflected_mid);
                reflected_high =
                    relative_energy.mul_add(1.0 - f64::from(absorption.high()), reflected_high);
            }
        }

        AcousticResponse::new(
            direct,
            BandEnergy::from_solver(reflected_low, reflected_mid, reflected_high),
        )
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
    vertices: [crate::Point3; 3],
) -> Option<(Vector3, f64)> {
    let [first, second, third] = vertices;
    let triangle_origin = Vector3::from_point(first);
    let edge_a = Vector3::from_point(second).subtract(triangle_origin);
    let edge_b = Vector3::from_point(third).subtract(triangle_origin);
    let normal = edge_a.cross(edge_b);
    let normal_squared = normal.dot(normal);
    let source_offset = source.subtract(triangle_origin).dot(normal);
    let receiver_offset = receiver.subtract(triangle_origin).dot(normal);

    // Specular reflection requires source and listener on the same side of the surface.
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

    // The image ray must reach the finite triangle between its endpoints.
    let ray_parameter = triangle_origin.subtract(image_source).dot(normal) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter) {
        return None;
    }
    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    if !point_in_triangle(reflection_point, triangle_origin, edge_a, edge_b) {
        return None;
    }

    let reflected_distance = image_to_receiver.length();
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance))
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
fn path_intersects_triangle(
    path_start: Vector3,
    path_end: Vector3,
    vertices: [crate::Point3; 3],
) -> bool {
    let [first, second, third] = vertices;
    let first = Vector3::from_point(first);
    let edge_a = Vector3::from_point(second).subtract(first);
    let edge_b = Vector3::from_point(third).subtract(first);
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

    use super::{Vector3, first_reflection, path_intersects_triangle, point_in_triangle};
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
