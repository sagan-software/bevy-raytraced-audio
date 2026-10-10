//! CPU acoustic propagation over material-bearing line segments in XY.

use crate::bvh::{BoundingVolumeHierarchy, Bounds, SceneDimensions};
use crate::math2d::Vector2;
use crate::{
    AcousticResponse, BandEnergy, BandGain, Emitter2d, Listener2d, PathResponse, ReflectionPath2d,
    ReflectionSurfaceIndex, Segment2d, SolverPoint2d,
};
use std::sync::OnceLock;

/// Tolerance for unitless segment parameters near path endpoints.
const PARAMETER_EPSILON: f64 = 1.0e-9;

/// Relative tolerance used to classify two 2D segments as parallel.
const PARALLEL_EPSILON: f64 = 1.0e-12;

/// A two-dimensional acoustic scene using meter coordinates in the XY plane.
#[derive(Clone, Debug, Default)]
pub struct AcousticScene2d {
    /// Explicitly registered line segments with absorption and transmission.
    segments: Vec<Segment2d>,
    /// Lazily rebuilt broad-phase bounds after the last surface mutation.
    acceleration: OnceLock<BoundingVolumeHierarchy>,
}

impl AcousticScene2d {
    /// Adds one validated surface segment to the scene.
    pub fn add_segment(&mut self, segment: Segment2d) {
        self.segments.push(segment);
        drop(self.acceleration.take());
    }

    /// Removes every surface segment from the scene.
    pub fn clear(&mut self) {
        self.segments.clear();
        drop(self.acceleration.take());
    }

    /// Returns the number of registered surface segments.
    #[must_use]
    pub const fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Traces direct transmission and first-order reflections between two points.
    #[must_use]
    pub fn trace(&self, emitter: Emitter2d, listener: Listener2d) -> AcousticResponse {
        self.trace_internal(emitter, listener, None)
    }

    /// Returns visible first-order reflections lazily in surface insertion order.
    ///
    /// Any other registered segment crossing either open reflection leg omits the path, regardless
    /// of that segment's direct transmission value.
    pub fn reflection_paths(
        &self,
        emitter: Emitter2d,
        listener: Listener2d,
    ) -> impl Iterator<Item = ReflectionPath2d> + '_ {
        let source = Vector2::from_point(emitter.position());
        let receiver = Vector2::from_point(listener.position());
        let direct_distance = receiver.subtract(source).length();
        self.reflection_paths_between(source, receiver, direct_distance)
    }

    /// Traces a response while writing first-order paths into caller-owned reusable storage.
    ///
    /// The output is cleared before each query and retains its allocated capacity.
    #[must_use]
    pub fn trace_with_reflection_paths(
        &self,
        emitter: Emitter2d,
        listener: Listener2d,
        paths: &mut Vec<ReflectionPath2d>,
    ) -> AcousticResponse {
        self.trace_internal(emitter, listener, Some(paths))
    }

    /// Computes the aggregate response and optionally appends the same visible paths to reusable storage.
    fn trace_internal(
        &self,
        emitter: Emitter2d,
        listener: Listener2d,
        mut paths: Option<&mut Vec<ReflectionPath2d>>,
    ) -> AcousticResponse {
        if let Some(output) = paths.as_deref_mut() {
            output.clear();
        }

        let source = Vector2::from_point(emitter.position());
        let receiver = Vector2::from_point(listener.position());
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

    /// Multiplies transmission once per distinct surface crossing, including collinear mesh seams.
    ///
    /// Returns the accumulated band gain and whether any crossed surface attenuates a band.
    pub(crate) fn direct_transmission(
        &self,
        source: Vector2,
        receiver: Vector2,
    ) -> (BandGain, bool) {
        // If source and listener coincide, the path has no interior where a surface can attenuate it.
        let mut direct_occluded = false;
        let mut direct_gain = BandGain::UNITY;
        let mut crossings: Vec<(f64, Vector2, BandGain)> = Vec::new();
        if receiver.subtract(source).length() > 0.0 {
            let _traversal_completed = self.acceleration().visit_candidates_until(
                Bounds::path_2d((source.x, source.y), (receiver.x, receiver.y)),
                None,
                |index| {
                    if let Some(segment) = self.segments.get(index)
                        && path_intersects_segment(
                            source,
                            receiver,
                            Vector2::from_point(segment.start()),
                            Vector2::from_point(segment.end()),
                        )
                    {
                        let transmission = segment.material().transmission();
                        let start = Vector2::from_point(segment.start());
                        let edge = Vector2::from_point(segment.end()).subtract(start);
                        let parameter = start.subtract(source).cross(edge)
                            / receiver.subtract(source).cross(edge);
                        let direction = edge.scale(1.0 / edge.length());
                        if crossings
                            .iter()
                            .any(|&(previous, previous_direction, gain)| {
                                (previous - parameter).abs() <= PARAMETER_EPSILON
                                    && previous_direction.dot(direction).abs()
                                        >= 1.0 - PARALLEL_EPSILON
                                    && gain == transmission
                            })
                        {
                            return true;
                        }
                        crossings.push((parameter, direction, transmission));
                        direct_occluded |= transmission != BandGain::UNITY;
                        direct_gain = direct_gain.multiply(transmission);
                    }
                    direct_gain != BandGain::ZERO
                },
            );
        }
        (direct_gain, direct_occluded)
    }

    /// Returns the registered segments in insertion order.
    pub(crate) fn segments(&self) -> &[Segment2d] {
        &self.segments
    }

    /// Builds a lazy path iterator from the query's already-converted endpoints and distance.
    fn reflection_paths_between(
        &self,
        source: Vector2,
        receiver: Vector2,
        direct_distance: f64,
    ) -> impl Iterator<Item = ReflectionPath2d> + '_ {
        self.segments
            .iter()
            .enumerate()
            .filter_map(move |(index, reflector)| {
                // If source and listener coincide, this surface cannot form a positive-length reflected path.
                if direct_distance == 0.0 {
                    return None;
                }

                let reflector_start = Vector2::from_point(reflector.start());
                let reflector_end = Vector2::from_point(reflector.end());
                let (reflection_point, reflected_distance, image_source) =
                    first_reflection(source, receiver, reflector_start, reflector_end)?;

                // If another surface crosses either leg, this reflected candidate is omitted.
                if self.segment_is_occluded(source, reflection_point, Some(index))
                    || self.segment_is_occluded(reflection_point, receiver, Some(index))
                {
                    return None;
                }

                let distance_ratio = direct_distance / reflected_distance;
                Some(ReflectionPath2d::new(
                    ReflectionSurfaceIndex::new(index),
                    SolverPoint2d::new(reflection_point.x, reflection_point.y),
                    SolverPoint2d::new(image_source.x, image_source.y),
                    reflected_distance,
                    distance_ratio * distance_ratio,
                    reflector.material().reflected_fraction(),
                ))
            })
    }

    /// Checks whether another registered segment crosses an open reflection leg.
    pub(crate) fn segment_is_occluded(
        &self,
        start: Vector2,
        end: Vector2,
        skipped_index: Option<usize>,
    ) -> bool {
        self.acceleration().any_intersection(
            Bounds::path_2d((start.x, start.y), (end.x, end.y)),
            skipped_index,
            |index| {
                self.segments.get(index).is_some_and(|segment| {
                    path_intersects_segment(
                        start,
                        end,
                        Vector2::from_point(segment.start()),
                        Vector2::from_point(segment.end()),
                    )
                })
            },
        )
    }

    /// Builds one deterministic hierarchy and reuses it until a surface mutation.
    pub(crate) fn acceleration(&self) -> &BoundingVolumeHierarchy {
        self.acceleration.get_or_init(|| {
            BoundingVolumeHierarchy::build(
                self.segments.iter().enumerate().map(|(index, segment)| {
                    let start = Vector2::from_point(segment.start());
                    let end = Vector2::from_point(segment.end());
                    (
                        index,
                        Bounds::segment_surface_2d(
                            (start.x, start.y),
                            (end.x, end.y),
                            PARAMETER_EPSILON,
                        ),
                    )
                }),
                SceneDimensions::Two,
            )
        })
    }
}

/// Finds one valid image-source reflection and returns its point and path length.
fn first_reflection(
    source: Vector2,
    receiver: Vector2,
    wall_start: Vector2,
    wall_end: Vector2,
) -> Option<(Vector2, f64, Vector2)> {
    let wall = wall_end.subtract(wall_start);
    let normal = Vector2 {
        x: -wall.y,
        y: wall.x,
    };
    let normal_squared = normal.dot(normal);
    let source_offset = source.subtract(wall_start).dot(normal);
    let receiver_offset = receiver.subtract(wall_start).dot(normal);

    // If either endpoint lies on the surface or they lie on opposite sides, no image-source reflection exists.
    if source_offset == 0.0
        || receiver_offset == 0.0
        || source_offset.is_sign_positive() != receiver_offset.is_sign_positive()
    {
        return None;
    }

    let image_source = source.subtract(normal.scale(2.0 * source_offset / normal_squared));
    let image_to_receiver = receiver.subtract(image_source);
    let denominator = image_to_receiver.cross(wall);
    let scale = image_to_receiver.length() * wall.length();
    if denominator.abs() <= PARALLEL_EPSILON * scale {
        return None;
    }

    // If the image ray misses the finite wall segment, the candidate cannot reflect from this surface.
    let image_to_wall = wall_start.subtract(image_source);
    let ray_parameter = image_to_wall.cross(wall) / denominator;
    let wall_parameter = image_to_wall.cross(image_to_receiver) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter)
        || !(-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&wall_parameter)
    {
        return None;
    }

    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    let reflected_distance = image_to_receiver.length();
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance, image_source))
}

/// Tests an open path against a finite 2D segment, including wall endpoints.
fn path_intersects_segment(
    path_start: Vector2,
    path_end: Vector2,
    wall_start: Vector2,
    wall_end: Vector2,
) -> bool {
    let path_min_x = path_start.x.min(path_end.x);
    let path_max_x = path_start.x.max(path_end.x);
    let path_min_y = path_start.y.min(path_end.y);
    let path_max_y = path_start.y.max(path_end.y);
    let surface_padding_x = PARAMETER_EPSILON * (wall_end.x - wall_start.x).abs();
    let surface_padding_y = PARAMETER_EPSILON * (wall_end.y - wall_start.y).abs();
    let surface_min_x = wall_start.x.min(wall_end.x) - surface_padding_x;
    let surface_max_x = wall_start.x.max(wall_end.x) + surface_padding_x;
    let surface_min_y = wall_start.y.min(wall_end.y) - surface_padding_y;
    let surface_max_y = wall_start.y.max(wall_end.y) + surface_padding_y;

    // If the path and tolerant surface bounds do not overlap, they cannot intersect.
    if path_max_x < surface_min_x
        || surface_max_x < path_min_x
        || path_max_y < surface_min_y
        || surface_max_y < path_min_y
    {
        return false;
    }

    let path = path_end.subtract(path_start);
    let wall = wall_end.subtract(wall_start);
    let path_length = path.length();
    let wall_length = wall.length();
    if path_length == 0.0 || wall_length == 0.0 {
        return false;
    }

    let start_delta = wall_start.subtract(path_start);
    let denominator = path.cross(wall);
    if denominator.abs() <= PARALLEL_EPSILON * path_length * wall_length {
        // Collinear overlap blocks only when it reaches the open path interior.
        if start_delta.cross(path).abs() > PARALLEL_EPSILON * path_length * start_delta.length() {
            return false;
        }

        let path_length_squared = path.dot(path);
        let first_parameter = start_delta.dot(path) / path_length_squared;
        let second_parameter = wall_end.subtract(path_start).dot(path) / path_length_squared;
        let overlap_start = first_parameter.min(second_parameter);
        let overlap_end = first_parameter.max(second_parameter);
        return overlap_end > PARAMETER_EPSILON && overlap_start < 1.0 - PARAMETER_EPSILON;
    }

    // Parallelism was rejected above, so these parameters identify one segment crossing.
    let path_parameter = start_delta.cross(wall) / denominator;
    let wall_parameter = start_delta.cross(path) / denominator;
    path_parameter > PARAMETER_EPSILON
        && path_parameter < 1.0 - PARAMETER_EPSILON
        && (-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&wall_parameter)
}

#[cfg(test)]
mod tests {
    //! Private boundary coverage for line intersection and image-source edge cases.

    use super::{
        AcousticScene2d, PARAMETER_EPSILON, Vector2, first_reflection, path_intersects_segment,
    };
    use crate::{AcousticMaterial, Emitter2d, Listener2d, Point2, Segment2d};

    /// Splitting a straight wall into adjoining segments cannot double attenuation at the join.
    #[test]
    fn transmission_is_continuous_at_segment_seams() {
        use crate::{AcousticMaterial, BandGain, Point2, Segment2d};
        let gain = BandGain::try_new(0.5, 0.3, 0.1).unwrap();
        let material = AcousticMaterial::default()
            .try_with_transmission(gain)
            .unwrap();
        let mut scene = AcousticScene2d::default();
        for x in [0.0, 0.2] {
            for (a, b) in [(-1.0, 0.0), (0.0, 1.0)] {
                scene.add_segment(
                    Segment2d::try_new(
                        Point2::try_new(x, a).unwrap(),
                        Point2::try_new(x, b).unwrap(),
                        material,
                    )
                    .unwrap(),
                );
            }
        }
        for y in [-0.01, 0.0, 0.01] {
            let (actual, _) = scene.direct_transmission(point(-1.0, y), point(1.0, y));
            assert_eq!(actual, gain.multiply(gain));
        }
    }

    /// Makes a double-precision test point.
    fn point(x: f64, y: f64) -> Vector2 {
        Vector2 { x, y }
    }

    /// Captured paths match the response query and stale outputs clear without losing capacity.
    #[test]
    fn trace_with_paths_reuses_storage_and_preserves_response() {
        let mut scene = AcousticScene2d::default();
        let reflector = Segment2d::try_new(
            Point2::try_new(0.0, -2.0).expect("finite reflector start"),
            Point2::try_new(0.0, 2.0).expect("finite reflector end"),
            AcousticMaterial::default(),
        )
        .expect("nondegenerate reflector");
        scene.add_segment(reflector);
        let emitter = Emitter2d::new(Point2::try_new(-1.0, 0.0).expect("finite emitter"));
        let listener = Listener2d::new(Point2::try_new(-3.0, 0.0).expect("finite listener"));
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

    /// Collinear overlap blocks an open path, while endpoint touch and parallel offset do not.
    #[test]
    fn segment_intersection_handles_parallel_and_endpoint_cases() {
        assert!(path_intersects_segment(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(0.5, 0.0),
            point(1.5, 0.0),
        ));
        assert!(!path_intersects_segment(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, 0.0),
            point(3.0, 0.0),
        ));
        assert!(!path_intersects_segment(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(0.0, 1.0),
            point(2.0, 1.0),
        ));
        assert!(!path_intersects_segment(
            point(0.0, 0.0),
            point(0.0, 0.0),
            point(-1.0, 0.0),
            point(1.0, 0.0),
        ));
    }

    /// A valid wall hit remains accepted inside the tolerant segment-parameter boundary.
    #[test]
    fn segment_intersection_preserves_wall_endpoint_tolerance() {
        let y_offset = PARAMETER_EPSILON * 0.5;
        assert!(path_intersects_segment(
            point(-1.0, 1.0 + y_offset),
            point(1.0, 1.0 + y_offset),
            point(0.0, 0.0),
            point(0.0, 1.0),
        ));
    }

    /// A wall endpoint can block a crossing, and a reflection must meet the finite wall.
    #[test]
    fn image_reflection_checks_side_and_segment_bounds() {
        assert!(path_intersects_segment(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(1.0, 0.0),
            point(1.0, 1.0),
        ));
        assert!(
            first_reflection(
                point(0.0, 1.0),
                point(0.0, -1.0),
                point(-1.0, 0.0),
                point(1.0, 0.0),
            )
            .is_none()
        );
        assert!(
            first_reflection(
                point(0.0, 1.0),
                point(0.0, 0.0),
                point(-1.0, 0.0),
                point(1.0, 0.0),
            )
            .is_none()
        );
        assert!(
            first_reflection(
                point(0.0, 1.0),
                point(4.0, 1.0),
                point(3.0, 0.0),
                point(4.0, 0.0),
            )
            .is_none()
        );
        assert!(
            first_reflection(
                point(0.0, -1.0),
                point(1.0e15, -2.0),
                point(-1.0, 0.0),
                point(1.0, 0.0),
            )
            .is_none()
        );
    }
}
