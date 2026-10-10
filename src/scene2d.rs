//! CPU acoustic propagation over material-bearing line segments in XY.

use crate::bvh::{BoundingVolumeHierarchy, Bounds, Ray, SceneDimensions};
use crate::math2d::Vector2;
use crate::{
    AcousticMaterial, AcousticResponse, BandEnergy, BandGain, Emitter2d, Listener2d, PathResponse,
    ReflectionPath2d, ReflectionSurfaceIndex, Segment2d, SolverPoint2d,
};
use std::sync::{Arc, OnceLock};

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
    acceleration: OnceLock<SceneAcceleration2d>,
    /// Shared identity for the current immutable geometry, created only when traced.
    revision: OnceLock<Arc<()>>,
}

impl AcousticScene2d {
    /// Stable identity shared by unchanged clones; mutations invalidate it without address reuse.
    pub(crate) fn revision(&self) -> &Arc<()> {
        self.revision.get_or_init(|| Arc::new(()))
    }

    /// Adds one validated surface segment to the scene.
    pub fn add_segment(&mut self, segment: Segment2d) {
        self.segments.push(segment);
        drop(self.acceleration.take());
        drop(self.revision.take());
    }

    /// Removes every surface segment from the scene.
    pub fn clear(&mut self) {
        self.segments.clear();
        drop(self.acceleration.take());
        drop(self.revision.take());
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

        if self.segments.is_empty() {
            return AcousticResponse::new(
                PathResponse::new(direct_distance, BandGain::UNITY, false),
                BandEnergy::ZERO,
            );
        }

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
        let acceleration = self.acceleration();
        let mut crossings: Vec<(f64, Vector2, BandGain)> = Vec::new();
        if receiver.subtract(source).length() > 0.0 {
            let offset = receiver.subtract(source);
            let _traversal_completed = acceleration.hierarchy.visit_segment_candidates(
                Ray::new((source.x, source.y, 0.0), (offset.x, offset.y, 0.0)),
                Bounds::path_2d((source.x, source.y), (receiver.x, receiver.y)),
                None,
                |index| {
                    if let Some(segment) = acceleration.segments.get(index)
                        && path_intersects_geometry(source, receiver, segment)
                    {
                        let transmission = segment.transmission;
                        let start = segment.start;
                        let edge = segment.edge;
                        let parameter = start.subtract(source).cross(edge)
                            / receiver.subtract(source).cross(edge);
                        let direction = edge.scale(segment.length.recip());
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
        let acceleration = self.acceleration();
        let candidates = if acceleration.hierarchy.may_have_reflections(Bounds::path_2d(
            (source.x, source.y),
            (receiver.x, receiver.y),
        )) {
            acceleration.segments.as_slice()
        } else {
            &[]
        };
        candidates
            .iter()
            .enumerate()
            .filter_map(move |(index, reflector)| {
                // If source and listener coincide, this surface cannot form a positive-length reflected path.
                if direct_distance == 0.0 {
                    return None;
                }

                let (reflection_point, reflected_distance, image_source) =
                    first_reflection_geometry(source, receiver, reflector)?;

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
                    reflector.reflected_fraction,
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
        let acceleration = self.acceleration();
        let offset = end.subtract(start);
        !acceleration.hierarchy.visit_segment_candidates(
            Ray::new((start.x, start.y, 0.0), (offset.x, offset.y, 0.0)),
            Bounds::path_2d((start.x, start.y), (end.x, end.y)),
            skipped_index,
            |index| {
                !acceleration
                    .segments
                    .get(index)
                    .is_some_and(|segment| path_intersects_geometry(start, end, segment))
            },
        )
    }

    /// Builds one deterministic hierarchy and reuses it until a surface mutation.
    pub(crate) fn acceleration(&self) -> &SceneAcceleration2d {
        self.acceleration.get_or_init(|| {
            let segments: Vec<_> = self
                .segments
                .iter()
                .map(|segment| {
                    SegmentGeometry::new(
                        Vector2::from_point(segment.start()),
                        Vector2::from_point(segment.end()),
                        segment.material(),
                    )
                })
                .collect();
            let hierarchy = BoundingVolumeHierarchy::build(
                segments
                    .iter()
                    .enumerate()
                    .map(|(index, segment)| (index, segment.bounds)),
                SceneDimensions::Two,
            );
            SceneAcceleration2d {
                hierarchy,
                segments,
            }
        })
    }
}

/// Geometry and material terms reused by every query until a surface mutation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SegmentGeometry {
    /// Segment origin in solver precision.
    pub(crate) start: Vector2,
    /// Segment end in solver precision.
    pub(crate) end: Vector2,
    /// Difference between endpoints.
    pub(crate) edge: Vector2,
    /// Perpendicular unnormalized wall normal.
    pub(crate) normal: Vector2,
    /// Wall length, also the normal length.
    pub(crate) length: f64,
    /// Squared normal length used by image-source reflection.
    normal_squared: f64,
    /// Bounds including the exact geometry predicate's tolerance.
    bounds: Bounds,
    /// Fraction of energy available to reflection.
    reflected_fraction: BandEnergy,
    /// Transmission applied at each distinct wall crossing.
    transmission: BandGain,
}

impl SegmentGeometry {
    /// Converts one wall to cached double-precision geometry and material terms.
    fn new(start: Vector2, end: Vector2, material: AcousticMaterial) -> Self {
        let edge = end.subtract(start);
        let normal = Vector2 {
            x: -edge.y,
            y: edge.x,
        };
        Self {
            start,
            end,
            edge,
            normal,
            length: edge.length(),
            normal_squared: normal.dot(normal),
            bounds: Bounds::segment_surface_2d(
                (start.x, start.y),
                (end.x, end.y),
                PARAMETER_EPSILON,
            ),
            reflected_fraction: material.reflected_fraction(),
            transmission: material.transmission(),
        }
    }
}

/// Lazily derived 2D geometry and its bounds hierarchy.
#[derive(Clone, Debug)]
pub(crate) struct SceneAcceleration2d {
    /// Balanced hierarchy for broad-phase candidate rejection.
    pub(crate) hierarchy: BoundingVolumeHierarchy,
    /// Segments with cached coordinates and material properties.
    pub(crate) segments: Vec<SegmentGeometry>,
}

/// Finds one valid image-source reflection and returns its point and path length.
fn first_reflection_geometry(
    source: Vector2,
    receiver: Vector2,
    geometry: &SegmentGeometry,
) -> Option<(Vector2, f64, Vector2)> {
    let wall_start = geometry.start;
    let wall = geometry.edge;
    let normal = geometry.normal;
    let normal_squared = geometry.normal_squared;
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

    // If the image ray misses the finite wall segment, the candidate cannot reflect from this surface.
    let image_to_wall = wall_start.subtract(image_source);
    let ray_parameter = image_to_wall.cross(wall) / denominator;
    let wall_parameter = image_to_wall.cross(image_to_receiver) / denominator;
    if !(0.0..=1.0).contains(&ray_parameter)
        || !(-PARAMETER_EPSILON..=1.0 + PARAMETER_EPSILON).contains(&wall_parameter)
    {
        return None;
    }

    let reflected_distance = image_to_receiver.length();
    if denominator.abs() <= PARALLEL_EPSILON * (reflected_distance * geometry.length) {
        return None;
    }
    let reflection_point = image_source.add(image_to_receiver.scale(ray_parameter));
    (reflected_distance > 0.0).then_some((reflection_point, reflected_distance, image_source))
}

/// Tests an open path against a finite 2D segment, including wall endpoints.
fn path_intersects_geometry(
    path_start: Vector2,
    path_end: Vector2,
    geometry: &SegmentGeometry,
) -> bool {
    if !geometry.bounds.overlaps(Bounds::path_2d(
        (path_start.x, path_start.y),
        (path_end.x, path_end.y),
    )) {
        return false;
    }
    let wall_start = geometry.start;
    let wall_end = geometry.end;
    let path = path_end.subtract(path_start);
    let wall = geometry.edge;
    let path_length = path.length();
    let wall_length = geometry.length;
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
#[path = "../tests/unit/scene2d.rs"]
mod tests;
