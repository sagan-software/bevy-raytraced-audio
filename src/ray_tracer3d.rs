//! Listener-centred stochastic ray tracing over 3D triangles.

use crate::bvh::Ray;
use crate::math3d::Vector3;
use crate::ray_trace::{EchoStatistics, SourceAccumulator, TraceRng, narrow};
use crate::scene3d::TriangleGeometry;
use crate::{
    AcousticScene3d, Emitter3d, Listener3d, RayKind, RayTraceSettings, ReverbEstimate,
    SolverPoint3d, SourceRayResponse,
};
use core::f64::consts::TAU;

/// Smallest accepted hit distance along a ray, in meters, so a bounce cannot re-hit its origin.
const MINIMUM_HIT_DISTANCE_M: f64 = 1.0e-7;

/// Tolerance for unitless barycentric coordinates at triangle edges.
const TRIANGLE_PARAMETER_EPSILON: f64 = 1.0e-9;

/// Relative tolerance used to classify a ray as parallel to a triangle's plane.
const PARALLEL_EPSILON: f64 = 1.0e-12;

/// Distance a permeation ray starts in front of its bounce point so the hit triangle is crossed.
const PERMEATION_BACKOFF_M: f64 = 1.0e-4;

/// Mid-band energy below which a primary ray stops bouncing.
const MINIMUM_RAY_ENERGY: f64 = 1.0e-3;

/// Golden angle in radians, `π (3 - √5)`, the azimuth step of a Fibonacci sphere.
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653;

/// One recorded ray segment for visualization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaySegment3d {
    /// Segment start in meters.
    start: SolverPoint3d,
    /// Segment end in meters.
    end: SolverPoint3d,
    /// Role of the segment in the trace.
    kind: RayKind,
    /// Total path length from the listener to the segment start, in meters.
    start_distance_m: f64,
    /// Mid-band ray energy carried along the segment, in `[0, 1]`.
    energy: f32,
    /// Source index for occlusion and permeation segments.
    source_index: Option<usize>,
}

impl RaySegment3d {
    /// Returns the segment start in meters.
    #[must_use]
    pub const fn start(self) -> SolverPoint3d {
        self.start
    }

    /// Returns the segment end in meters.
    #[must_use]
    pub const fn end(self) -> SolverPoint3d {
        self.end
    }

    /// Returns the segment's role in the trace.
    #[must_use]
    pub const fn kind(self) -> RayKind {
        self.kind
    }

    /// Returns the path length from the listener to the segment start, in meters.
    ///
    /// Dividing by a visualization speed gives the time at which the segment should start drawing.
    #[must_use]
    pub const fn start_distance_m(self) -> f64 {
        self.start_distance_m
    }

    /// Returns the segment length in meters.
    #[must_use]
    pub fn length_m(self) -> f64 {
        (self.end.x_m() - self.start.x_m())
            .hypot(self.end.y_m() - self.start.y_m())
            .hypot(self.end.z_m() - self.start.z_m())
    }

    /// Returns the mid-band ray energy along the segment.
    #[must_use]
    pub const fn energy(self) -> f32 {
        self.energy
    }

    /// Returns the source index for occlusion and permeation segments.
    #[must_use]
    pub const fn source_index(self) -> Option<usize> {
        self.source_index
    }
}

/// Results of one listener ray trace, with reusable storage.
#[derive(Clone, Debug, Default)]
pub struct ListenerTrace3d {
    /// Per-source responses in the order sources were passed.
    sources: Vec<SourceRayResponse>,
    /// Listener reverb estimate.
    reverb: ReverbEstimate,
    /// Unit direction from the listener toward outdoor ambience, if any ray escaped visibly.
    ambient_direction: Option<SolverPoint3d>,
    /// Agreement of escaped-ray directions, from `0` (spread out) to `1` (one direction).
    ambient_focus: f32,
    /// Recorded segments when the settings request them.
    segments: Vec<RaySegment3d>,
    /// Scratch per-source accumulators.
    accumulators: Vec<SourceAccumulator>,
    /// Scratch per-source index of the last primary ray that discovered the source.
    discovered_by_ray: Vec<u32>,
}

impl ListenerTrace3d {
    /// Returns one response per traced source, in input order.
    #[must_use]
    pub fn sources(&self) -> &[SourceRayResponse] {
        &self.sources
    }

    /// Returns the listener's reverb estimate.
    #[must_use]
    pub const fn reverb(&self) -> ReverbEstimate {
        self.reverb
    }

    /// Returns the unit direction from the listener toward outdoor ambience.
    #[must_use]
    pub const fn ambient_direction(&self) -> Option<SolverPoint3d> {
        self.ambient_direction
    }

    /// Returns how strongly escaped rays agree on one ambience direction, in `[0, 1]`.
    #[must_use]
    pub const fn ambient_focus(&self) -> f32 {
        self.ambient_focus
    }

    /// Returns recorded ray segments; empty unless recording was enabled.
    #[must_use]
    pub fn segments(&self) -> &[RaySegment3d] {
        &self.segments
    }

    /// Clears results while keeping allocated storage.
    fn reset(&mut self, source_count: usize) {
        self.sources.clear();
        self.segments.clear();
        self.accumulators.clear();
        self.accumulators
            .resize(source_count, SourceAccumulator::default());
        self.discovered_by_ray.clear();
        self.discovered_by_ray.resize(source_count, u32::MAX);
        self.reverb = ReverbEstimate::DRY;
        self.ambient_direction = None;
        self.ambient_focus = 0.0;
    }

    /// Records one segment when recording is enabled.
    fn record(
        &mut self,
        enabled: bool,
        (start, end): (Vector3, Vector3),
        kind: RayKind,
        start_distance_m: f64,
        energy: f64,
        source_index: Option<usize>,
    ) {
        if enabled {
            self.segments.push(RaySegment3d {
                start: SolverPoint3d::new(start.x, start.y, start.z),
                end: SolverPoint3d::new(end.x, end.y, end.z),
                kind,
                start_distance_m,
                energy: narrow(energy.clamp(0.0, 1.0)),
                source_index,
            });
        }
    }
}

/// The nearest surface hit along a ray.
#[derive(Clone, Copy, Debug)]
struct Hit {
    /// Distance from the ray origin in meters.
    distance_m: f64,
    /// Index of the hit triangle.
    triangle_index: usize,
    /// Unit surface normal facing back toward the incoming ray.
    normal: Vector3,
}

/// Per-band energy carried by one primary ray.
#[derive(Clone, Copy, Debug)]
struct RayEnergy {
    /// Low-band energy fraction.
    low: f64,
    /// Mid-band energy fraction.
    mid: f64,
    /// High-band energy fraction.
    high: f64,
}

impl AcousticScene3d {
    /// Fires rays from the listener, follows their bounces, and estimates muffling and reverb.
    ///
    /// Results are written into `output`, reusing its storage. Source responses follow the order
    /// of `sources`. See [`crate::RayTraceSettings`] for the ray model.
    pub fn trace_listener(
        &self,
        listener: Listener3d,
        sources: &[Emitter3d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace3d,
    ) {
        output.reset(sources.len());
        let record = settings.records_rays();
        let origin = Vector3::from_point(listener.position());
        let source_points: Vec<Vector3> = sources
            .iter()
            .map(|source| Vector3::from_point(source.position()))
            .collect();

        // Straight lines from the listener decide direct visibility and seed permeation.
        for (index, source) in source_points.iter().copied().enumerate() {
            let (gain, occluded) = self.direct_transmission(origin, source);
            if let Some(accumulator) = output.accumulators.get_mut(index) {
                accumulator.direct_visible = !occluded;
                accumulator.add_permeation(gain, 1.0);
            }
            if occluded {
                output.record(
                    record,
                    (origin, source),
                    RayKind::Permeation,
                    0.0,
                    f64::from(gain.mid()),
                    Some(index),
                );
            }
        }

        let ray_count = settings.ray_count();
        let permeation_stride = ray_count
            .checked_div(settings.permeation_ray_count())
            .map_or(u32::MAX, |stride| stride.max(1));
        let mut rng = TraceRng::new(settings.seed());
        let spiral = FibonacciSphere::new(ray_count, &mut rng);
        let mut statistics = EchoStatistics::default();
        let mut ambient_sum = Vector3::default();
        let mut ambient_weight = 0.0_f64;

        for ray_index in 0..ray_count {
            let casts_permeation =
                settings.permeation_ray_count() > 0 && ray_index % permeation_stride == 0;
            let ambient = self.follow_ray(
                PrimaryRay {
                    index: ray_index,
                    origin,
                    direction: spiral.direction(ray_index),
                    casts_permeation,
                },
                &source_points,
                settings,
                &mut rng,
                &mut statistics,
                output,
            );
            if let Some(direction) = ambient {
                ambient_sum = ambient_sum.add(direction);
                ambient_weight += direction.length();
            }
        }

        output.reverb = ReverbEstimate::from_statistics(&statistics, ray_count);
        let ambient_length = ambient_sum.length();
        if ambient_weight > 0.0 && ambient_length > 0.0 {
            let unit = ambient_sum.scale(ambient_length.recip());
            output.ambient_direction = Some(SolverPoint3d::new(unit.x, unit.y, unit.z));
            output.ambient_focus = narrow((ambient_length / ambient_weight).clamp(0.0, 1.0));
        }
        let accumulators = core::mem::take(&mut output.accumulators);
        output.sources.extend(
            accumulators
                .iter()
                .map(|accumulator| accumulator.finish(ray_count)),
        );
        output.accumulators = accumulators;
    }

    /// Follows one primary ray and returns its ambience direction if it escaped.
    fn follow_ray(
        &self,
        ray: PrimaryRay,
        sources: &[Vector3],
        settings: RayTraceSettings,
        rng: &mut TraceRng,
        statistics: &mut EchoStatistics,
        output: &mut ListenerTrace3d,
    ) -> Option<Vector3> {
        let record = settings.records_rays();
        let listener = ray.origin;
        let mut position = ray.origin;
        let mut direction = ray.direction;
        let mut travelled_m = 0.0;
        let mut energy = RayEnergy {
            low: 1.0,
            mid: 1.0,
            high: 1.0,
        };
        let mut skipped_triangle = None;
        let mut last_echo_point: Option<Vector3> = None;

        for bounce in 0..settings.max_bounces() {
            let Some(hit) = self.closest_hit(
                position,
                direction,
                settings.escape_distance_m(),
                skipped_triangle,
            ) else {
                return Self::escape(
                    (listener, position, direction),
                    (bounce, travelled_m, energy.mid),
                    last_echo_point,
                    settings,
                    statistics,
                    output,
                );
            };

            let hit_point = position.add(direction.scale(hit.distance_m));
            output.record(
                record,
                (position, hit_point),
                RayKind::Primary,
                travelled_m,
                energy.mid,
                None,
            );
            travelled_m += hit.distance_m;
            let triangle = self.triangles().get(hit.triangle_index)?;
            let reflected = triangle.material().reflected_fraction();
            energy.low *= reflected.low();
            energy.mid *= reflected.mid();
            energy.high *= reflected.high();
            statistics.bounce_count += 1;
            statistics.bounce_leg_distance_m += hit.distance_m;
            statistics.reflected_energy_sum += reflected.mid();
            statistics.reflected_low_sum += reflected.low();
            statistics.reflected_high_sum += reflected.high();

            // Echo: the bounce point must face the listener and see it without obstruction.
            let faces_listener = listener.subtract(hit_point).dot(hit.normal) > 0.0;
            if faces_listener
                && !self.segment_is_blocked(hit_point, listener, Some(hit.triangle_index))
            {
                let return_m = travelled_m + listener.subtract(hit_point).length();
                statistics.echo_count += 1;
                statistics.echo_energy += energy.mid;
                statistics.echo_weighted_distance_m = energy
                    .mid
                    .mul_add(return_m, statistics.echo_weighted_distance_m);
                if bounce == 0 {
                    statistics.first_echo_energy += energy.mid;
                    statistics.first_echo_weighted_distance_m = energy
                        .mid
                        .mul_add(return_m, statistics.first_echo_weighted_distance_m);
                }
                last_echo_point = Some(hit_point);
                output.record(
                    record,
                    (hit_point, listener),
                    RayKind::Echo,
                    travelled_m,
                    energy.mid,
                    None,
                );
            }

            self.discover_sources(
                (ray.index, hit_point, hit, travelled_m, energy.mid),
                sources,
                record,
                output,
            );

            if bounce == 0 && ray.casts_permeation {
                let start = hit_point.subtract(direction.scale(PERMEATION_BACKOFF_M));
                for (index, source) in sources.iter().copied().enumerate() {
                    let direct_visible = output
                        .accumulators
                        .get(index)
                        .is_some_and(|accumulator| accumulator.direct_visible);
                    if direct_visible {
                        continue;
                    }
                    let (gain, _) = self.direct_transmission(start, source);
                    if let Some(accumulator) = output.accumulators.get_mut(index) {
                        accumulator.add_permeation(gain, 1.0);
                    }
                    output.record(
                        record,
                        (hit_point, source),
                        RayKind::Permeation,
                        travelled_m,
                        f64::from(gain.mid()),
                        Some(index),
                    );
                }
            }

            if energy.low.max(energy.mid).max(energy.high) < MINIMUM_RAY_ENERGY {
                return None;
            }
            direction = scatter(
                reflect(direction, hit.normal),
                hit.normal,
                triangle
                    .material()
                    .scattering()
                    .unwrap_or_else(|| settings.scattering()),
                rng,
            );
            position = hit_point;
            skipped_triangle = Some(hit.triangle_index);
        }
        None
    }

    /// Records an escaped leg and returns its ambience direction scaled by remaining energy.
    fn escape(
        (listener, position, direction): (Vector3, Vector3, Vector3),
        (bounce, travelled_m, energy): (u32, f64, f64),
        last_echo_point: Option<Vector3>,
        settings: RayTraceSettings,
        statistics: &mut EchoStatistics,
        output: &mut ListenerTrace3d,
    ) -> Option<Vector3> {
        let record = settings.records_rays();
        statistics.escaped_rays += 1;
        let end = position.add(direction.scale(settings.escape_distance_m()));
        output.record(
            record,
            (position, end),
            RayKind::Escaped,
            travelled_m,
            energy,
            None,
        );

        // The last bounce point that could see the listener is where outdoor sound enters.
        let arrival = match last_echo_point {
            Some(point) => point.subtract(listener),
            None if bounce == 0 => direction,
            None => return None,
        };
        let length = arrival.length();
        if length <= 0.0 {
            return None;
        }
        let unit = arrival.scale(length.recip());
        let shown_end = last_echo_point.unwrap_or_else(|| listener.add(unit.scale(2.0)));
        output.record(
            record,
            (listener, shown_end),
            RayKind::Ambient,
            0.0,
            energy,
            None,
        );
        // Rain heard through many absorbing bounces arrives weaker than rain from an opening.
        Some(unit.scale(energy.max(MINIMUM_RAY_ENERGY)))
    }

    /// Marks sources visible from a bounce point that this primary ray has not yet discovered.
    fn discover_sources(
        &self,
        (ray_index, hit_point, hit, travelled_m, energy): (u32, Vector3, Hit, f64, f64),
        sources: &[Vector3],
        record: bool,
        output: &mut ListenerTrace3d,
    ) {
        for (index, source) in sources.iter().copied().enumerate() {
            let already_discovered = output
                .discovered_by_ray
                .get(index)
                .is_none_or(|last_ray| *last_ray == ray_index);
            let faces_source = source.subtract(hit_point).dot(hit.normal) > 0.0;
            if already_discovered
                || !faces_source
                || self.segment_is_blocked(hit_point, source, Some(hit.triangle_index))
            {
                continue;
            }
            if let Some(last_ray) = output.discovered_by_ray.get_mut(index) {
                *last_ray = ray_index;
            }
            if let Some(accumulator) = output.accumulators.get_mut(index) {
                accumulator.discovered_energy += energy;
            }
            output.record(
                record,
                (hit_point, source),
                RayKind::Occlusion,
                travelled_m,
                energy,
                Some(index),
            );
        }
    }

    /// Finds the closest triangle hit along a ray within the escape distance.
    fn closest_hit(
        &self,
        origin: Vector3,
        direction: Vector3,
        maximum_distance_m: f64,
        skipped_triangle: Option<usize>,
    ) -> Option<Hit> {
        let acceleration = self.acceleration();
        let mut best: Option<(f64, usize)> = None;
        acceleration.hierarchy.visit_ray(
            Ray::new(
                (origin.x, origin.y, origin.z),
                (direction.x, direction.y, direction.z),
            ),
            maximum_distance_m,
            |index| {
                if skipped_triangle == Some(index) {
                    return None;
                }
                let triangle = acceleration.triangles.get(index)?;
                let distance = ray_triangle_distance(origin, direction, triangle)?;
                if distance > maximum_distance_m
                    || best.is_some_and(|(current, _)| current <= distance)
                {
                    return None;
                }
                best = Some((distance, index));
                Some(distance)
            },
        );
        // Only the winning triangle needs a normal, so it is built after traversal.
        let (distance_m, triangle_index) = best?;
        let triangle = acceleration.triangles.get(triangle_index)?;
        let mut normal = triangle.normal.scale(triangle.normal_length.recip());
        if normal.dot(direction) > 0.0 {
            normal = normal.scale(-1.0);
        }
        Some(Hit {
            distance_m,
            triangle_index,
            normal,
        })
    }

    /// Checks whether a triangle other than the skipped one crosses the open segment.
    ///
    /// Unlike the scene's bounds-overlap query, ray traversal only visits nodes the segment
    /// actually passes through and stops descending after the first blocker, which matters
    /// because a bounce point casts one of these checks per source and one toward the listener.
    fn segment_is_blocked(
        &self,
        start: Vector3,
        end: Vector3,
        skipped_triangle: Option<usize>,
    ) -> bool {
        let offset = end.subtract(start);
        let length = offset.length();
        // Endpoints never block, so the segment interior stops just short of its end.
        let limit = length - MINIMUM_HIT_DISTANCE_M;
        if limit <= MINIMUM_HIT_DISTANCE_M {
            return false;
        }
        let direction = offset.scale(length.recip());
        let acceleration = self.acceleration();
        let mut blocked = false;
        acceleration.hierarchy.visit_ray(
            Ray::new(
                (start.x, start.y, start.z),
                (direction.x, direction.y, direction.z),
            ),
            limit,
            |index| {
                if blocked || skipped_triangle == Some(index) {
                    return None;
                }
                let triangle = acceleration.triangles.get(index)?;
                let distance = ray_triangle_distance(start, direction, triangle)?;
                // Reporting a zero hit parameter prunes every node ahead of the segment start.
                blocked = distance < limit;
                blocked.then_some(0.0)
            },
        );
        blocked
    }
}

/// Inputs that identify one primary ray.
#[derive(Clone, Copy, Debug)]
struct PrimaryRay {
    /// Primary ray index within the trace.
    index: u32,
    /// Listener position where the ray starts.
    origin: Vector3,
    /// Initial unit direction.
    direction: Vector3,
    /// Whether the first bounce point casts permeation rays.
    casts_permeation: bool,
}

/// A randomly rotated golden-angle spiral of nearly evenly spaced unit directions.
#[derive(Clone, Copy, Debug)]
struct FibonacciSphere {
    /// Number of directions on the spiral.
    count: f64,
    /// Random azimuth added to every direction, in radians.
    azimuth_offset: f64,
    /// Random fraction of one height band added to every direction, in `[0, 1)`.
    height_offset: f64,
}

impl FibonacciSphere {
    /// Draws the spiral's random rotation for `count` directions.
    fn new(count: u32, rng: &mut TraceRng) -> Self {
        Self {
            count: f64::from(count.max(1)),
            azimuth_offset: rng.next_unit() * TAU,
            height_offset: rng.next_unit(),
        }
    }

    /// Returns the unit direction at one spiral index.
    fn direction(self, index: u32) -> Vector3 {
        // Equal-height bands have equal area on a sphere, so one jittered height per band is
        // uniform in `z`, and the golden-angle azimuth spreads neighboring bands apart.
        let z = (-2.0 * (f64::from(index) + self.height_offset)).mul_add(self.count.recip(), 1.0);
        let radius = z.mul_add(-z, 1.0).max(0.0).sqrt();
        let azimuth = GOLDEN_ANGLE.mul_add(f64::from(index), self.azimuth_offset);
        Vector3 {
            x: radius * azimuth.cos(),
            y: radius * azimuth.sin(),
            z,
        }
    }
}

/// Returns the distance along a unit ray to a finite triangle, if the ray hits it.
///
/// Uses the Möller–Trumbore barycentric solve with the scene's edge tolerance.
fn ray_triangle_distance(
    origin: Vector3,
    direction: Vector3,
    triangle: &TriangleGeometry,
) -> Option<f64> {
    let determinant_vector = unfused_cross(direction, triangle.edge_b);
    let determinant = unfused_dot(triangle.edge_a, determinant_vector);
    // The unit direction contributes no length to the scale of this parallel test.
    if determinant.abs() <= PARALLEL_EPSILON * triangle.edge_a_length * triangle.edge_b_length {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let origin_delta = origin.subtract(triangle.origin);
    let coordinate_a = unfused_dot(origin_delta, determinant_vector) * inverse_determinant;
    if !(-TRIANGLE_PARAMETER_EPSILON..=1.0 + TRIANGLE_PARAMETER_EPSILON).contains(&coordinate_a) {
        return None;
    }
    let cross_vector = unfused_cross(origin_delta, triangle.edge_a);
    let coordinate_b = unfused_dot(direction, cross_vector) * inverse_determinant;
    if coordinate_b < -TRIANGLE_PARAMETER_EPSILON
        || coordinate_a + coordinate_b > 1.0 + TRIANGLE_PARAMETER_EPSILON
    {
        return None;
    }
    let distance = unfused_dot(triangle.edge_b, cross_vector) * inverse_determinant;
    (distance > MINIMUM_HIT_DISTANCE_M).then_some(distance)
}

/// Returns the dot product using separate multiplies and adds.
///
/// Without FMA target features, `f64::mul_add` lowers to a runtime-dispatched library call. The
/// ray–triangle test runs for every BVH candidate of every bounce, where those calls tripled its
/// cost; its tolerances do not need fused rounding.
#[expect(
    clippy::suboptimal_flops,
    reason = "mul_add is a library call on targets without FMA, and this is the hottest loop"
)]
fn unfused_dot(left: Vector3, right: Vector3) -> f64 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

/// Returns the cross product using separate multiplies and subtractions.
///
/// See [`unfused_dot`] for why the ray–triangle test avoids `f64::mul_add`.
#[expect(
    clippy::suboptimal_flops,
    reason = "mul_add is a library call on targets without FMA, and this is the hottest loop"
)]
fn unfused_cross(left: Vector3, right: Vector3) -> Vector3 {
    Vector3 {
        x: left.y * right.z - left.z * right.y,
        y: left.z * right.x - left.x * right.z,
        z: left.x * right.y - left.y * right.x,
    }
}

/// Mirrors a unit direction about a unit surface normal.
fn reflect(direction: Vector3, normal: Vector3) -> Vector3 {
    direction.subtract(normal.scale(2.0 * direction.dot(normal)))
}

/// Returns two unit tangents that complete a right-handed orthonormal basis with a unit normal.
///
/// The branchless construction of Duff et al. (2017) stays stable for every normal direction.
fn tangent_basis(normal: Vector3) -> (Vector3, Vector3) {
    let sign = 1.0_f64.copysign(normal.z);
    let a = -(sign + normal.z).recip();
    let b = normal.x * normal.y * a;
    let first = Vector3 {
        x: (sign * normal.x * normal.x).mul_add(a, 1.0),
        y: sign * b,
        z: -sign * normal.x,
    };
    let second = Vector3 {
        x: b,
        y: (normal.y * normal.y).mul_add(a, sign),
        z: -normal.y,
    };
    (first, second)
}

/// Blends a specular direction toward a cosine-weighted random direction around the normal.
fn scatter(specular: Vector3, normal: Vector3, scattering: f32, rng: &mut TraceRng) -> Vector3 {
    if scattering <= 0.0 {
        return specular;
    }
    // Uniform points on a unit disk projected up onto the hemisphere are cosine-weighted.
    let azimuth = rng.next_unit() * TAU;
    let sine_squared = rng.next_unit();
    let sine = sine_squared.sqrt();
    let cosine = (1.0 - sine_squared).max(0.0).sqrt();
    let (first, second) = tangent_basis(normal);
    let diffuse = normal
        .scale(cosine)
        .add(first.scale(sine * azimuth.cos()))
        .add(second.scale(sine * azimuth.sin()));
    let blend = f64::from(scattering);
    let mixed = specular.scale(1.0 - blend).add(diffuse.scale(blend));
    let length = mixed.length();
    if length <= f64::EPSILON {
        normal
    } else {
        mixed.scale(length.recip())
    }
}

#[cfg(test)]
mod tests {
    //! Behavior checks for the 3D listener ray tracer against small reference rooms.

    use super::{FibonacciSphere, ListenerTrace3d, tangent_basis};
    use crate::math3d::Vector3;
    use crate::ray_trace::TraceRng;
    use crate::{
        AcousticMaterial, AcousticScene3d, BandAbsorption, BandGain, Emitter3d, Listener3d, Point3,
        RayKind, RayTraceSettings, Triangle3d,
    };

    /// Makes a finite point.
    fn point(x: f32, y: f32, z: f32) -> Point3 {
        Point3::try_new(x, y, z).expect("finite test point")
    }

    /// Adds one planar quad as two triangles sharing the `a`–`c` diagonal.
    fn quad(
        scene: &mut AcousticScene3d,
        corners: [(f32, f32, f32); 4],
        material: AcousticMaterial,
    ) {
        let [a, b, c, d] = corners.map(|(x, y, z)| point(x, y, z));
        for vertices in [[a, b, c], [a, c, d]] {
            scene
                .add_triangle(Triangle3d::try_new(vertices, material).expect("nondegenerate quad"));
        }
    }

    /// Lightly absorbing, opaque walls.
    fn plaster() -> AcousticMaterial {
        AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.1, 0.15).expect("valid absorption"))
    }

    /// Half-transmissive walls that pass bass more readily than treble.
    fn leaky() -> AcousticMaterial {
        AcousticMaterial::default()
            .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
            .expect("valid material")
    }

    /// Builds an axis-aligned box room centred on the origin with `z` up.
    ///
    /// A doorway of half-width `gap` leaves a hole in the east wall from the floor to `z = gap`,
    /// framed by two side quads and a lintel above it.
    fn box_room((hx, hy, hz): (f32, f32, f32), doorway: Option<f32>) -> AcousticScene3d {
        let mut scene = AcousticScene3d::default();
        let material = plaster();
        quad(
            &mut scene,
            [
                (-hx, -hy, -hz),
                (hx, -hy, -hz),
                (hx, hy, -hz),
                (-hx, hy, -hz),
            ],
            material,
        );
        quad(
            &mut scene,
            [(-hx, -hy, hz), (hx, -hy, hz), (hx, hy, hz), (-hx, hy, hz)],
            material,
        );
        quad(
            &mut scene,
            [
                (-hx, -hy, -hz),
                (hx, -hy, -hz),
                (hx, -hy, hz),
                (-hx, -hy, hz),
            ],
            material,
        );
        quad(
            &mut scene,
            [(-hx, hy, -hz), (hx, hy, -hz), (hx, hy, hz), (-hx, hy, hz)],
            material,
        );
        quad(
            &mut scene,
            [
                (-hx, -hy, -hz),
                (-hx, hy, -hz),
                (-hx, hy, hz),
                (-hx, -hy, hz),
            ],
            material,
        );
        match doorway {
            Some(gap) => {
                quad(
                    &mut scene,
                    [
                        (hx, -hy, -hz),
                        (hx, -gap, -hz),
                        (hx, -gap, hz),
                        (hx, -hy, hz),
                    ],
                    material,
                );
                quad(
                    &mut scene,
                    [(hx, gap, -hz), (hx, hy, -hz), (hx, hy, hz), (hx, gap, hz)],
                    material,
                );
                quad(
                    &mut scene,
                    [
                        (hx, -gap, gap),
                        (hx, gap, gap),
                        (hx, gap, hz),
                        (hx, -gap, hz),
                    ],
                    material,
                );
            }
            None => {
                quad(
                    &mut scene,
                    [(hx, -hy, -hz), (hx, hy, -hz), (hx, hy, hz), (hx, -hy, hz)],
                    material,
                );
            }
        }
        scene
    }

    /// Traces one listener and sources with default settings and recording enabled.
    fn trace(
        scene: &AcousticScene3d,
        listener: (f32, f32, f32),
        sources: &[(f32, f32, f32)],
    ) -> ListenerTrace3d {
        let sources: Vec<_> = sources
            .iter()
            .map(|(x, y, z)| Emitter3d::new(point(*x, *y, *z)))
            .collect();
        let mut output = ListenerTrace3d::default();
        scene.trace_listener(
            Listener3d::new(point(listener.0, listener.1, listener.2)),
            &sources,
            RayTraceSettings::default().with_recorded_rays(true),
            &mut output,
        );
        output
    }

    /// Spiral directions are unit length and balanced around the sphere.
    #[test]
    fn fibonacci_directions_cover_the_sphere() {
        let mut rng = TraceRng::new(3);
        let spiral = FibonacciSphere::new(256, &mut rng);
        let mut sum = Vector3::default();
        let mut second_moment = Vector3::default();
        for index in 0..256 {
            let direction = spiral.direction(index);
            assert!((direction.length() - 1.0).abs() < 1.0e-12);
            sum = sum.add(direction);
            second_moment = second_moment.add(Vector3 {
                x: direction.x.powi(2),
                y: direction.y.powi(2),
                z: direction.z.powi(2),
            });
        }
        assert!(sum.length() / 256.0 < 0.01, "{sum:?}");
        // A horizontal circle can have zero mean, but cannot pass this isotropy check.
        for moment in [second_moment.x, second_moment.y, second_moment.z] {
            assert!((moment / 256.0 - 1.0 / 3.0).abs() < 0.01);
        }
    }

    /// Tangent bases are orthonormal, including for normals pointing straight down.
    #[test]
    fn tangent_basis_is_orthonormal() {
        for normal in [
            Vector3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Vector3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            Vector3 {
                x: 0.6,
                y: 0.0,
                z: 0.8,
            },
            Vector3 {
                x: 0.0,
                y: -0.6,
                z: -0.8,
            },
        ] {
            let (first, second) = tangent_basis(normal);
            assert!((first.length() - 1.0).abs() < 1.0e-12);
            assert!((second.length() - 1.0).abs() < 1.0e-12);
            assert!(first.dot(normal).abs() < 1.0e-12);
            assert!(second.dot(normal).abs() < 1.0e-12);
            assert!(first.dot(second).abs() < 1.0e-12);
        }
    }

    /// A sealed room hides an outside source completely and keeps all rays inside.
    #[test]
    fn sealed_room_fully_muffles_outside_source() {
        let scene = box_room((4.0, 4.0, 2.0), None);
        let output = trace(&scene, (0.0, 0.0, 0.0), &[(10.0, 0.0, 0.0)]);
        let source = output.sources()[0];
        assert!(!source.is_direct_visible());
        assert!(source.discovered_fraction() < f32::EPSILON);
        assert!(source.filter().gain_hf() < 1.0e-6);
        let reverb = output.reverb();
        assert!(reverb.outdoor_fraction() < f32::EPSILON);
        assert!(reverb.return_fraction() > 0.5, "{reverb:?}");
        assert!(reverb.decay_time_s() > 0.2, "{reverb:?}");
        assert!(reverb.wet_gain() > 0.5, "{reverb:?}");
        assert!(output.ambient_direction().is_none());
    }

    /// Fully absorbing room surfaces leave no energy for audible reverberation.
    #[test]
    fn absorbing_room_has_no_reverb_send() {
        let reflective = box_room((4.0, 4.0, 2.0), None);
        let material = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0).unwrap());
        let mut absorbing = AcousticScene3d::default();
        for surface in reflective.triangles() {
            absorbing.add_triangle(Triangle3d::try_new(surface.vertices(), material).unwrap());
        }
        let dry = trace(&absorbing, (0.0, 0.0, 0.0), &[]).reverb();
        let wet = trace(&reflective, (0.0, 0.0, 0.0), &[]).reverb();
        assert!(dry.wet_gain() < f32::EPSILON, "{dry:?}");
        assert!(wet.wet_gain() > 0.5, "{wet:?}");
    }

    /// With no geometry, every ray escapes and nothing reverberates.
    #[test]
    fn open_field_is_outdoor_and_dry() {
        let scene = AcousticScene3d::default();
        let output = trace(&scene, (0.0, 0.0, 0.0), &[(3.0, 0.0, 0.0)]);
        assert!(output.sources()[0].is_direct_visible());
        assert!((output.sources()[0].clarity() - 1.0).abs() < f32::EPSILON);
        let reverb = output.reverb();
        assert!((reverb.outdoor_fraction() - 1.0).abs() < f32::EPSILON);
        assert!(reverb.wet_gain() < f32::EPSILON);
        assert!(output.ambient_focus() < 0.05, "uniform escapes cancel out");
    }

    /// A source in the next room through a doorway is partly discovered by bounces.
    #[test]
    fn doorway_partially_muffles_hidden_source() {
        let mut scene = box_room((4.0, 4.0, 2.0), Some(1.0));
        // A second wall outside the doorway hides the source from a straight line.
        quad(
            &mut scene,
            [
                (6.0, -3.0, -2.0),
                (6.0, 3.0, -2.0),
                (6.0, 3.0, 2.0),
                (6.0, -3.0, 2.0),
            ],
            plaster(),
        );
        let output = trace(&scene, (-2.0, 2.0, 0.0), &[(8.0, 0.0, 0.0)]);
        let source = output.sources()[0];
        assert!(!source.is_direct_visible());
        let sealed = trace(
            &box_room((4.0, 4.0, 2.0), None),
            (-2.0, 2.0, 0.0),
            &[(8.0, 0.0, 0.0)],
        )
        .sources()[0];
        assert!(source.clarity() >= sealed.clarity());
        let around_corner = trace(
            &box_room((4.0, 4.0, 2.0), Some(1.0)),
            (-2.0, 2.0, 0.0),
            &[(5.0, 3.0, 0.0)],
        )
        .sources()[0];
        assert!(!around_corner.is_direct_visible());
        assert!(
            around_corner.discovered_fraction() > 0.0,
            "{around_corner:?}"
        );
        assert!(around_corner.clarity() > 0.0 && around_corner.clarity() < 1.0);
        assert!(around_corner.filter().gain_hf() < around_corner.filter().gain_lf());
    }

    /// Escaping rays through an east doorway point the ambience east.
    #[test]
    fn ambience_arrives_from_the_doorway() {
        let scene = box_room((4.0, 4.0, 2.0), Some(1.5));
        let output = trace(&scene, (-1.0, 0.0, 0.0), &[]);
        let direction = output
            .ambient_direction()
            .expect("rays escape through the doorway");
        let length = direction
            .x_m()
            .hypot(direction.y_m())
            .hypot(direction.z_m());
        assert!((length - 1.0).abs() < 1.0e-9, "{direction:?}");
        assert!(direction.x_m() > 0.8, "{direction:?}");
        let reverb = output.reverb();
        assert!(reverb.outdoor_fraction() > 0.0 && reverb.outdoor_fraction() < 1.0);
    }

    /// Recording emits primary, echo, occlusion, permeation, and escape segments.
    #[test]
    fn recording_contains_every_ray_kind() {
        let mut scene = box_room((4.0, 4.0, 2.0), Some(1.0));
        quad(
            &mut scene,
            [
                (0.0, -4.0, -2.0),
                (0.0, 0.5, -2.0),
                (0.0, 0.5, 2.0),
                (0.0, -4.0, 2.0),
            ],
            leaky(),
        );
        let output = trace(&scene, (-2.0, -2.0, 0.0), &[(2.0, -2.0, 0.0)]);
        for kind in [
            RayKind::Primary,
            RayKind::Echo,
            RayKind::Occlusion,
            RayKind::Permeation,
            RayKind::Escaped,
            RayKind::Ambient,
        ] {
            assert!(
                output
                    .segments()
                    .iter()
                    .any(|segment| segment.kind() == kind),
                "missing {kind:?}"
            );
        }
        let primary_distances_increase = output
            .segments()
            .windows(2)
            .filter(|pair| pair[0].kind() == RayKind::Primary && pair[1].kind() == RayKind::Primary)
            .all(|pair| {
                pair[1].start_distance_m() == 0.0
                    || pair[1].start_distance_m() >= pair[0].start_distance_m()
            });
        assert!(primary_distances_increase);
        assert!(
            output
                .segments()
                .iter()
                .all(|segment| segment.length_m() >= 0.0)
        );
    }

    /// Two transmitting walls muffle more than one.
    #[test]
    fn thicker_walls_transmit_less() {
        let mut thin = AcousticScene3d::default();
        quad(
            &mut thin,
            [
                (0.0, -20.0, -20.0),
                (0.0, 20.0, -20.0),
                (0.0, 20.0, 20.0),
                (0.0, -20.0, 20.0),
            ],
            leaky(),
        );
        let mut thick = thin.clone();
        quad(
            &mut thick,
            [
                (0.4, -20.0, -20.0),
                (0.4, 20.0, -20.0),
                (0.4, 20.0, 20.0),
                (0.4, -20.0, 20.0),
            ],
            leaky(),
        );
        let thin_response = trace(&thin, (-2.0, 0.0, 0.0), &[(2.0, 0.0, 0.0)]).sources()[0];
        let thick_response = trace(&thick, (-2.0, 0.0, 0.0), &[(2.0, 0.0, 0.0)]).sources()[0];
        assert!(thick_response.permeation().low() < thin_response.permeation().low());
        assert!(thick_response.filter().gain_lf() < thin_response.filter().gain_lf());
    }
    /// An isolated outdoor wall gives an early reflection without a sealed-room tail.
    #[test]
    fn outdoor_wall_retains_early_echoes() {
        let mut scene = AcousticScene3d::default();
        quad(
            &mut scene,
            [
                (-100.0, -100.0, -3.0),
                (100.0, -100.0, -3.0),
                (100.0, 100.0, -3.0),
                (-100.0, 100.0, -3.0),
            ],
            plaster(),
        );
        let reverb = trace(&scene, (0.0, 0.0, 0.0), &[]).reverb();
        assert!(reverb.outdoor_fraction() > 0.99, "{reverb:?}");
        assert!(reverb.wet_gain() < 1.0e-6, "{reverb:?}");
        assert!(reverb.early_gain() > 0.1, "{reverb:?}");
        assert!(reverb.reflections_delay_s() > 6.0 / 343.0, "{reverb:?}");
    }
}
