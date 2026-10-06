//! CPU acoustic propagation over explicit line-segment surfaces in XY.

use crate::math2d::Vector2;
use crate::{
    AcousticResponse, BandEnergy, BandGain, Emitter2d, Listener2d, PathResponse, ReflectionPath2d,
    ReflectionSurfaceIndex, Segment2d, SolverPoint2d,
};

/// Tolerance for unitless segment parameters near path endpoints.
const PARAMETER_EPSILON: f64 = 1.0e-9;

/// Relative tolerance used to classify two 2D segments as parallel.
const PARALLEL_EPSILON: f64 = 1.0e-12;

/// A two-dimensional acoustic scene using meter coordinates in the XY plane.
#[derive(Clone, Debug, Default)]
pub struct AcousticScene2d {
    /// Explicitly registered opaque and reflective line segments.
    segments: Vec<Segment2d>,
}

impl AcousticScene2d {
    /// Adds one validated surface segment to the scene.
    pub fn add_segment(&mut self, segment: Segment2d) {
        self.segments.push(segment);
    }

    /// Removes every surface segment from the scene.
    pub fn clear(&mut self) {
        self.segments.clear();
    }

    /// Returns the number of registered surface segments.
    #[must_use]
    pub const fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Traces direct visibility and first-order reflections between two points.
    #[must_use]
    pub fn trace(&self, emitter: Emitter2d, listener: Listener2d) -> AcousticResponse {
        let source = Vector2::from_point(emitter.position());
        let receiver = Vector2::from_point(listener.position());
        let direct_distance = receiver.subtract(source).length();

        // If source and listener coincide, the path has no interior where an opaque surface can block it.
        let direct_occluded = direct_distance > 0.0
            && self.segments.iter().any(|segment| {
                path_intersects_segment(
                    source,
                    receiver,
                    Vector2::from_point(segment.start()),
                    Vector2::from_point(segment.end()),
                )
            });
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

    /// Returns geometrically visible first-order reflections lazily in surface insertion order; indices refer to this scene's current registration sequence.
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
                    reflector.material().absorption(),
                ))
            })
    }

    /// Checks whether another registered segment intersects an open path interior.
    fn segment_is_occluded(
        &self,
        start: Vector2,
        end: Vector2,
        skipped_index: Option<usize>,
    ) -> bool {
        self.segments.iter().enumerate().any(|(index, segment)| {
            if skipped_index == Some(index) {
                return false;
            }

            path_intersects_segment(
                start,
                end,
                Vector2::from_point(segment.start()),
                Vector2::from_point(segment.end()),
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

    use super::{PARAMETER_EPSILON, Vector2, first_reflection, path_intersects_segment};

    /// Makes a double-precision test point.
    fn point(x: f64, y: f64) -> Vector2 {
        Vector2 { x, y }
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
