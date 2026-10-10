//! Device-free debug drawing checks.

use super::{
    Alpha, Gizmos, Local, RayKind, RayKindMask, RaytracedAudioDebugDraw3d,
    RaytracedAudioListenerTrace3d, RaytracedAudioTracing3d, ReplayState, Res, ResMut, Time,
    draw_rays, restart_replay, sync_recording,
};

/// Device-free replay checks use the real Gizmos system parameter and inspect emitted vertices.
#[test]
fn replay_emits_finite_vertices_and_observes_controls() {
    use bevy::ecs::system::SystemState;
    use bevy::gizmos::{AppGizmoBuilder, config::DefaultGizmoConfigGroup};
    use bevy::prelude::App;
    use bevy_raytraced_audio::{AcousticScene3d, Emitter3d, Listener3d, Point3, RayTraceSettings};
    let mut app = App::new();
    app.init_gizmo_group::<DefaultGizmoConfigGroup>();
    app.init_resource::<RaytracedAudioDebugDraw3d>();
    app.init_resource::<RaytracedAudioTracing3d>();
    app.init_resource::<Time>();
    let mut scene = AcousticScene3d::default();
    let point = |x, y, z| Point3::try_new(x, y, z).unwrap();
    scene.add_triangle(
        bevy_raytraced_audio::Triangle3d::try_new(
            [point(0., -3., -3.), point(0., 3., -3.), point(0., 0., 3.)],
            bevy_raytraced_audio::AcousticMaterial::default(),
        )
        .unwrap(),
    );
    let mut trace = RaytracedAudioListenerTrace3d::default();
    scene.trace_listener(
        Listener3d::new(point(-2., 0., 0.)),
        &[
            Emitter3d::new(point(-1., 0., 0.)),
            Emitter3d::new(point(1., 0., 0.)),
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
        Res<'_, RaytracedAudioDebugDraw3d>,
        ResMut<'_, RaytracedAudioTracing3d>,
    )>::new(app.world_mut());
    let (draw, tracing) =
        crate::test_support::test_system_param(recording.get_mut(app.world_mut()));
    sync_recording(draw, tracing);
    assert!(
        app.world()
            .resource::<RaytracedAudioTracing3d>()
            .settings
            .records_rays()
    );
    let mut state = SystemState::<(
        Res<'_, RaytracedAudioDebugDraw3d>,
        Res<'_, RaytracedAudioListenerTrace3d>,
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
        let mut config = app.world_mut().resource_mut::<RaytracedAudioDebugDraw3d>();
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
        .resource_mut::<RaytracedAudioListenerTrace3d>()
        .valid = false;
    restart_replay(
        app.world().resource::<RaytracedAudioListenerTrace3d>(),
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
        assert!(RaytracedAudioDebugDraw3d::color(kind).alpha().is_finite());
    }
}

/// The debug plugin registers its resource and drawing systems without requiring a GPU.
#[test]
fn plugin_registers_drawing_configuration() {
    let mut app = bevy::prelude::App::new();
    app.add_plugins(super::super::RaytracedAudio3dDebugPlugin);
    assert!(app.world().contains_resource::<RaytracedAudioDebugDraw3d>());
}
