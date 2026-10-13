//! Listener-centred stochastic ray tracing over 2D segments.

use crate::bvh::{Bounds, Ray};
use crate::cache_statistics::{TraceCacheMiss, TraceCacheStatistics};
use crate::math2d::Vector2;
use crate::ray_trace::{
    EchoStatistics, SourceAccumulator, TraceRng, estimate_ray_units, for_each_slot, narrow,
    trace_threads,
};
use crate::scene2d::clear_of_source;
use crate::{
    AcousticScene2d, BandGain, Emitter2d, Listener2d, RayKind, RayTraceSettings, ReverbEstimate,
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

/// Position, clearance, and accumulation state fused for one traced source.
///
/// The ray loop reads this single array instead of walking parallel per-source buffers, which
/// keeps the hottest scan cache-friendly.
#[derive(Clone, Copy, Debug, Default)]
struct SourceScratch {
    /// Solver-space position in meters.
    point: Vector2,
    /// Clearance radius in meters inside which crossings are ignored.
    clearance_m: f64,
    /// Visibility, discovery, and permeation totals for the current trace.
    accumulator: SourceAccumulator,
}

/// Independent results of one primary ray, folded into the trace in ray order.
///
/// Rays write only their own slot, so lanes can execute them concurrently; folding in ray order
/// keeps every accumulated value independent of the lane count.
#[derive(Clone, Debug, Default)]
struct RayResults {
    /// Echo, bounce, and escape totals along this ray's path.
    statistics: EchoStatistics,
    /// Ambience direction scaled by remaining energy, if this ray escaped.
    ambient: Option<Vector2>,
    /// Sources this ray discovered with the mid-band energy at discovery.
    discovered: Vec<(usize, f64)>,
    /// Permeation gains this ray's first bounce contributed to hidden sources.
    permeated: Vec<(usize, BandGain)>,
    /// Segments recorded for this ray when recording is enabled.
    segments: Vec<RaySegment2d>,
    /// Sources this ray has not discovered yet; shrinks as bounces find them.
    active: Vec<usize>,
    /// Reused crossing list for this ray's permeation transmission queries.
    crossings: Vec<(f64, Vector2, BandGain)>,
}

impl RayResults {
    /// Clears one ray's results while keeping every allocation for the next trace.
    fn clear(&mut self) {
        self.statistics = EchoStatistics::default();
        self.ambient = None;
        self.discovered.clear();
        self.permeated.clear();
        self.segments.clear();
        self.active.clear();
        self.crossings.clear();
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
    /// Scratch per-source positions, clearances, and accumulators.
    scratch: Vec<SourceScratch>,
    /// Per-primary-ray results, reused across traces and folded in ray order.
    ray_results: Vec<RayResults>,
    /// Reused solver-space source positions and clearances in meters; retained across changing
    /// emitter counts.
    source_points: Vec<(Vector2, f64)>,
    /// Exact deterministic input key for the last completed trace.
    cached_input: Option<(std::sync::Arc<()>, Listener2d, RayTraceSettings)>,
    /// Cache decisions accumulated across calls, including forced computations.
    cache_statistics: TraceCacheStatistics,
}

impl ListenerTrace2d {
    /// Returns cumulative result-cache decisions for this output, including cloned history.
    #[must_use]
    pub const fn cache_statistics(&self) -> TraceCacheStatistics {
        self.cache_statistics
    }

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

    /// Clears published results while keeping allocated storage.
    fn reset(&mut self) {
        self.sources.clear();
        self.segments.clear();
        self.reverb = ReverbEstimate::DRY;
        self.ambient_direction = None;
        self.ambient_focus = 0.0;
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
    /// of `sources`. Repeating an identical scene, listener, source list and settings reuses
    /// the completed deterministic result. Changes to any input trigger a full trace.
    /// See [`crate::RayTraceSettings`] for the ray model.
    ///
    /// Primary rays execute on scoped threads when the estimated work is large enough to pay
    /// for them. Each ray draws from its own deterministic stream derived from the seed and its
    /// index, and per-ray results fold in ray order, so a completed trace depends only on its
    /// inputs, never on the lane count.
    pub fn trace_listener(
        &self,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace2d,
    ) {
        let revision = settings.reuses_results().then(|| self.revision());
        let miss = if let Some(revision) = revision {
            match &output.cached_input {
                None => Some(TraceCacheMiss::Cold),
                Some((previous, _, _)) if !std::sync::Arc::ptr_eq(previous, revision) => {
                    Some(TraceCacheMiss::Scene)
                }
                Some((_, receiver, _)) if *receiver != listener => Some(TraceCacheMiss::Listener),
                Some((_, _, config)) if *config != settings => Some(TraceCacheMiss::Settings),
                Some(_)
                    if sources.len() != output.source_points.len()
                        || !sources
                            .iter()
                            .zip(&output.source_points)
                            .all(|(source, previous)| source_point(*source) == *previous) =>
                {
                    Some(TraceCacheMiss::Sources)
                }
                Some(_) => None,
            }
        } else {
            Some(TraceCacheMiss::Forced)
        };
        output.cache_statistics.record(miss);
        if miss.is_none() {
            return;
        }
        // Invalidate before recomputing so unwinding cannot leave a partially refreshed cache.
        output.cached_input = None;
        output.reset();
        let record = settings.records_rays();
        let origin = Vector2::from_point(listener.position());
        let mut source_points = core::mem::take(&mut output.source_points);
        source_points.clear();
        source_points.extend(sources.iter().map(|source| source_point(*source)));

        // Positions, clearances, and accumulators share one array for the ray loop's hot scan.
        let mut scratch = core::mem::take(&mut output.scratch);
        scratch.clear();
        scratch.extend(
            source_points
                .iter()
                .map(|&(point, clearance_m)| SourceScratch {
                    point,
                    clearance_m,
                    accumulator: SourceAccumulator::default(),
                }),
        );

        // Straight lines from the listener decide direct visibility and seed permeation.
        let mut direct_crossings: Vec<(f64, Vector2, BandGain)> = Vec::new();
        for (index, source) in scratch.iter_mut().enumerate() {
            // Crossings inside the source clearance are skipped by ending the path before it.
            let (gain, occluded) = clear_of_source(source.point, origin, source.clearance_m)
                .map_or((BandGain::UNITY, false), |end| {
                    self.direct_transmission_with(origin, end, &mut direct_crossings)
                });
            source.accumulator.direct_visible = !occluded;
            source.accumulator.add_permeation(gain, 1.0);
            if occluded {
                record_segment(
                    &mut output.segments,
                    record,
                    (origin, source.point),
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
        let mut base_rng = TraceRng::new(settings.seed());
        let angle_offset = base_rng.next_unit() * TAU / f64::from(ray_count);

        // Per-ray result slots keep their allocations across traces and lane counts.
        let slot_count = usize::try_from(ray_count).unwrap_or(usize::MAX);
        let mut ray_results = core::mem::take(&mut output.ray_results);
        while ray_results.len() < slot_count {
            ray_results.push(RayResults::default());
        }
        for results in ray_results.iter_mut().take(slot_count) {
            results.clear();
        }
        let (ray_slots, _) = ray_results.split_at_mut(slot_count);

        // The hierarchy must exist before lanes run, so no thread waits on its lazy build.
        let _ = self.acceleration();
        let units = estimate_ray_units(
            ray_count,
            settings.max_bounces(),
            scratch.len(),
            self.segment_count(),
        );
        let threads = trace_threads(units);
        for_each_slot(ray_slots, threads, &|ray_index, results| {
            let ray_index = u32::try_from(ray_index).unwrap_or(u32::MAX);
            let angle = TAU.mul_add(f64::from(ray_index) / f64::from(ray_count), angle_offset);
            let direction = Vector2 {
                x: angle.cos(),
                y: angle.sin(),
            };
            let casts_permeation =
                settings.permeation_ray_count() > 0 && ray_index % permeation_stride == 0;
            let mut rng = TraceRng::for_ray(settings.seed(), ray_index);
            results.active.extend(0..scratch.len());
            self.follow_ray(
                PrimaryRay {
                    origin,
                    direction,
                    casts_permeation,
                },
                &scratch,
                settings,
                &mut rng,
                results,
            );
        });

        // Folding in ray order makes statistics, accumulations, and recordings lane independent.
        let mut statistics = EchoStatistics::default();
        let mut ambient_sum = Vector2::default();
        let mut ambient_weight = 0.0_f64;
        for results in ray_slots.iter() {
            statistics.merge(results.statistics);
            if let Some(direction) = results.ambient {
                ambient_sum = ambient_sum.add(direction);
                ambient_weight += direction.length();
            }
            for &(index, energy) in &results.discovered {
                if let Some(source) = scratch.get_mut(index) {
                    source.accumulator.discovered_energy += energy;
                }
            }
            for &(index, gain) in &results.permeated {
                if let Some(source) = scratch.get_mut(index) {
                    source.accumulator.add_permeation(gain, 1.0);
                }
            }
            output.segments.extend(results.segments.iter().copied());
        }

        output.reverb = ReverbEstimate::from_statistics(&statistics, ray_count);
        let ambient_length = ambient_sum.length();
        if ambient_weight > 0.0 && ambient_length > 0.0 {
            let unit = ambient_sum.scale(ambient_length.recip());
            output.ambient_direction = Some(SolverPoint2d::new(unit.x, unit.y));
            output.ambient_focus = narrow((ambient_length / ambient_weight).clamp(0.0, 1.0));
        }
        output.sources.extend(
            scratch
                .iter()
                .map(|source| source.accumulator.finish(ray_count)),
        );
        output.scratch = scratch;
        output.ray_results = ray_results;
        output.source_points = source_points;
        output.cached_input =
            revision.map(|revision| (std::sync::Arc::clone(revision), listener, settings));
    }

    /// Follows one primary ray, writing its independent results into `results`.
    fn follow_ray(
        &self,
        ray: PrimaryRay,
        sources: &[SourceScratch],
        settings: RayTraceSettings,
        rng: &mut TraceRng,
        results: &mut RayResults,
    ) {
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
                let ambient = Self::escape(
                    (listener, position, direction),
                    (bounce, travelled_m, energy.mid),
                    last_echo_point,
                    settings,
                    results,
                );
                results.ambient = ambient;
                return;
            };

            let hit_point = position.add(direction.scale(hit.distance_m));
            record_segment(
                &mut results.segments,
                record,
                (position, hit_point),
                RayKind::Primary,
                travelled_m,
                energy.mid,
                None,
            );
            travelled_m += hit.distance_m;
            let Some(segment) = self.segments().get(hit.segment_index) else {
                return;
            };
            let material = segment.material();
            let reflected = material.reflected_fraction();
            energy.low *= reflected.low();
            energy.mid *= reflected.mid();
            energy.high *= reflected.high();
            let statistics = &mut results.statistics;
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
                record_segment(
                    &mut results.segments,
                    record,
                    (hit_point, listener),
                    RayKind::Echo,
                    travelled_m,
                    energy.mid,
                    None,
                );
            }

            self.discover_sources(
                (hit_point, hit, travelled_m, energy.mid),
                sources,
                record,
                results,
            );

            if bounce == 0 && ray.casts_permeation {
                let start = hit_point.subtract(direction.scale(PERMEATION_BACKOFF_M));
                for (index, source) in sources.iter().enumerate() {
                    if source.accumulator.direct_visible {
                        continue;
                    }
                    let (gain, _) = clear_of_source(source.point, start, source.clearance_m)
                        .map_or((BandGain::UNITY, false), |end| {
                            self.direct_transmission_with(start, end, &mut results.crossings)
                        });
                    results.permeated.push((index, gain));
                    record_segment(
                        &mut results.segments,
                        record,
                        (hit_point, source.point),
                        RayKind::Permeation,
                        travelled_m,
                        f64::from(gain.mid()),
                        Some(index),
                    );
                }
            }

            if energy.low.max(energy.mid).max(energy.high) < MINIMUM_RAY_ENERGY {
                return;
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
    }

    /// Records an escaped leg into the ray's results and returns its scaled ambience direction.
    fn escape(
        (listener, position, direction): (Vector2, Vector2, Vector2),
        (bounce, travelled_m, energy): (u32, f64, f64),
        last_echo_point: Option<Vector2>,
        settings: RayTraceSettings,
        results: &mut RayResults,
    ) -> Option<Vector2> {
        let record = settings.records_rays();
        results.statistics.escaped_rays += 1;
        let end = position.add(direction.scale(settings.escape_distance_m()));
        record_segment(
            &mut results.segments,
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
        record_segment(
            &mut results.segments,
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
    ///
    /// Only the ray's still-undiscovered sources are scanned, and each discovery removes its
    /// source from that scan, so later bounces of the same ray skip work the earlier bounces
    /// already finished.
    fn discover_sources(
        &self,
        (hit_point, hit, travelled_m, energy): (Vector2, Hit, f64, f64),
        sources: &[SourceScratch],
        record: bool,
        results: &mut RayResults,
    ) {
        let RayResults {
            active,
            discovered,
            segments,
            ..
        } = results;
        let bounce_start = discovered.len();
        let mut cursor = 0;
        while cursor < active.len() {
            let Some(&index) = active.get(cursor) else {
                break;
            };
            let visible = sources.get(index).is_some_and(|source| {
                let faces_source = source.point.subtract(hit_point).dot(hit.normal) > 0.0;
                faces_source
                    && clear_of_source(source.point, hit_point, source.clearance_m).is_none_or(
                        |end| !self.segment_is_blocked(hit_point, end, Some(hit.segment_index)),
                    )
            });
            if visible {
                discovered.push((index, energy));
                let last = active.len() - 1;
                active.swap(cursor, last);
                active.pop();
            } else {
                cursor += 1;
            }
        }
        if record {
            // Recorded paths keep source order within the bounce even though discovery does not.
            let (_, bounce_discoveries) = discovered.split_at_mut(bounce_start);
            bounce_discoveries.sort_unstable_by_key(|&(index, _)| index);
            for &(index, _) in bounce_discoveries.iter() {
                let Some(source) = sources.get(index) else {
                    continue;
                };
                record_segment(
                    segments,
                    true,
                    (hit_point, source.point),
                    RayKind::Occlusion,
                    travelled_m,
                    energy,
                    Some(index),
                );
            }
        }
    }

    /// Returns whether any segment other than `skipped_segment` crosses the open path interior.
    ///
    /// The visibility-specialized traversal only visits nodes the path passes through and stops
    /// after the first blocker, which matters because every bounce casts one check per source
    /// and one toward the listener.
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
        let acceleration = self.acceleration();
        acceleration.hierarchy.any_hit(
            Ray::new((start.x, start.y, 0.0), (direction.x, direction.y, 0.0)),
            Bounds::path_2d((start.x, start.y), (end.x, end.y)),
            limit,
            skipped_segment,
            |index| {
                let Some(segment) = acceleration.segments.get(index) else {
                    return false;
                };
                ray_segment_distance(start, direction, segment.start, segment.end)
                    .is_some_and(|distance| distance < limit)
            },
        )
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
        let acceleration = self.acceleration();
        acceleration.hierarchy.visit_ray(
            Ray::new((origin.x, origin.y, 0.0), (direction.x, direction.y, 0.0)),
            maximum_distance_m,
            |index| {
                if skipped_segment == Some(index) {
                    return None;
                }
                let segment = acceleration.segments.get(index)?;
                let start = segment.start;
                let end = segment.end;
                let distance = ray_segment_distance(origin, direction, start, end)?;
                if distance > maximum_distance_m
                    || best.is_some_and(|current| current.distance_m <= distance)
                {
                    return None;
                }
                let mut normal = segment.normal.scale(segment.length.recip());
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

/// Converts one emitter into its solver position and clearance in meters.
fn source_point(source: Emitter2d) -> (Vector2, f64) {
    (
        Vector2::from_point(source.position()),
        f64::from(source.clearance_m()),
    )
}

/// Records one segment into a buffer when recording is enabled.
fn record_segment(
    segments: &mut Vec<RaySegment2d>,
    enabled: bool,
    (start, end): (Vector2, Vector2),
    kind: RayKind,
    start_distance_m: f64,
    energy: f64,
    source_index: Option<usize>,
) {
    if enabled {
        segments.push(RaySegment2d {
            start: SolverPoint2d::new(start.x, start.y),
            end: SolverPoint2d::new(end.x, end.y),
            kind,
            start_distance_m,
            energy: narrow(energy.clamp(0.0, 1.0)),
            source_index,
        });
    }
}

/// Inputs that identify one primary ray.
#[derive(Clone, Copy, Debug)]
struct PrimaryRay {
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
#[path = "../tests/unit/ray_tracer2d.rs"]
mod tests;
