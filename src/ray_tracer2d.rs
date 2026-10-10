//! Listener-centred stochastic ray tracing over 2D segments.

use crate::bvh::Ray;
use crate::math2d::Vector2;
use crate::ray_trace::{EchoStatistics, SourceAccumulator, TraceRng, narrow};
use crate::{
    AcousticScene2d, Emitter2d, Listener2d, RayKind, RayTraceSettings, ReverbEstimate,
    SolverPoint2d, SourceRayResponse,
};
use core::f64::consts::TAU;

/// Smallest accepted hit distance along a ray, in meters, so a bounce cannot re-hit its origin.
const MINIMUM_HIT_DISTANCE_M: f64 = 1.0e-7;

/// Tolerance for unitless segment parameters at wall endpoints.
const WALL_PARAMETER_EPSILON: f64 = 1.0e-9;

/// Distance a permeation ray starts in front of its bounce point so the hit wall is crossed.
const PERMEATION_BACKOFF_M: f64 = 1.0e-4;

/// Mid-band energy below which a primary ray stops bouncing.
const MINIMUM_RAY_ENERGY: f64 = 1.0e-3;

/// One recorded ray segment for visualization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaySegment2d {
    /// Segment start in meters.
    start: SolverPoint2d,
    /// Segment end in meters.
    end: SolverPoint2d,
    /// Role of the segment in the trace.
    kind: RayKind,
    /// Total path length from the listener to the segment start, in meters.
    start_distance_m: f64,
    /// Mid-band ray energy carried along the segment, in `[0, 1]`.
    energy: f32,
    /// Source index for occlusion and permeation segments.
    source_index: Option<usize>,
}

impl RaySegment2d {
    /// Returns the segment start in meters.
    #[must_use]
    pub const fn start(self) -> SolverPoint2d {
        self.start
    }

    /// Returns the segment end in meters.
    #[must_use]
    pub const fn end(self) -> SolverPoint2d {
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
        (self.end.x_m() - self.start.x_m()).hypot(self.end.y_m() - self.start.y_m())
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
pub struct ListenerTrace2d {
    /// Per-source responses in the order sources were passed.
    sources: Vec<SourceRayResponse>,
    /// Listener reverb estimate.
    reverb: ReverbEstimate,
    /// Unit direction from the listener toward outdoor ambience, if any ray escaped visibly.
    ambient_direction: Option<SolverPoint2d>,
    /// Agreement of escaped-ray directions, from `0` (spread out) to `1` (one direction).
    ambient_focus: f32,
    /// Recorded segments when the settings request them.
    segments: Vec<RaySegment2d>,
    /// Scratch per-source accumulators.
    accumulators: Vec<SourceAccumulator>,
    /// Scratch per-source index of the last primary ray that discovered the source.
    discovered_by_ray: Vec<u32>,
    /// Reused solver-space source positions; retained across changing emitter counts.
    source_points: Vec<Vector2>,
}

impl ListenerTrace2d {
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
    pub const fn ambient_direction(&self) -> Option<SolverPoint2d> {
        self.ambient_direction
    }

    /// Returns how strongly escaped rays agree on one ambience direction, in `[0, 1]`.
    #[must_use]
    pub const fn ambient_focus(&self) -> f32 {
        self.ambient_focus
    }

    /// Returns recorded ray segments; empty unless recording was enabled.
    #[must_use]
    pub fn segments(&self) -> &[RaySegment2d] {
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
        (start, end): (Vector2, Vector2),
        kind: RayKind,
        start_distance_m: f64,
        energy: f64,
        source_index: Option<usize>,
    ) {
        if enabled {
            self.segments.push(RaySegment2d {
                start: SolverPoint2d::new(start.x, start.y),
                end: SolverPoint2d::new(end.x, end.y),
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
    /// Index of the hit segment.
    segment_index: usize,
    /// Unit surface normal facing back toward the incoming ray.
    normal: Vector2,
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

impl AcousticScene2d {
    /// Fires rays from the listener, follows their bounces, and estimates muffling and reverb.
    ///
    /// Results are written into `output`, reusing its storage. Source responses follow the order
    /// of `sources`. See [`crate::RayTraceSettings`] for the ray model.
    pub fn trace_listener(
        &self,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace2d,
    ) {
        output.reset(sources.len());
        let record = settings.records_rays();
        let origin = Vector2::from_point(listener.position());
        let mut source_points = core::mem::take(&mut output.source_points);
        source_points.clear();
        source_points.extend(
            sources
                .iter()
                .map(|source| Vector2::from_point(source.position())),
        );

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
        let angle_offset = rng.next_unit() * TAU / f64::from(ray_count);
        let mut statistics = EchoStatistics::default();
        let mut ambient_sum = Vector2::default();
        let mut ambient_weight = 0.0_f64;

        for ray_index in 0..ray_count {
            let angle = TAU.mul_add(f64::from(ray_index) / f64::from(ray_count), angle_offset);
            let initial_direction = Vector2 {
                x: angle.cos(),
                y: angle.sin(),
            };
            let casts_permeation =
                settings.permeation_ray_count() > 0 && ray_index % permeation_stride == 0;
            let ambient = self.follow_ray(
                PrimaryRay {
                    index: ray_index,
                    origin,
                    direction: initial_direction,
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
            output.ambient_direction = Some(SolverPoint2d::new(unit.x, unit.y));
            output.ambient_focus = narrow((ambient_length / ambient_weight).clamp(0.0, 1.0));
        }
        let accumulators = core::mem::take(&mut output.accumulators);
        output.sources.extend(
            accumulators
                .iter()
                .map(|accumulator| accumulator.finish(ray_count)),
        );
        output.accumulators = accumulators;
        output.source_points = source_points;
    }

    /// Follows one primary ray and returns its ambience direction if it escaped.
    fn follow_ray(
        &self,
        ray: PrimaryRay,
        sources: &[Vector2],
        settings: RayTraceSettings,
        rng: &mut TraceRng,
        statistics: &mut EchoStatistics,
        output: &mut ListenerTrace2d,
    ) -> Option<Vector2> {
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
        let mut skipped_segment = None;
        let mut last_echo_point: Option<Vector2> = None;

        for bounce in 0..settings.max_bounces() {
            let Some(hit) = self.closest_hit(
                position,
                direction,
                settings.escape_distance_m(),
                skipped_segment,
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
            let segment = self.segments().get(hit.segment_index)?;
            let material = segment.material();
            let reflected = material.reflected_fraction();
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
                && !self.segment_is_blocked(hit_point, listener, Some(hit.segment_index))
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
                segment
                    .material()
                    .scattering()
                    .unwrap_or_else(|| settings.scattering()),
                rng,
            );
            position = hit_point;
            skipped_segment = Some(hit.segment_index);
        }
        None
    }

    /// Records an escaped leg and returns its ambience direction scaled by remaining energy.
    fn escape(
        (listener, position, direction): (Vector2, Vector2, Vector2),
        (bounce, travelled_m, energy): (u32, f64, f64),
        last_echo_point: Option<Vector2>,
        settings: RayTraceSettings,
        statistics: &mut EchoStatistics,
        output: &mut ListenerTrace2d,
    ) -> Option<Vector2> {
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
        (ray_index, hit_point, hit, travelled_m, energy): (u32, Vector2, Hit, f64, f64),
        sources: &[Vector2],
        record: bool,
        output: &mut ListenerTrace2d,
    ) {
        for (index, source) in sources.iter().copied().enumerate() {
            let already_discovered = output
                .discovered_by_ray
                .get(index)
                .is_none_or(|last_ray| *last_ray == ray_index);
            let faces_source = source.subtract(hit_point).dot(hit.normal) > 0.0;
            if already_discovered
                || !faces_source
                || self.segment_is_blocked(hit_point, source, Some(hit.segment_index))
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

    /// Returns whether any segment other than `skipped_segment` crosses the open path interior.
    ///
    /// Ray traversal only visits nodes the path passes through and stops after the first
    /// blocker, which matters because every bounce casts one check per source and one toward
    /// the listener.
    fn segment_is_blocked(
        &self,
        start: Vector2,
        end: Vector2,
        skipped_segment: Option<usize>,
    ) -> bool {
        let offset = end.subtract(start);
        let length = offset.length();
        // Endpoints never block, so the path interior stops just short of its end.
        let limit = length - MINIMUM_HIT_DISTANCE_M;
        if limit <= MINIMUM_HIT_DISTANCE_M {
            return false;
        }
        let direction = offset.scale(length.recip());
        let mut blocked = false;
        self.acceleration().visit_ray(
            Ray::new((start.x, start.y, 0.0), (direction.x, direction.y, 0.0)),
            limit,
            |index| {
                if blocked || skipped_segment == Some(index) {
                    return None;
                }
                let segment = self.segments().get(index)?;
                let distance = ray_segment_distance(
                    start,
                    direction,
                    Vector2::from_point(segment.start()),
                    Vector2::from_point(segment.end()),
                )?;
                blocked = distance < limit;
                // A negative hit parameter prunes every remaining node.
                blocked.then_some(-1.0)
            },
        );
        blocked
    }

    /// Finds the closest segment hit along a ray within the escape distance.
    fn closest_hit(
        &self,
        origin: Vector2,
        direction: Vector2,
        maximum_distance_m: f64,
        skipped_segment: Option<usize>,
    ) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        self.acceleration().visit_ray(
            Ray::new((origin.x, origin.y, 0.0), (direction.x, direction.y, 0.0)),
            maximum_distance_m,
            |index| {
                if skipped_segment == Some(index) {
                    return None;
                }
                let segment = self.segments().get(index)?;
                let start = Vector2::from_point(segment.start());
                let end = Vector2::from_point(segment.end());
                let distance = ray_segment_distance(origin, direction, start, end)?;
                if distance > maximum_distance_m
                    || best.is_some_and(|current| current.distance_m <= distance)
                {
                    return None;
                }
                let wall = end.subtract(start);
                let mut normal = Vector2 {
                    x: -wall.y,
                    y: wall.x,
                };
                normal = normal.scale(normal.length().recip());
                if normal.dot(direction) > 0.0 {
                    normal = normal.scale(-1.0);
                }
                best = Some(Hit {
                    distance_m: distance,
                    segment_index: index,
                    normal,
                });
                Some(distance)
            },
        );
        best
    }
}

/// Inputs that identify one primary ray.
#[derive(Clone, Copy, Debug)]
struct PrimaryRay {
    /// Primary ray index within the trace.
    index: u32,
    /// Listener position where the ray starts.
    origin: Vector2,
    /// Initial unit direction.
    direction: Vector2,
    /// Whether the first bounce point casts permeation rays.
    casts_permeation: bool,
}

/// Returns the distance along a unit ray to a finite segment, if the ray hits it.
fn ray_segment_distance(
    origin: Vector2,
    direction: Vector2,
    start: Vector2,
    end: Vector2,
) -> Option<f64> {
    let wall = end.subtract(start);
    let denominator = cross(direction, wall);
    if denominator.abs() <= f64::EPSILON * (wall.x.abs() + wall.y.abs()) {
        return None;
    }
    let offset = start.subtract(origin);
    let distance = cross(offset, wall) / denominator;
    let wall_parameter = cross(offset, direction) / denominator;
    (distance > MINIMUM_HIT_DISTANCE_M
        && (-WALL_PARAMETER_EPSILON..=1.0 + WALL_PARAMETER_EPSILON).contains(&wall_parameter))
    .then_some(distance)
}

/// Returns the scalar 2D cross product with plain arithmetic.
///
/// The tracer's inner loops avoid `mul_add`, which compiles to a slow library call on targets
/// without fused multiply-add instructions, including WebAssembly.
#[expect(
    clippy::suboptimal_flops,
    reason = "unfused arithmetic is several times faster without hardware FMA"
)]
fn cross(first: Vector2, second: Vector2) -> f64 {
    first.x * second.y - first.y * second.x
}

/// Mirrors a unit direction about a unit surface normal.
fn reflect(direction: Vector2, normal: Vector2) -> Vector2 {
    direction.subtract(normal.scale(2.0 * direction.dot(normal)))
}

/// Blends a specular direction toward a cosine-weighted random direction around the normal.
fn scatter(specular: Vector2, normal: Vector2, scattering: f32, rng: &mut TraceRng) -> Vector2 {
    if scattering <= 0.0 {
        return specular;
    }
    // In 2D, a cosine-weighted (Lambertian) direction has sin(angle) uniform in [-1, 1].
    let sine = rng.next_unit().mul_add(2.0, -1.0);
    let cosine = sine.mul_add(-sine, 1.0).max(0.0).sqrt();
    let tangent = Vector2 {
        x: -normal.y,
        y: normal.x,
    };
    let diffuse = normal.scale(cosine).add(tangent.scale(sine));
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
    //! Behavior checks for the listener ray tracer against small reference rooms.

    use super::ListenerTrace2d;
    use crate::{
        AcousticMaterial, AcousticScene2d, BandAbsorption, BandGain, Emitter2d, Listener2d, Point2,
        RayKind, RayTraceSettings, Segment2d,
    };

    /// Makes a finite point.
    fn point(x: f32, y: f32) -> Point2 {
        Point2::try_new(x, y).expect("finite test point")
    }

    /// Adds one wall with the given material.
    fn wall(
        scene: &mut AcousticScene2d,
        start: (f32, f32),
        end: (f32, f32),
        material: AcousticMaterial,
    ) {
        scene.add_segment(
            Segment2d::try_new(point(start.0, start.1), point(end.0, end.1), material)
                .expect("nondegenerate wall"),
        );
    }

    /// Lightly absorbing, opaque walls.
    fn plaster() -> AcousticMaterial {
        AcousticMaterial::new(BandAbsorption::try_new(0.1, 0.1, 0.15).expect("valid absorption"))
    }

    /// Builds a square room centred on the origin, optionally leaving a doorway in the east wall.
    fn square_room(half: f32, doorway: Option<f32>) -> AcousticScene2d {
        let mut scene = AcousticScene2d::default();
        let material = plaster();
        wall(&mut scene, (-half, -half), (half, -half), material);
        wall(&mut scene, (-half, half), (half, half), material);
        wall(&mut scene, (-half, -half), (-half, half), material);
        match doorway {
            Some(gap) => {
                wall(&mut scene, (half, -half), (half, -gap), material);
                wall(&mut scene, (half, gap), (half, half), material);
            }
            None => wall(&mut scene, (half, -half), (half, half), material),
        }
        scene
    }

    /// Traces one listener and sources with default settings.
    fn trace(
        scene: &AcousticScene2d,
        listener: (f32, f32),
        sources: &[(f32, f32)],
    ) -> ListenerTrace2d {
        let sources: Vec<_> = sources
            .iter()
            .map(|(x, y)| Emitter2d::new(point(*x, *y)))
            .collect();
        let mut output = ListenerTrace2d::default();
        scene.trace_listener(
            Listener2d::new(point(listener.0, listener.1)),
            &sources,
            RayTraceSettings::default().with_recorded_rays(true),
            &mut output,
        );
        output
    }

    /// A sealed room hides an outside source completely and keeps all rays inside.
    #[test]
    fn sealed_room_fully_muffles_outside_source() {
        let scene = square_room(4.0, None);
        let output = trace(&scene, (0.0, 0.0), &[(10.0, 0.0)]);
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
        let reflective = square_room(4.0, None);
        let material = AcousticMaterial::new(BandAbsorption::try_new(1.0, 1.0, 1.0).unwrap());
        let mut absorbing = AcousticScene2d::default();
        for surface in reflective.segments() {
            absorbing
                .add_segment(Segment2d::try_new(surface.start(), surface.end(), material).unwrap());
        }
        let dry = trace(&absorbing, (0.0, 0.0), &[]).reverb();
        let wet = trace(&reflective, (0.0, 0.0), &[]).reverb();
        assert!(dry.wet_gain() < f32::EPSILON, "{dry:?}");
        assert!(wet.wet_gain() > 0.5, "{wet:?}");
    }

    /// With no geometry, every ray escapes and nothing reverberates.
    #[test]
    fn open_field_is_outdoor_and_dry() {
        let scene = AcousticScene2d::default();
        let output = trace(&scene, (0.0, 0.0), &[(3.0, 0.0)]);
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
        let mut scene = square_room(4.0, Some(1.0));
        // A second wall outside the doorway hides the source from a straight line.
        wall(&mut scene, (6.0, -3.0), (6.0, 3.0), plaster());
        let output = trace(&scene, (-2.0, 2.0), &[(8.0, 0.0)]);
        let source = output.sources()[0];
        assert!(!source.is_direct_visible());
        let sealed = trace(&square_room(4.0, None), (-2.0, 2.0), &[(8.0, 0.0)]).sources()[0];
        assert!(source.clarity() >= sealed.clarity());
        let around_corner =
            trace(&square_room(4.0, Some(1.0)), (-2.0, 2.0), &[(5.0, 3.0)]).sources()[0];
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
        let scene = square_room(4.0, Some(1.5));
        let output = trace(&scene, (-1.0, 0.0), &[]);
        let direction = output
            .ambient_direction()
            .expect("rays escape through the doorway");
        assert!(direction.x_m() > 0.8, "{direction:?}");
        let reverb = output.reverb();
        assert!(reverb.outdoor_fraction() > 0.0 && reverb.outdoor_fraction() < 1.0);
    }

    /// Recording emits primary, echo, occlusion, permeation, and escape segments.
    #[test]
    fn recording_contains_every_ray_kind() {
        let mut scene = square_room(4.0, Some(1.0));
        let thin = AcousticMaterial::default()
            .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
            .expect("valid material");
        wall(&mut scene, (0.0, -4.0), (0.0, 0.5), thin);
        let output = trace(&scene, (-2.0, -2.0), &[(2.0, -2.0)]);
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
    }

    /// Two transmitting walls muffle more than one.
    #[test]
    fn thicker_walls_transmit_less() {
        let leaky = AcousticMaterial::default()
            .try_with_transmission(BandGain::try_new(0.5, 0.3, 0.1).expect("valid gain"))
            .expect("valid material");
        let mut thin = AcousticScene2d::default();
        wall(&mut thin, (0.0, -20.0), (0.0, 20.0), leaky);
        let mut thick = thin.clone();
        wall(&mut thick, (0.4, -20.0), (0.4, 20.0), leaky);
        let thin_response = trace(&thin, (-2.0, 0.0), &[(2.0, 0.0)]).sources()[0];
        let thick_response = trace(&thick, (-2.0, 0.0), &[(2.0, 0.0)]).sources()[0];
        assert!(thick_response.permeation().low() < thin_response.permeation().low());
        assert!(thick_response.filter().gain_lf() < thin_response.filter().gain_lf());
    }
}
