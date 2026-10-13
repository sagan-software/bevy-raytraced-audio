//! Listener-centred stochastic ray tracing over 2D segments.

use crate::bvh::{Bounds, Ray};
use crate::cache_statistics::{TraceCacheMiss, TraceCacheStatistics};
use crate::math2d::Vector2;
use crate::ray_trace::{
    EchoStatistics, SourceAccumulator, TraceResume, TraceRng, TraceStep, estimate_ray_units,
    for_each_slot, narrow, trace_threads, within_coherent_motion,
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
    /// Poses and folded rays of a trace that has not been published yet.
    latch: Option<TraceLatch2d>,
    /// Listener pose used by the in-progress trace, or by the last completed one.
    settled_listener: Option<Listener2d>,
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

    /// Returns whether a trace has started and has not published yet.
    #[must_use]
    pub const fn is_tracing(&self) -> bool {
        self.latch.is_some()
    }

    /// Returns the listener pose the in-progress or last completed trace used.
    #[must_use]
    pub const fn traced_listener(&self) -> Option<Listener2d> {
        self.settled_listener
    }
}

/// Poses and running totals for a 2D listener trace that is still casting rays.
#[derive(Clone, Debug)]
struct TraceLatch2d {
    /// Scene revision the trace started against.
    revision: std::sync::Arc<()>,
    /// Listener pose captured when the trace started.
    listener: Listener2d,
    /// Settings captured when the trace started.
    settings: RayTraceSettings,
    /// Primary rays already folded into `statistics` and the source accumulators.
    rays_completed: u32,
    /// Echo totals for the folded rays.
    statistics: EchoStatistics,
    /// Sum of escaped-ray ambience vectors.
    ambient_sum: Vector2,
    /// Sum of the lengths of those ambience vectors.
    ambient_weight: f64,
    /// Shared azimuth added to every primary ray, drawn once for the whole trace.
    angle_offset: f64,
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
    /// Fires every primary ray and publishes one complete listener trace.
    ///
    /// Results are written into `output`, reusing its storage. Source responses follow the order
    /// of `sources`. Repeating an identical scene, listener, source list and settings reuses
    /// the completed deterministic result. Changes to any input replace that result.
    /// See [`crate::RayTraceSettings`] for the ray model.
    ///
    /// This is [`Self::advance_listener_trace`] with a budget of every ray and
    /// [`TraceResume::Exact`]. A frame loop that must stay inside a frame budget should call
    /// `advance_listener_trace` instead.
    pub fn trace_listener(
        &self,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace2d,
    ) {
        let step = self.advance_listener_trace(
            listener,
            sources,
            settings,
            output,
            settings.ray_count(),
            TraceResume::Exact,
        );
        debug_assert!(step.completed());
    }

    /// Advances an in-progress listener trace by at most `ray_budget` primary rays.
    ///
    /// A budget of zero does not start work. An exact cache hit returns the completed trace
    /// without casting. While a trace is in progress, [`TraceResume::Coherent`] keeps casting
    /// from the poses captured at the start until [`COHERENT_TRACE_MOTION_M`](crate::COHERENT_TRACE_MOTION_M) is exceeded.
    /// Published sources, reverb, and ambience stay at the previous completed result until the
    /// new trace finishes, so a sliced trace does not clear audio for the frames it spans.
    ///
    /// Primary rays in one call share scoped threads when the batch is large enough to pay for
    /// them. Each ray draws from its own deterministic stream, and batches fold in ray order, so
    /// the published trace matches [`Self::trace_listener`] for the poses the trace started with.
    pub fn advance_listener_trace(
        &self,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace2d,
        ray_budget: u32,
        resume: TraceResume,
    ) -> TraceStep {
        let ray_count = settings.ray_count();
        if self.latch_matches(output, listener, sources, settings, resume) {
            if ray_budget == 0 {
                let completed = output
                    .latch
                    .as_ref()
                    .is_some_and(|latch| latch.rays_completed >= latch.settings.ray_count());
                return trace_step(output, completed);
            }
            return self.trace_ray_batch(output, ray_budget);
        }
        if let Some(miss) = self.cache_miss(output, listener, sources, settings) {
            if ray_budget == 0 {
                return TraceStep::new(0, ray_count, false);
            }
            output.cache_statistics.record(Some(miss));
            self.begin_listener_trace(listener, sources, settings, output);
            self.trace_ray_batch(output, ray_budget)
        } else {
            output.cache_statistics.record(None);
            TraceStep::new(ray_count, ray_count, true)
        }
    }

    /// Returns why `output`'s completed result cannot answer this request.
    ///
    /// `None` means the completed result is an exact hit. An in-progress latch is not a hit;
    /// callers check the latch before this.
    fn cache_miss(
        &self,
        output: &ListenerTrace2d,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
    ) -> Option<TraceCacheMiss> {
        let Some(revision) = settings.reuses_results().then(|| self.revision()) else {
            return Some(TraceCacheMiss::Forced);
        };
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
    }

    /// Returns whether `output`'s in-progress trace can accept this request without restarting.
    ///
    /// Source poses live on `output`, where the trace captured them, rather than on the latch.
    fn latch_matches(
        &self,
        output: &ListenerTrace2d,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        resume: TraceResume,
    ) -> bool {
        let Some(latch) = output.latch.as_ref() else {
            return false;
        };
        if latch.settings != settings || !std::sync::Arc::ptr_eq(&latch.revision, self.revision()) {
            return false;
        }
        if sources.len() != output.source_points.len() {
            return false;
        }
        let listener_ok = match resume {
            TraceResume::Exact => latch.listener == listener,
            TraceResume::Coherent => latch.listener.within_trace_motion(listener),
        };
        listener_ok
            && sources
                .iter()
                .zip(&output.source_points)
                .all(|(source, previous)| {
                    let next = source_point(*source);
                    // Clearance is an exact pose field. Stored values are finite, and adding zero
                    // canonicalizes negative zero, so bit equality matches numeric equality.
                    next.1.to_bits() == previous.1.to_bits()
                        && match resume {
                            TraceResume::Exact => next.0 == previous.0,
                            TraceResume::Coherent => within_trace_motion(previous.0, next.0),
                        }
                })
    }

    /// Drops any partial trace and captures direct paths for a new one.
    ///
    /// Published sources and reverb stay in place until the trace publishes, so audio keeps the
    /// previous result while the new rays accumulate. Recorded segments belong to the new trace
    /// and start empty.
    fn begin_listener_trace(
        &self,
        listener: Listener2d,
        sources: &[Emitter2d],
        settings: RayTraceSettings,
        output: &mut ListenerTrace2d,
    ) {
        output.cached_input = None;
        output.latch = None;
        output.segments.clear();
        output.settled_listener = Some(listener);
        let origin = listener_vector(listener);
        let mut source_points = core::mem::take(&mut output.source_points);
        source_points.clear();
        source_points.extend(sources.iter().map(|source| source_point(*source)));
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
        self.record_direct_sources(
            origin,
            &mut scratch,
            &mut output.segments,
            settings.records_rays(),
        );
        let ray_count = settings.ray_count();
        let mut base_rng = TraceRng::new(settings.seed());
        let angle_offset = base_rng.next_unit() * TAU / f64::from(ray_count);
        output.scratch = scratch;
        output.source_points = source_points;
        output.latch = Some(TraceLatch2d {
            revision: std::sync::Arc::clone(self.revision()),
            listener,
            settings,
            rays_completed: 0,
            statistics: EchoStatistics::default(),
            ambient_sum: Vector2::default(),
            ambient_weight: 0.0,
            angle_offset,
        });
    }

    /// Stores each source's direct gain and records occluded permeation segments.
    fn record_direct_sources(
        &self,
        origin: Vector2,
        scratch: &mut [SourceScratch],
        segments: &mut Vec<RaySegment2d>,
        record: bool,
    ) {
        let mut direct_crossings: Vec<(f64, Vector2, BandGain)> = Vec::new();
        for (index, source) in scratch.iter_mut().enumerate() {
            let (gain, occluded) = clear_of_source(source.point, origin, source.clearance_m)
                .map_or((BandGain::UNITY, false), |end| {
                    self.direct_transmission_with(origin, end, &mut direct_crossings)
                });
            source.accumulator.direct_visible = !occluded;
            source.accumulator.add_permeation(gain, 1.0);
            if occluded {
                record_segment(
                    segments,
                    record,
                    (origin, source.point),
                    RayKind::Permeation,
                    0.0,
                    f64::from(gain.mid()),
                    Some(index),
                );
            }
        }
    }

    /// Casts the next budget of rays on the latched trace and publishes it when it finishes.
    fn trace_ray_batch(&self, output: &mut ListenerTrace2d, ray_budget: u32) -> TraceStep {
        let Some(mut latch) = output.latch.take() else {
            return trace_step(output, false);
        };
        let ray_count = latch.settings.ray_count();
        let start_ray = latch.rays_completed;
        let end_ray = start_ray.saturating_add(ray_budget).min(ray_count);
        if end_ray == start_ray {
            output.latch = Some(latch);
            return trace_step(output, false);
        }
        let start = usize::try_from(start_ray).unwrap_or(usize::MAX);
        let end = usize::try_from(end_ray).unwrap_or(usize::MAX);
        let mut ray_results = core::mem::take(&mut output.ray_results);
        if ray_results.len() < end {
            ray_results.resize_with(end, RayResults::default);
        }
        let mut scratch = core::mem::take(&mut output.scratch);
        let _ = self.acceleration();
        let threads = trace_threads(
            estimate_ray_units(
                end_ray - start_ray,
                latch.settings.max_bounces(),
                scratch.len(),
                self.segment_count(),
            ),
            end_ray - start_ray,
        );
        // `end` is at least `start` and the vector now reaches `end`, so both splits are in range.
        let (_, rest) = ray_results.split_at_mut(start);
        let (batch, _) = rest.split_at_mut(end - start);
        for results in batch.iter_mut() {
            results.clear();
        }
        self.cast_primary_batch(&latch, &scratch, batch, start_ray, threads);
        fold_primary_batch(&mut latch, &mut scratch, &mut output.segments, batch);
        latch.rays_completed = end_ray;
        let completed = end_ray == ray_count;
        if completed {
            publish_listener_trace(output, &latch, &scratch);
        } else {
            output.latch = Some(latch);
        }
        output.scratch = scratch;
        output.ray_results = ray_results;
        TraceStep::new(end_ray, ray_count, completed)
    }

    /// Casts one frame of primary rays into `batch`, starting at `start_ray`.
    fn cast_primary_batch(
        &self,
        latch: &TraceLatch2d,
        scratch: &[SourceScratch],
        batch: &mut [RayResults],
        start_ray: u32,
        threads: usize,
    ) {
        let settings = latch.settings;
        let ray_count = settings.ray_count();
        let origin = listener_vector(latch.listener);
        let angle_offset = latch.angle_offset;
        let permeation_stride = ray_count
            .checked_div(settings.permeation_ray_count())
            .map_or(u32::MAX, |stride| stride.max(1));
        for_each_slot(batch, threads, &|local_index, results| {
            let ray_index = start_ray + u32::try_from(local_index).unwrap_or(u32::MAX);
            let angle = TAU.mul_add(f64::from(ray_index) / f64::from(ray_count), angle_offset);
            let direction = Vector2 {
                x: angle.cos(),
                y: angle.sin(),
            };
            let casts_permeation =
                settings.permeation_ray_count() > 0 && ray_index.is_multiple_of(permeation_stride);
            let mut rng = TraceRng::for_ray(settings.seed(), ray_index);
            results.active.extend(0..scratch.len());
            self.follow_ray(
                PrimaryRay {
                    origin,
                    direction,
                    casts_permeation,
                },
                scratch,
                settings,
                &mut rng,
                results,
            );
        });
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

/// Folds `batch` into the latch and scratch in ray order and appends recorded segments.
fn fold_primary_batch(
    latch: &mut TraceLatch2d,
    scratch: &mut [SourceScratch],
    segments: &mut Vec<RaySegment2d>,
    batch: &[RayResults],
) {
    for results in batch {
        latch.statistics.merge(results.statistics);
        if let Some(direction) = results.ambient {
            latch.ambient_sum = latch.ambient_sum.add(direction);
            latch.ambient_weight += direction.length();
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
        segments.extend(results.segments.iter().copied());
    }
}

/// Replaces published acoustics with one finished trace and stores its cache key.
fn publish_listener_trace(
    output: &mut ListenerTrace2d,
    latch: &TraceLatch2d,
    scratch: &[SourceScratch],
) {
    let ray_count = latch.settings.ray_count();
    output.sources.clear();
    output.sources.extend(
        scratch
            .iter()
            .map(|source| source.accumulator.finish(ray_count)),
    );
    output.reverb = ReverbEstimate::from_statistics(&latch.statistics, ray_count);
    output.ambient_direction = None;
    output.ambient_focus = 0.0;
    let ambient_length = latch.ambient_sum.length();
    if latch.ambient_weight > 0.0 && ambient_length > 0.0 {
        let unit = latch.ambient_sum.scale(ambient_length.recip());
        output.ambient_direction = Some(SolverPoint2d::new(unit.x, unit.y));
        output.ambient_focus = narrow((ambient_length / latch.ambient_weight).clamp(0.0, 1.0));
    }
    output.cached_input = latch.settings.reuses_results().then(|| {
        (
            std::sync::Arc::clone(&latch.revision),
            latch.listener,
            latch.settings,
        )
    });
    output.settled_listener = Some(latch.listener);
}

/// Returns a step for the trace currently stored in `output`.
fn trace_step(output: &ListenerTrace2d, completed: bool) -> TraceStep {
    let (rays_completed, ray_count) = output.latch.as_ref().map_or((0, 0), |latch| {
        (latch.rays_completed, latch.settings.ray_count())
    });
    TraceStep::new(rays_completed, ray_count, completed)
}

/// Returns whether two solver positions are at most [`COHERENT_TRACE_MOTION_M`](crate::COHERENT_TRACE_MOTION_M) apart.
fn within_trace_motion(previous: Vector2, next: Vector2) -> bool {
    let delta = next.subtract(previous);
    within_coherent_motion(delta.dot(delta))
}

/// Returns the listener position in solver space.
fn listener_vector(listener: Listener2d) -> Vector2 {
    Vector2::from_point(listener.position())
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
