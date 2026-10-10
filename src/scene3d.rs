//! CPU acoustic propagation over material-bearing triangles in 3D.

use crate::bvh::{BoundingVolumeHierarchy, Bounds, SceneDimensions};
use crate::math3d::Vector3;
use crate::{
    AcousticResponse, BandEnergy, BandGain, Emitter3d, Listener3d, PathResponse, ReflectionPath3d,
    ReflectionSurfaceIndex, SolverPoint3d, Triangle3d,
};
use std::sync::OnceLock;

/// Tolerance for unitless segment and triangle coordinates.
const PARAMETER_EPSILON: f64 = 1.0e-9;

/// Relative tolerance used to classify a ray as parallel to a triangle.
const PARALLEL_EPSILON: f64 = 1.0e-12;

/// A three-dimensional acoustic scene using meter coordinates.
#[derive(Clone, Debug, Default)]
pub struct AcousticScene3d {
    /// Explicitly registered triangles with absorption and transmission.
    triangles: Vec<Triangle3d>,
    /// Lazily rebuilt broad-phase bounds after the last surface mutation.
    acceleration: OnceLock<SceneAcceleration3d>,
}

impl AcousticScene3d {
    /// Adds one validated surface triangle to the scene.
    pub fn add_triangle(&mut self, triangle: Triangle3d) {
        self.triangles.push(triangle);
        drop(self.acceleration.take());
    }

    /// Removes every surface triangle from the scene.
    pub fn clear(&mut self) {
        self.triangles.clear();
        drop(self.acceleration.take());
    }

    /// Returns the number of registered surface triangles.
    #[must_use]
    pub const fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Traces direct transmission and first-order reflections between two points.
    #[must_use]
    pub fn trace(&self, emitter: Emitter3d, listener: Listener3d) -> AcousticResponse {
        self.trace_internal(emitter, listener, None)
    }

    /// Returns visible first-order reflections lazily in triangle insertion order.
    ///
    /// Any other registered triangle crossing either open reflection leg omits the path,
    /// regardless of that triangle's direct transmission value.
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

    /// Traces a response while writing first-order paths into caller-owned reusable storage.
    ///
    /// The output is cleared before each query and retains its allocated capacity.
    #[must_use]
    pub fn trace_with_reflection_paths(
        &self,
        emitter: Emitter3d,
        listener: Listener3d,
        paths: &mut Vec<ReflectionPath3d>,
    ) -> AcousticResponse {
        self.trace_internal(emitter, listener, Some(paths))
    }

    /// Computes the aggregate response and optionally appends the same visible paths to reusable storage.
    fn trace_internal(
        &self,
        emitter: Emitter3d,
        listener: Listener3d,
        mut paths: Option<&mut Vec<ReflectionPath3d>>,
    ) -> AcousticResponse {
        if let Some(output) = paths.as_deref_mut() {
            output.clear();
        }

        let source = Vector3::from_point(emitter.position());
        let receiver = Vector3::from_point(listener.position());
        let direct_distance = receiver.subtract(source).length();

        let (direct_gain, direct_occluded) = self.direct_transmission(source, receiver);
        let direct = PathResponse::new(direct_distance, direct_gain, direct_occluded);

        // Fused accumulation preserves the existing aggregate result for every visible path.
        let mut reflected_low = 0.0;
        let mut reflected_mid = 0.0;
        let mut reflected_high = 0.0;
        for path in self.reflection_paths_between(source, receiver, direct_distance) {
            let (distance_ratio_squared, reflected_fraction) = path.accumulation_terms();
            // Fused accumulation adds each reflected path with one rounding per band.
            reflected_low = distance_ratio_squared.mul_add(reflected_fraction.low(), reflected_low);
            reflected_mid = distance_ratio_squared.mul_add(reflected_fraction.mid(), reflected_mid);
            reflected_high =
                distance_ratio_squared.mul_add(reflected_fraction.high(), reflected_high);

            // The optional path output uses the same validated candidate as aggregate energy.
            if let Some(output) = paths.as_deref_mut() {
                output.push(path);
            }
        }

        AcousticResponse::new(
            direct,
            BandEnergy::from_solver(reflected_low, reflected_mid, reflected_high),
        )
    }

    /// Multiplies transmission once per distinct surface crossing on an open path.
    /// Coplanar triangles with the same material share a crossing at mesh seams.
    ///
    /// Returns the accumulated band gain and whether any crossed triangle attenuates a band.
    pub(crate) fn direct_transmission(
        &self,
        source: Vector3,
        receiver: Vector3,
    ) -> (BandGain, bool) {
        // If source and listener coincide, the path has no interior where a surface can attenuate it.
        let acceleration = self.acceleration();
        let mut direct_occluded = false;
        let mut direct_gain = BandGain::UNITY;
        let mut crossings: Vec<(f64, Vector3, BandGain)> = Vec::new();
        if receiver.subtract(source).length() > 0.0 {
            let _traversal_completed = acceleration.hierarchy.visit_candidates_until(
                Bounds::path_3d(
                    (source.x, source.y, source.z),
                    (receiver.x, receiver.y, receiver.z),
                ),
                None,
                |index| {
                    if let Some(triangle) = acceleration.triangles.get(index)
                        && let Some(parameter) = path_triangle_parameter(source, receiver, triangle)
                    {
                        let normal = triangle.normal.scale(1.0 / triangle.normal_length);
                        if crossings.iter().any(|&(previous, previous_normal, gain)| {
                            (previous - parameter).abs() <= PARAMETER_EPSILON
                                && previous_normal.dot(normal).abs() >= 1.0 - PARALLEL_EPSILON
                                && gain == triangle.transmission
                        }) {
                            return true;
                        }
                        crossings.push((parameter, normal, triangle.transmission));
                        direct_occluded |= triangle.transmission != BandGain::UNITY;
                        direct_gain = direct_gain.multiply(triangle.transmission);
                    }
                    direct_gain != BandGain::ZERO
                },
            );
        }
        (direct_gain, direct_occluded)
    }

    /// Returns the registered triangles in insertion order.
    pub(crate) fn triangles(&self) -> &[Triangle3d] {
        &self.triangles
    }

    /// Builds a lazy path iterator from the query's already-converted endpoints and distance.
    fn reflection_paths_between(
        &self,
        source: Vector3,
        receiver: Vector3,
        direct_distance: f64,
    ) -> impl Iterator<Item = ReflectionPath3d> + '_ {
        let acceleration = self.acceleration();
        acceleration
            .triangles
            .iter()
            .enumerate()
            .filter_map(move |(index, reflector)| {
                // If source and listener coincide, this triangle cannot form a positive-length reflected path.
                if direct_distance == 0.0 {
                    return None;
                }

                let (reflection_point, reflected_distance, image_source) =
                    first_reflection(source, receiver, reflector)?;

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
                    reflector.reflected_fraction,
                ))
            })
    }

    /// Checks whether another registered triangle crosses an open reflection leg.
    pub(crate) fn segment_is_occluded(
        &self,
        start: Vector3,
        end: Vector3,
        skipped_index: Option<usize>,
    ) -> bool {
        let acceleration = self.acceleration();
        acceleration.hierarchy.any_intersection(
            Bounds::path_3d((start.x, start.y, start.z), (end.x, end.y, end.z)),
            skipped_index,
            |index| {
                acceleration
                    .triangles
                    .get(index)
                    .is_some_and(|triangle| path_intersects_triangle(start, end, triangle))
            },
        )
    }

    /// Builds one deterministic hierarchy and reuses it until a surface mutation.
    pub(crate) fn acceleration(&self) -> &SceneAcceleration3d {
        self.acceleration.get_or_init(|| {
            let triangles: Vec<_> = self
                .triangles
                .iter()
                .copied()
                .map(TriangleGeometry::new)
                .collect();
            let hierarchy = BoundingVolumeHierarchy::build(
                triangles
                    .iter()
                    .enumerate()
                    .map(|(index, triangle)| (index, triangle.bounds)),
                SceneDimensions::Three,
            );
            SceneAcceleration3d {
                hierarchy,
                triangles,
            }
        })
    }
}

/// Cached triangle values derived once after a scene mutation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TriangleGeometry {
    /// First vertex, used as the origin for barycentric coordinates.
    pub(crate) origin: Vector3,
    /// Edge from the first vertex to the second vertex.
    pub(crate) edge_a: Vector3,
    /// Edge from the first vertex to the third vertex.
    pub(crate) edge_b: Vector3,
    /// Unnormalized normal retained for reflection and ray intersection math.
    pub(crate) normal: Vector3,
    /// Squared normal length used to reflect the source point.
    normal_squared: f64,
    /// Normal length used for scale-relative parallel checks.
    pub(crate) normal_length: f64,
    /// First edge length used for scale-relative parallel checks.
    pub(crate) edge_a_length: f64,
    /// Second edge length used for scale-relative parallel checks.
    pub(crate) edge_b_length: f64,
    /// Precomputed inner products used by barycentric projection.
    barycentric_basis: BarycentricBasis,
    /// Triangle bounds padded by the narrow-phase barycentric tolerance.
    bounds: Bounds,
    /// Incident-energy fraction available for reflection in each band.
    reflected_fraction: BandEnergy,
    /// Material transmission applied when a direct ray crosses this triangle.
    transmission: BandGain,
}

impl TriangleGeometry {
    /// Converts one validated triangle into reusable double-precision geometry.
    fn new(triangle: Triangle3d) -> Self {
        let material = triangle.material();
        let [first, second, third] = triangle.vertices();
        let origin = Vector3::from_point(first);
        let second = Vector3::from_point(second);
        let third = Vector3::from_point(third);
        let edge_a = second.subtract(origin);
        let edge_b = third.subtract(origin);
        let normal = edge_a.cross(edge_b);
        let normal_squared = normal.dot(normal);
        let barycentric_basis = BarycentricBasis::new(edge_a, edge_b);
        let bounds = Bounds::triangle_surface_3d(
            [
                (origin.x, origin.y, origin.z),
                (second.x, second.y, second.z),
                (third.x, third.y, third.z),
            ],
            PARAMETER_EPSILON,
        );

        Self {
            origin,
            edge_a,
            edge_b,
            normal,
            normal_squared,
            normal_length: normal.length(),
            edge_a_length: edge_a.length(),
            edge_b_length: edge_b.length(),
            barycentric_basis,
            bounds,
            reflected_fraction: material.reflected_fraction(),
            transmission: material.transmission(),
        }
    }

    /// Tests triangle membership with the precomputed Gram determinant and edge products.
    fn contains_point(self, point: Vector3) -> bool {
        point_in_triangle_with_basis(
            point,
            self.origin,
            self.edge_a,
            self.edge_b,
            self.barycentric_basis,
        )
    }
}

/// Inner products and determinant for one triangle's barycentric basis.
#[derive(Clone, Copy, Debug)]
struct BarycentricBasis {
    /// Squared first edge length.
    first_edge_length_squared: f64,
    /// Product of the two edge vectors.
    edge_dot: f64,
    /// Squared second edge length.
    second_edge_length_squared: f64,
    /// Gram determinant used to reject a degenerate basis.
    denominator: f64,
}

impl BarycentricBasis {
    /// Computes the inner products once for a stable pair of triangle edges.
    fn new(edge_a: Vector3, edge_b: Vector3) -> Self {
        let first_edge_length_squared = edge_a.dot(edge_a);
        let edge_dot = edge_a.dot(edge_b);
        let second_edge_length_squared = edge_b.dot(edge_b);
        let denominator = edge_dot.mul_add(
            -edge_dot,
            first_edge_length_squared * second_edge_length_squared,
        );
        Self {
            first_edge_length_squared,
            edge_dot,
            second_edge_length_squared,
            denominator,
        }
    }
}

/// Lazily derived per-scene geometry and its bounds hierarchy.
#[derive(Clone, Debug)]
pub(crate) struct SceneAcceleration3d {
    /// Balanced hierarchy for broad-phase candidate rejection.
    pub(crate) hierarchy: BoundingVolumeHierarchy,
    /// Triangles with cached coordinates and material properties.
    pub(crate) triangles: Vec<TriangleGeometry>,
}

/// Finds one valid image-source reflection and returns its point and path length.
fn first_reflection(
    source: Vector3,
    receiver: Vector3,
    triangle: &TriangleGeometry,
) -> Option<(Vector3, f64, Vector3)> {
    let source_offset = source.subtract(triangle.origin).dot(triangle.normal);
    let receiver_offset = receiver.subtract(triangle.origin).dot(triangle.normal);

    // If either endpoint lies on the surface or they lie on opposite sides, no image-source reflection exists.
    if source_offset == 0.0
        || receiver_offset == 0.0
        || source_offset.is_sign_positive() != receiver_offset.is_sign_positive()
    {
        return None;
    }

    let image_source = source.subtract(
        triangle
            .normal
            .scale(2.0 * source_offset / triangle.normal_squared),
    );
    let image_to_receiver = receiver.subtract(image_source);
    let denominator = image_to_receiver.dot(triangle.normal);
    let denominator_scale = image_to_receiver.length() * triangle.normal_length;
    if denominator.abs() <= PARALLEL_EPSILON * denominator_scale {
        return None;
    }

    // If the image ray misses the finite triangle, the candidate cannot reflect from this surface.
    let ray_parameter = triangle.origin.subtract(image_source).dot(triangle.normal) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter) {
        return None;
    }
    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    let reflection_point_bounds = Bounds::path_3d(
        (reflection_point.x, reflection_point.y, reflection_point.z),
        (reflection_point.x, reflection_point.y, reflection_point.z),
    );
    if !triangle.bounds.overlaps(reflection_point_bounds) {
        return None;
    }
    if !triangle.contains_point(reflection_point) {
        return None;
    }

    let reflected_distance = image_to_receiver.length();
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance, image_source))
}

/// Tests whether a point lies inside or on the boundary of one triangle.
#[cfg(test)]
fn point_in_triangle(point: Vector3, origin: Vector3, edge_a: Vector3, edge_b: Vector3) -> bool {
    point_in_triangle_with_basis(
        point,
        origin,
        edge_a,
        edge_b,
        BarycentricBasis::new(edge_a, edge_b),
    )
}

/// Projects a point through precomputed edge products and the Gram determinant.
fn point_in_triangle_with_basis(
    point: Vector3,
    origin: Vector3,
    edge_a: Vector3,
    edge_b: Vector3,
    basis: BarycentricBasis,
) -> bool {
    let relative = point.subtract(origin);
    let relative_dot_a = relative.dot(edge_a);
    let relative_dot_b = relative.dot(edge_b);
    if basis.denominator == 0.0 {
        return false;
    }

    let coordinate_a = basis.edge_dot.mul_add(
        -relative_dot_b,
        basis.second_edge_length_squared * relative_dot_a,
    ) / basis.denominator;
    let coordinate_b = basis.edge_dot.mul_add(
        -relative_dot_a,
        basis.first_edge_length_squared * relative_dot_b,
    ) / basis.denominator;
    coordinate_a >= -PARAMETER_EPSILON
        && coordinate_b >= -PARAMETER_EPSILON
        && coordinate_a + coordinate_b <= 1.0 + PARAMETER_EPSILON
}

/// Tests an open path against one finite triangle using a double-precision ray test.
fn path_intersects_triangle(
    path_start: Vector3,
    path_end: Vector3,
    triangle: &TriangleGeometry,
) -> bool {
    path_triangle_parameter(path_start, path_end, triangle).is_some()
}

/// Returns the crossing fraction along an open path, including shared triangle edges.
fn path_triangle_parameter(
    path_start: Vector3,
    path_end: Vector3,
    triangle: &TriangleGeometry,
) -> Option<f64> {
    let path_bounds = Bounds::path_3d(
        (path_start.x, path_start.y, path_start.z),
        (path_end.x, path_end.y, path_end.z),
    );
    if !triangle.bounds.overlaps(path_bounds) {
        return None;
    }

    let edge_a = triangle.edge_a;
    let edge_b = triangle.edge_b;
    let direction = path_end.subtract(path_start);
    let determinant_vector = direction.cross(edge_b);
    let determinant = edge_a.dot(determinant_vector);
    let determinant_scale = direction.length() * triangle.edge_a_length * triangle.edge_b_length;
    if determinant.abs() <= PARALLEL_EPSILON * determinant_scale {
        // A path parallel to the plane does not cross the triangle surface.
        return None;
    }

    let inverse_determinant = 1.0 / determinant;
    let origin_delta = path_start.subtract(triangle.origin);
    let coordinate_a = origin_delta.dot(determinant_vector) * inverse_determinant;
    if !(-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&coordinate_a) {
        return None;
    }

    let cross_vector = origin_delta.cross(edge_a);
    let coordinate_b = direction.dot(cross_vector) * inverse_determinant;
    if coordinate_b < -PARAMETER_EPSILON || coordinate_a + coordinate_b > 1.0 + PARAMETER_EPSILON {
        return None;
    }

    // Endpoints do not block; the path must cross the triangle interior.
    let path_parameter = edge_b.dot(cross_vector) * inverse_determinant;
    (path_parameter > PARAMETER_EPSILON && path_parameter < 1.0 - PARAMETER_EPSILON)
        .then_some(path_parameter)
}

#[cfg(test)]
mod tests {
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
                scene.add_triangle(
                    Triangle3d::try_new(indices.map(|i| corners[i]), material).unwrap(),
                );
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
            first_reflection(vector(-1.0, 11.8, 1.8), vector(-3.0, 11.8, 1.8), &triangle,)
                .is_none()
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
}
