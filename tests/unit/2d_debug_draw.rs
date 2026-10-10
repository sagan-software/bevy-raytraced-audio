//! Trail window arithmetic used by the animated ray drawing.

use super::visible_fractions;

/// The wavefront reveals a segment progressively, and a finite trail hides its tail.
#[test]
fn trail_window_follows_the_wavefront() {
    assert_eq!(visible_fractions(2.0, 4.0, 1.0, 3.0), None);
    assert_eq!(
        visible_fractions(2.0, 4.0, 4.0, f32::INFINITY),
        Some((0.0, 0.5))
    );
    assert_eq!(visible_fractions(2.0, 4.0, 5.0, 2.0), Some((0.25, 0.75)));
    assert_eq!(visible_fractions(2.0, 4.0, 9.0, 2.0), None);
    assert_eq!(
        visible_fractions(2.0, 4.0, 9.0, f32::INFINITY),
        Some((0.0, 1.0))
    );
}

use super::{
    Alpha, Gizmos, Local, RayKind, RayKindMask, RaytracedAudioDebugDraw2d,
    RaytracedAudioListenerTrace2d, RaytracedAudioTracing2d, ReplayState, Res, ResMut, Time,
    draw_rays, restart_replay, sync_recording,
};

/// Device-free replay checks use the real Gizmos system parameter and inspect emitted vertices.
#[test]
fn replay_emits_finite_vertices_and_observes_controls() {
    use bevy::ecs::system::SystemState;
    use bevy::gizmos::{AppGizmoBuilder, config::DefaultGizmoConfigGroup};
    use bevy::prelude::App;
    use bevy_raytraced_audio::{AcousticScene2d, Emitter2d, Listener2d, Point2, RayTraceSettings};
    let mut app = App::new();
    app.init_gizmo_group::<DefaultGizmoConfigGroup>();
    app.init_resource::<RaytracedAudioDebugDraw2d>();
    app.init_resource::<RaytracedAudioTracing2d>();
    app.init_resource::<Time>();
    let mut scene = AcousticScene2d::default();
    let point = |x, y| Point2::try_new(x, y).unwrap();
    scene.add_segment(
        bevy_raytraced_audio::Segment2d::try_new(
            point(0., -3.),
            point(0., 3.),
            bevy_raytraced_audio::AcousticMaterial::default(),
        )
        .unwrap(),
    );
    let mut trace = RaytracedAudioListenerTrace2d::default();
    scene.trace_listener(
        Listener2d::new(point(-2., 0.)),
        &[
            Emitter2d::new(point(-1., 0.)),
            Emitter2d::new(point(1., 0.)),
        ],
        RayTraceSettings::default()
            .try_with_ray_count(32)
            .unwrap()
            .with_recorded_rays(true),
        &mut trace.trace,
    );
    trace.valid = true;
    app.insert_resource(trace);
    let mut recording = SystemState::<(
        Res<'_, RaytracedAudioDebugDraw2d>,
        ResMut<'_, RaytracedAudioTracing2d>,
    )>::new(app.world_mut());
    let (draw, tracing) =
        crate::test_support::test_system_param(recording.get_mut(app.world_mut()));
    sync_recording(draw, tracing);
    assert!(
        app.world()
            .resource::<RaytracedAudioTracing2d>()
            .settings
            .records_rays()
    );
    let mut state = SystemState::<(
        Res<'_, RaytracedAudioDebugDraw2d>,
        Res<'_, RaytracedAudioListenerTrace2d>,
        Res<'_, Time>,
        Local<'_, ReplayState>,
        Gizmos<'_, '_>,
    )>::new(app.world_mut());
    for (speed, trail, enabled, mask) in [
        (f32::INFINITY, f32::INFINITY, true, RayKindMask::ALL),
        (20., 3., true, RayKindMask::ALL),
        (20., 0., true, RayKindMask::ALL),
        (20., 3., true, RayKindMask::NONE),
        (20., 3., false, RayKindMask::ALL),
    ] {
        let mut config = app.world_mut().resource_mut::<RaytracedAudioDebugDraw2d>();
        config.ray_speed_m_per_s = speed;
        config.trail_m = trail;
        config.enabled = enabled;
        config.kinds = mask;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(20));
        let (draw, trace, time, local, gizmos) =
            crate::test_support::test_system_param(state.get_mut(app.world_mut()));
        draw_rays(draw, trace, time, local, gizmos);
        let (_, _, _, local, mut gizmos) =
            crate::test_support::test_system_param(state.get_mut(app.world_mut()));
        assert!(gizmos.list_positions.iter().all(|point| point.is_finite()));
        if speed.is_infinite() {
            assert_ne!(gizmos.list_positions, Vec::new());
        }
        if !enabled {
            assert_eq!(local.segments, Vec::new());
        }
        gizmos.clear();
    }
    let mut replay = ReplayState::default();
    app.world_mut()
        .resource_mut::<RaytracedAudioListenerTrace2d>()
        .valid = false;
    restart_replay(
        app.world().resource::<RaytracedAudioListenerTrace2d>(),
        &mut replay,
    );
    assert_eq!(replay.segments, Vec::new());
    for kind in [
        RayKind::Primary,
        RayKind::Escaped,
        RayKind::Occlusion,
        RayKind::Echo,
        RayKind::Permeation,
        RayKind::Ambient,
    ] {
        assert!(RaytracedAudioDebugDraw2d::color(kind).alpha().is_finite());
    }
}

/// Every drawing kind can be enabled, disabled and toggled independently.
#[test]
fn masks_preserve_independent_bits_and_plugin_registers() {
    let mut mask = RayKindMask::default();
    for kind in [
        RayKind::Primary,
        RayKind::Escaped,
        RayKind::Occlusion,
        RayKind::Echo,
        RayKind::Permeation,
        RayKind::Ambient,
    ] {
        assert!(mask.contains(kind));
        mask.set(kind, false);
        assert!(!mask.contains(kind));
        assert!(mask.toggle(kind));
        assert!(!mask.toggle(kind));
        mask = mask.with(kind);
        mask.set(kind, true);
        assert!(mask.contains(kind));
    }
    let mut app = bevy::prelude::App::new();
    app.add_plugins(super::super::RaytracedAudio2dDebugPlugin);
    assert!(app.world().contains_resource::<RaytracedAudioDebugDraw2d>());
}
