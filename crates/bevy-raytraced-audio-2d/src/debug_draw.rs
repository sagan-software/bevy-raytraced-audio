//! Animated gizmo drawing of recorded 2D listener rays.
//!
//! The drawing replays one recorded trace as an expanding wavefront: each segment starts drawing
//! once the front reaches its path distance from the listener. The front advances with Bevy's
//! virtual [`Time`], so pausing or slowing virtual time also slows the rays.

use super::ray_tracing::{RaytracedAudioListenerTrace2d, RaytracedAudioTracing2d};
use bevy::color::{Alpha, Color};
use bevy::gizmos::gizmos::Gizmos;
use bevy::math::Vec3;
use bevy::prelude::{Local, Res, ResMut, Resource, Time};
use bevy_raytraced_audio::{RayKind, RaySegment2d};

/// Which ray kinds the debug drawing shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RayKindMask {
    /// One bit per [`RayKind`], in declaration order.
    bits: u8,
}

impl RayKindMask {
    /// Shows every ray kind.
    pub const ALL: Self = Self { bits: 0b11_1111 };

    /// Shows no ray kinds.
    pub const NONE: Self = Self { bits: 0 };

    /// Returns the bit for one ray kind.
    const fn bit(kind: RayKind) -> u8 {
        match kind {
            RayKind::Primary => 1,
            RayKind::Escaped => 1 << 1,
            RayKind::Occlusion => 1 << 2,
            RayKind::Echo => 1 << 3,
            RayKind::Permeation => 1 << 4,
            RayKind::Ambient => 1 << 5,
        }
    }

    /// Returns a mask that also shows `kind`.
    #[must_use]
    pub const fn with(self, kind: RayKind) -> Self {
        Self {
            bits: self.bits | Self::bit(kind),
        }
    }

    /// Returns whether `kind` is shown.
    #[must_use]
    pub const fn contains(self, kind: RayKind) -> bool {
        self.bits & Self::bit(kind) != 0
    }

    /// Shows or hides one kind.
    pub const fn set(&mut self, kind: RayKind, shown: bool) {
        if shown {
            self.bits |= Self::bit(kind);
        } else {
            self.bits &= !Self::bit(kind);
        }
    }

    /// Toggles one kind and returns whether it is now shown.
    pub const fn toggle(&mut self, kind: RayKind) -> bool {
        self.bits ^= Self::bit(kind);
        self.contains(kind)
    }
}

impl Default for RayKindMask {
    /// Shows every ray kind.
    fn default() -> Self {
        Self::ALL
    }
}

/// Configuration for the animated 2D ray drawing.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RaytracedAudioDebugDraw2d {
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
    /// Z coordinate at which rays are drawn.
    pub height: f32,
}

impl Default for RaytracedAudioDebugDraw2d {
    /// Draws every kind at 20 m/s with 3 m trails, slow enough to watch rays travel and bounce.
    fn default() -> Self {
        Self {
            enabled: true,
            kinds: RayKindMask::ALL,
            ray_speed_m_per_s: 20.0,
            trail_m: 3.0,
            linger_s: 0.6,
            height: 0.0,
        }
    }
}

impl RaytracedAudioDebugDraw2d {
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
    segments: Vec<RaySegment2d>,
    /// Virtual seconds since the replay started.
    elapsed_s: f32,
    /// Longest path distance reached by any segment end, in meters.
    farthest_m: f32,
}

/// Turns on ray recording while drawing is enabled.
pub(super) fn sync_recording(
    draw: Res<'_, RaytracedAudioDebugDraw2d>,
    mut tracing: ResMut<'_, RaytracedAudioTracing2d>,
) {
    if draw.enabled && !tracing.settings.records_rays() {
        tracing.settings = tracing.settings.with_recorded_rays(true);
    }
}

/// Draws the replayed trace up to the current wavefront distance.
pub(super) fn draw_rays(
    draw: Res<'_, RaytracedAudioDebugDraw2d>,
    trace: Res<'_, RaytracedAudioListenerTrace2d>,
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
            draw_segment(&mut gizmos, *segment, (front_m, trail_m), draw.height);
        }
    }
}

/// Copies the latest recorded segments and restarts the wavefront.
fn restart_replay(trace: &RaytracedAudioListenerTrace2d, state: &mut ReplayState) {
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

/// Returns the fractions of a segment between the trail tail and the wavefront, if any.
///
/// `start_distance_m` is the path distance at the segment start. With an infinite trail the
/// visible part starts at the segment start.
pub(crate) fn visible_fractions(
    start_distance_m: f32,
    length_m: f32,
    front_m: f32,
    trail_m: f32,
) -> Option<(f32, f32)> {
    if length_m <= 0.0 || front_m <= start_distance_m {
        return None;
    }
    let head = (front_m - start_distance_m).min(length_m);
    let tail = (front_m - trail_m - start_distance_m).max(0.0);
    (tail < head).then(|| (tail / length_m, head / length_m))
}

/// Returns the line alpha for one ray kind at a given energy.
pub(crate) const fn kind_alpha(kind: RayKind, energy: f32) -> f32 {
    match kind {
        RayKind::Primary | RayKind::Escaped => 0.6_f32.mul_add(energy, 0.25),
        RayKind::Occlusion | RayKind::Echo => 0.6_f32.mul_add(energy, 0.35),
        RayKind::Permeation | RayKind::Ambient => 0.9,
    }
}

/// Draws the part of one segment inside the travelling trail, fading from tail to head.
fn draw_segment(
    gizmos: &mut Gizmos<'_, '_>,
    segment: RaySegment2d,
    (front_m, trail_m): (f32, f32),
    height: f32,
) {
    let start = Vec3::new(
        narrow(segment.start().x_m()),
        narrow(segment.start().y_m()),
        height,
    );
    let end = Vec3::new(
        narrow(segment.end().x_m()),
        narrow(segment.end().y_m()),
        height,
    );
    let length = start.distance(end);
    let Some((tail, head)) =
        visible_fractions(narrow(segment.start_distance_m()), length, front_m, trail_m)
    else {
        return;
    };
    let base = RaytracedAudioDebugDraw2d::color(segment.kind());
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
        let side = Vec3::new(-direction.y, direction.x, 0.0) * 0.08;
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

#[cfg(test)]
#[path = "../../../tests/unit/2d_debug_draw.rs"]
mod tests;
