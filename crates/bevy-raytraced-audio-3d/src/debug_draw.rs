//! Animated gizmo drawing of recorded 3D listener rays.
//!
//! The drawing replays one recorded trace as an expanding wavefront: each segment starts drawing
//! once the front reaches its path distance from the listener. The front advances with Bevy's
//! virtual [`Time`], so pausing or slowing virtual time also slows the rays.

use super::ray_tracing::{RaytracedAudioListenerTrace3d, RaytracedAudioTracing3d};
use crate::audio_2d::{RayKindMask, kind_alpha, visible_fractions};
use bevy::color::{Alpha, Color};
use bevy::gizmos::gizmos::Gizmos;
use bevy::math::Vec3;
use bevy::prelude::{Local, Res, ResMut, Resource, Time};
use bevy_raytraced_audio::{RayKind, RaySegment3d};

/// Configuration for the animated 3D ray drawing.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RaytracedAudioDebugDraw3d {
    /// Whether rays are drawn; enabling it also turns on ray recording.
    pub enabled: bool,
    /// Ray kinds to draw.
    pub kinds: RayKindMask,
    /// Wavefront speed in meters per second of virtual time; infinite draws every trace at once.
    pub ray_speed_m_per_s: f32,
    /// Length of the fading trail drawn behind each wavefront, in meters.
    ///
    /// Short trails show rays travelling and bouncing; `f32::INFINITY` keeps every full path.
    pub trail_m: f32,
    /// Seconds the completed wave stays visible before the next recorded trace replays.
    pub linger_s: f32,
}

impl Default for RaytracedAudioDebugDraw3d {
    /// Draws every kind at 20 m/s with 3 m trails, slow enough to watch rays travel and bounce.
    fn default() -> Self {
        Self {
            enabled: true,
            kinds: RayKindMask::ALL,
            ray_speed_m_per_s: 20.0,
            trail_m: 3.0,
            linger_s: 0.6,
        }
    }
}

impl RaytracedAudioDebugDraw3d {
    /// Returns the colour used for one ray kind at full energy.
    #[must_use]
    pub const fn color(kind: RayKind) -> Color {
        match kind {
            RayKind::Primary => Color::srgb(0.92, 0.92, 0.95),
            RayKind::Escaped => Color::srgb(0.82, 0.86, 0.9),
            RayKind::Occlusion => Color::srgb(0.25, 0.95, 0.45),
            RayKind::Echo => Color::srgb(0.2, 0.8, 1.0),
            RayKind::Permeation => Color::srgb(1.0, 0.45, 0.1),
            RayKind::Ambient => Color::srgb(1.0, 0.85, 0.15),
        }
    }
}

/// The trace currently being replayed.
#[derive(Default)]
pub(super) struct ReplayState {
    /// Recorded segments copied from the trace when the replay started.
    segments: Vec<RaySegment3d>,
    /// Virtual seconds since the replay started.
    elapsed_s: f32,
    /// Longest path distance reached by any segment end, in meters.
    farthest_m: f32,
}

/// Turns on ray recording while drawing is enabled.
pub(super) fn sync_recording(
    draw: Res<'_, RaytracedAudioDebugDraw3d>,
    mut tracing: ResMut<'_, RaytracedAudioTracing3d>,
) {
    if draw.enabled && !tracing.settings.records_rays() {
        tracing.settings = tracing.settings.with_recorded_rays(true);
    }
}

/// Draws the replayed trace up to the current wavefront distance.
pub(super) fn draw_rays(
    draw: Res<'_, RaytracedAudioDebugDraw3d>,
    trace: Res<'_, RaytracedAudioListenerTrace3d>,
    time: Res<'_, Time>,
    mut state: Local<'_, ReplayState>,
    mut gizmos: Gizmos<'_, '_>,
) {
    if !draw.enabled {
        state.segments.clear();
        return;
    }
    let instant = !draw.ray_speed_m_per_s.is_finite() || draw.ray_speed_m_per_s <= 0.0;
    // Trailing segments must clear the last path end before the next replay starts.
    let trail_clearance = if draw.trail_m.is_finite() {
        draw.trail_m.max(0.0)
    } else {
        0.0
    };
    let wave_finished = state.elapsed_s * draw.ray_speed_m_per_s
        > draw
            .linger_s
            .mul_add(draw.ray_speed_m_per_s, state.farthest_m + trail_clearance);
    if instant || state.segments.is_empty() || wave_finished {
        restart_replay(&trace, &mut state);
    }
    state.elapsed_s += time.delta_secs();
    let front_m = if instant {
        f32::INFINITY
    } else {
        state.elapsed_s * draw.ray_speed_m_per_s
    };

    for segment in &state.segments {
        if draw.kinds.contains(segment.kind()) {
            let trail_m = if instant { f32::INFINITY } else { draw.trail_m };
            draw_segment(&mut gizmos, *segment, (front_m, trail_m));
        }
    }
}

/// Copies the latest recorded segments and restarts the wavefront.
fn restart_replay(trace: &RaytracedAudioListenerTrace3d, state: &mut ReplayState) {
    state.segments.clear();
    state.elapsed_s = 0.0;
    state.farthest_m = 0.0;
    if let Some(trace) = trace.trace() {
        state.segments.extend_from_slice(trace.segments());
        state.farthest_m = state
            .segments
            .iter()
            .filter(|segment| segment.kind() != RayKind::Escaped)
            .map(|segment| narrow(segment.start_distance_m() + segment.length_m()))
            .fold(0.0, f32::max);
    }
}

/// Draws the part of one segment inside the travelling trail, fading from tail to head.
fn draw_segment(
    gizmos: &mut Gizmos<'_, '_>,
    segment: RaySegment3d,
    (front_m, trail_m): (f32, f32),
) {
    let start = Vec3::new(
        narrow(segment.start().x_m()),
        narrow(segment.start().y_m()),
        narrow(segment.start().z_m()),
    );
    let end = Vec3::new(
        narrow(segment.end().x_m()),
        narrow(segment.end().y_m()),
        narrow(segment.end().z_m()),
    );
    let length = start.distance(end);
    let Some((tail, head)) =
        visible_fractions(narrow(segment.start_distance_m()), length, front_m, trail_m)
    else {
        return;
    };
    let base = RaytracedAudioDebugDraw3d::color(segment.kind());
    let alpha = kind_alpha(segment.kind(), segment.energy());
    let tail_alpha = if trail_m.is_finite() { 0.0 } else { alpha };
    let head_point = start.lerp(end, head);
    gizmos.line_gradient(
        start.lerp(end, tail),
        head_point,
        base.with_alpha(tail_alpha),
        base.with_alpha(alpha),
    );
    // A bright tick marks each primary ray's travelling wavefront.
    if head < 1.0 && segment.kind() == RayKind::Primary {
        let direction = (end - start) / length;
        // Any unit vector perpendicular to the ray works; 3D has no preferred drawing plane.
        let side = direction.any_orthonormal_vector() * 0.08;
        gizmos.line(head_point - side, head_point + side, base);
    }
}

/// Narrows a solver distance to render precision.
#[expect(
    clippy::cast_possible_truncation,
    reason = "scene coordinates and path distances fit comfortably in f32"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}
