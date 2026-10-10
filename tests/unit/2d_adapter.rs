//! Adapter behavior tests compiled against each supported Bevy minor.

use super::{RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d};
use super::{RaytracedAudioResponse2d, RaytracedAudioSurface2d};
use bevy::audio::{AudioSink, AudioSinkPlayback, Volume};
use bevy::prelude::{App, Entity, Transform, TransformPlugin, Vec2};
use bevy_raytraced_audio::{AcousticMaterial, BandGain};

/// Image-source responses obey the trace interval instead of repeating work every render frame.
#[test]
fn image_source_queries_share_listener_trace_cadence() {
    use super::RaytracedAudioTracing2d;
    use bevy::prelude::{Real, Time};
    use std::time::Duration;
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
    app.insert_resource(Time::<Real>::default());
    app.world_mut()
        .resource_mut::<RaytracedAudioTracing2d>()
        .interval_s = 0.1;
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(2.0, 0.0, 0.0)))
        .id();
    app.update();
    app.world_mut()
        .get_mut::<Transform>(emitter)
        .unwrap()
        .translation
        .x = 4.0;
    app.update();
    assert!(
        (app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .unwrap()
            .response()
            .direct
            .distance_m()
            - 2.0)
            .abs()
            < f64::EPSILON
    );
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_millis(110));
    app.update();
    assert!(
        (app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .unwrap()
            .response()
            .direct
            .distance_m()
            - 4.0)
            .abs()
            < f64::EPSILON
    );
    app.world_mut()
        .resource_mut::<RaytracedAudioTracing2d>()
        .enabled = false;
    app.world_mut()
        .get_mut::<Transform>(emitter)
        .unwrap()
        .translation
        .x = 6.0;
    app.update();
    assert!(
        (app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .unwrap()
            .response()
            .direct
            .distance_m()
            - 6.0)
            .abs()
            < f64::EPSILON
    );
}

/// Runtime toggles bypass the trace interval and clear stale processed-audio parameters.
#[test]
fn tracing_toggle_resets_processed_audio_immediately() {
    use super::{
        RaytracedAudioListenerTrace2d, RaytracedAudioPlayer, RaytracedAudioRayResponse2d,
        RaytracedAudioTracing2d,
    };
    use bevy::prelude::{Handle, Real, Time};
    use bevy_raytraced_audio::{MuffleFilter, ReverbEstimate};
    use std::sync::Arc;

    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    // Zero elapsed time prevents the periodic trace from hiding invalidation bugs.
    app.insert_resource(Time::<Real>::default());
    app.world_mut()
        .resource_mut::<RaytracedAudioTracing2d>()
        .interval_s = 60.0;
    let listener = app
        .world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()))
        .id();
    let player = RaytracedAudioPlayer::new(Handle::default());
    let params = Arc::clone(player.params());
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter2d,
            Transform::from_xyz(2.0, 0.0, 0.0),
            player,
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .resource::<RaytracedAudioListenerTrace2d>()
            .trace()
            .is_some()
    );

    params.set_filter(MuffleFilter::SILENT);
    params.set_reverb(ReverbEstimate::from_parameters(1.0, 2.0, 0.1), 1.0);
    app.world_mut()
        .resource_mut::<RaytracedAudioTracing2d>()
        .enabled = false;
    app.update();
    assert!(
        app.world()
            .resource::<RaytracedAudioListenerTrace2d>()
            .trace()
            .is_none()
    );
    assert!(
        app.world()
            .get::<RaytracedAudioRayResponse2d>(emitter)
            .is_none()
    );
    assert_eq!(params.filter(), MuffleFilter::CLEAR);
    assert!(params.wet_gain().abs() < f32::EPSILON);

    app.world_mut()
        .resource_mut::<RaytracedAudioTracing2d>()
        .enabled = true;
    app.update();
    assert!(
        app.world()
            .get::<RaytracedAudioRayResponse2d>(emitter)
            .is_some()
    );
    params.set_filter(MuffleFilter::SILENT);
    params.set_reverb(ReverbEstimate::from_parameters(1.0, 2.0, 0.1), 1.0);
    app.world_mut().despawn(listener);
    app.update();
    assert!(
        app.world()
            .resource::<RaytracedAudioListenerTrace2d>()
            .trace()
            .is_none()
    );
    assert_eq!(params.filter(), MuffleFilter::CLEAR);
    assert!(params.wet_gain().abs() < f32::EPSILON);
}

/// The plugin writes an occluded response after transforms propagate.
#[test]
fn plugin_writes_occluded_response() -> Result<(), bevy_raytraced_audio::GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(-1.0, 0.0, 0.0)))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            AcousticMaterial::default(),
        )?,
        Transform::default(),
    ));

    app.update();

    let response = app
        .world()
        .get::<RaytracedAudioResponse2d>(emitter)
        .expect("adapter inserts a response component")
        .response();
    assert!(response.direct.is_occluded());
    Ok(())
}

/// The plugin builder accepts a bounded gain and rejects invalid coefficients.
#[test]
fn plugin_builder_checks_occluded_gain() {
    assert!(
        RaytracedAudio2dPlugin::default()
            .with_occluded_gain(f32::NAN)
            .is_err()
    );
    assert!(
        RaytracedAudio2dPlugin::default()
            .with_occluded_gain(1.1)
            .is_err()
    );
    assert!(
        RaytracedAudio2dPlugin::default()
            .with_occluded_gain(0.25)
            .is_ok()
    );
}

/// Occlusion attenuates a sink and restores the latest user volume after the wall is removed.
#[test]
fn sink_volume_is_restored_after_occlusion() -> Result<(), bevy_raytraced_audio::GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(
        RaytracedAudio2dPlugin::default()
            .with_occluded_gain(0.25)
            .expect("valid opaque-path fallback"),
    );
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let mut sink = crate::test_support::test_audio_sink();
    sink.set_volume(Volume::Linear(0.4));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter2d,
            Transform::from_xyz(-1.0, 0.0, 0.0),
            sink,
        ))
        .id();
    let wall = app
        .world_mut()
        .spawn((
            RaytracedAudioSurface2d::new(
                Vec2::new(0.0, -1.0),
                Vec2::new(0.0, 1.0),
                AcousticMaterial::default(),
            )?,
            Transform::default(),
        ))
        .id();

    app.update();
    assert!((audio_volume(&app, emitter) - 0.1).abs() < 1.0e-6);

    app.world_mut()
        .get_mut::<AudioSink>(emitter)
        .expect("emitter retains its audio sink")
        .set_volume(Volume::Linear(0.2));
    app.update();
    assert!((audio_volume(&app, emitter) - 0.05).abs() < 1.0e-6);

    assert!(app.world_mut().despawn(wall));
    app.update();
    assert_eq!(audio_volume(&app, emitter), 0.2);
    Ok(())
}

/// Sink volume uses the mean direct transmission when an intersected surface passes sound.
#[test]
fn sink_volume_uses_surface_transmission() -> Result<(), bevy_raytraced_audio::GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let mut sink = crate::test_support::test_audio_sink();
    sink.set_volume(Volume::Linear(0.4));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter2d,
            Transform::from_xyz(-1.0, 0.0, 0.0),
            sink,
        ))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            AcousticMaterial::default().try_with_transmission(BandGain::try_new(0.2, 0.4, 0.6)?)?,
        )?,
        Transform::default(),
    ));

    app.update();

    let response = app
        .world()
        .get::<RaytracedAudioResponse2d>(emitter)
        .expect("the adapter publishes the per-band direct transmission")
        .response();
    assert!(response.direct.is_occluded());
    assert_eq!(response.direct.gain(), BandGain::try_new(0.2, 0.4, 0.6)?);
    assert!((audio_volume(&app, emitter) - 0.16).abs() < 1.0e-6);
    Ok(())
}

/// Removing the unique listener restores sink volume and clears stale responses.
#[test]
fn removing_listener_restores_sink_volume() -> Result<(), bevy_raytraced_audio::GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    let listener = app
        .world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    let mut sink = crate::test_support::test_audio_sink();
    sink.set_volume(Volume::Linear(0.4));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter2d,
            Transform::from_xyz(-1.0, 0.0, 0.0),
            sink,
        ))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            AcousticMaterial::default(),
        )?,
        Transform::default(),
    ));

    app.update();
    assert_eq!(audio_volume(&app, emitter), 0.0);
    assert!(
        app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .is_some()
    );

    assert!(app.world_mut().despawn(listener));
    app.update();

    assert_eq!(audio_volume(&app, emitter), 0.4);
    assert!(
        app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .is_none()
    );
    Ok(())
}

/// Reads the current Bevy sink volume in linear units.
fn audio_volume(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<AudioSink>(entity)
        .expect("emitter retains its audio sink")
        .volume()
        .to_linear()
}

/// Static inputs reuse traces; moving, removing and replacing sources invalidate the cache.
#[test]
fn deterministic_trace_cache_observes_emitter_changes() {
    use super::RaytracedAudioListenerTrace2d;
    use bevy::prelude::DetectChanges;
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(2., 0., 0.)))
        .id();
    app.update();
    let tick = app
        .world()
        .get_resource_ref::<RaytracedAudioListenerTrace2d>()
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        tick,
        app.world()
            .get_resource_ref::<RaytracedAudioListenerTrace2d>()
            .unwrap()
            .last_changed()
    );
    app.world_mut()
        .get_mut::<Transform>(emitter)
        .unwrap()
        .translation
        .x = 3.;
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .unwrap()
            .response()
            .direct
            .distance_m(),
        3.
    );
    app.world_mut().despawn(emitter);
    app.update();
    assert_eq!(
        app.world()
            .resource::<RaytracedAudioListenerTrace2d>()
            .trace()
            .unwrap()
            .sources(),
        &[]
    );
    let replacement = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(4., 0., 0.)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioResponse2d>(replacement)
            .unwrap()
            .response()
            .direct
            .distance_m(),
        4.
    );
}

/// Adding opt-in path output to an idle emitter still populates it immediately.
#[test]
fn cached_emitter_accepts_new_reflection_output() {
    use super::RaytracedAudioReflectionPaths2d;
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(0., 2., 0.)));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::default()))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(1., -1.),
            Vec2::new(1., 3.),
            AcousticMaterial::default(),
        )
        .unwrap(),
        Transform::default(),
    ));
    app.update();
    app.update();
    app.world_mut()
        .entity_mut(emitter)
        .insert(RaytracedAudioReflectionPaths2d::default());
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths2d>(emitter)
            .unwrap()
            .paths()
            .len(),
        1
    );
}

/// Many sinks recover after invalid emitter positions, ambiguous listeners and tracing changes.
#[test]
fn simultaneous_sink_recovery_and_invalid_transform_invalidation() {
    use bevy::app::TaskPoolPlugin;
    use bevy_raytraced_audio::RayTraceSettings;
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        TransformPlugin,
        RaytracedAudio2dPlugin::default()
            .with_occluded_gain(0.25)
            .unwrap()
            .with_ray_tracing(
                RayTraceSettings::default()
                    .try_with_ray_count(4)
                    .unwrap()
                    .with_max_bounces(1),
            )
            .without_ray_tracing(),
    ));
    let listener = app
        .world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1., 0., 0.)))
        .id();
    let surface = RaytracedAudioSurface2d::new(
        Vec2::new(0., -2.),
        Vec2::new(0., 2.),
        AcousticMaterial::default(),
    )
    .unwrap();
    assert_eq!(surface.segment().material(), AcousticMaterial::default());
    app.world_mut().spawn((surface, Transform::default()));
    let entities: Vec<_> = (0..128)
        .map(|_| {
            app.world_mut()
                .spawn((
                    RaytracedAudioEmitter2d,
                    Transform::from_xyz(-1., 0., 0.),
                    crate::test_support::test_audio_sink(),
                    super::RaytracedAudioReflectionPaths2d::default(),
                ))
                .id()
        })
        .collect();
    app.update();
    for entity in &entities {
        assert!(
            (app.world()
                .get::<AudioSink>(*entity)
                .unwrap()
                .volume()
                .to_linear()
                - 0.25)
                .abs()
                < f32::EPSILON
        );
    }
    app.world_mut()
        .resource_mut::<super::RaytracedAudioTracing2d>()
        .enabled = true;
    app.update();
    for entity in &entities {
        assert!(
            app.world()
                .get::<super::RaytracedAudioRayResponse2d>(*entity)
                .unwrap()
                .response()
                .filter()
                .gain_hf()
                .is_finite()
        );
        assert_eq!(
            app.world()
                .get::<super::RaytracedAudioReflectionPaths2d>(*entity)
                .unwrap()
                .paths(),
            &[]
        );
    }
    let first = *entities.first().unwrap();
    app.world_mut()
        .get_mut::<Transform>(first)
        .unwrap()
        .translation
        .x = f32::NAN;
    app.update();
    assert!(app.world().get::<RaytracedAudioResponse2d>(first).is_none());
    app.update(); // An already-invalid source is absent from the previous trace.
    app.world_mut()
        .get_mut::<Transform>(first)
        .unwrap()
        .translation
        .x = -1.;
    let duplicate = app
        .world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()))
        .id();
    app.update();
    assert!(
        app.world()
            .resource::<super::RaytracedAudioListenerTrace2d>()
            .trace()
            .is_none()
    );
    app.world_mut().despawn(duplicate);
    app.update();
    assert!(app.world().get::<RaytracedAudioResponse2d>(first).is_some());
    app.world_mut().despawn(listener);
    app.update();
    for entity in &entities {
        assert!(
            app.world()
                .get::<super::volume::RaytracedAudioBaseVolume>(*entity)
                .is_none()
        );
        assert!(
            app.world()
                .get::<super::RaytracedAudioRayResponse2d>(*entity)
                .is_none()
        );
        assert!(
            (app.world()
                .get::<AudioSink>(*entity)
                .unwrap()
                .volume()
                .to_linear()
                - 1.)
                .abs()
                < f32::EPSILON
        );
    }
}

/// Spatial sink volume follows occlusion and restores the user's base volume.
#[test]
fn spatial_sink_volume_tracks_occlusion() {
    crate::test_support::with_spatial_audio_sink(|mut sink| {
        let mut app = App::new();
        app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
        app.world_mut()
            .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
        sink.set_volume(Volume::Linear(0.4));
        let emitter = app
            .world_mut()
            .spawn((
                RaytracedAudioEmitter2d,
                Transform::from_xyz(-1.0, 0.0, 0.0),
                sink,
            ))
            .id();
        let wall = app
            .world_mut()
            .spawn((
                RaytracedAudioSurface2d::new(
                    Vec2::new(0.0, -1.0),
                    Vec2::new(0.0, 1.0),
                    AcousticMaterial::default(),
                )
                .unwrap(),
                Transform::default(),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<bevy::audio::SpatialAudioSink>(emitter)
                .unwrap()
                .volume()
                .to_linear(),
            0.0
        );
        assert!(app.world_mut().despawn(wall));
        app.update();
        assert_eq!(
            app.world()
                .get::<bevy::audio::SpatialAudioSink>(emitter)
                .unwrap()
                .volume()
                .to_linear(),
            0.4
        );
    });
}

/// Runtime cache policy toggles retain exact results and expose independent scheduling counts.
#[test]
fn result_reuse_policy_and_statistics_are_observable() {
    use super::{RaytracedAudioListenerTrace2d, RaytracedAudioTracing2d};
    let mut app = App::new();
    app.add_plugins((TransformPlugin, RaytracedAudio2dPlugin::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()));
    app.world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(2., 0., 0.)));
    app.update();
    app.update();
    let trace = app.world().resource::<RaytracedAudioListenerTrace2d>();
    assert_eq!(trace.scheduling_statistics().scheduled, 1);
    assert_eq!(trace.scheduling_statistics().unchanged, 1);
    assert_eq!(trace.trace().unwrap().cache_statistics().computations, 1);
    let expected = trace.trace().unwrap().sources().to_vec();
    {
        let mut settings = app.world_mut().resource_mut::<RaytracedAudioTracing2d>();
        settings.settings = settings.settings.with_result_reuse(false);
    }
    app.update();
    app.update();
    let trace = app.world().resource::<RaytracedAudioListenerTrace2d>();
    assert_eq!(trace.scheduling_statistics().scheduled, 3);
    assert_eq!(trace.trace().unwrap().cache_statistics().forced, 2);
    assert_eq!(trace.trace().unwrap().sources(), expected);
    {
        let mut settings = app.world_mut().resource_mut::<RaytracedAudioTracing2d>();
        settings.settings = settings.settings.with_result_reuse(true);
    }
    app.update();
    app.update();
    let trace = app.world().resource::<RaytracedAudioListenerTrace2d>();
    assert_eq!(trace.scheduling_statistics().scheduled, 4);
    assert_eq!(trace.scheduling_statistics().unchanged, 2);
    assert_eq!(trace.trace().unwrap().cache_statistics().cold, 2);
    assert_eq!(trace.trace().unwrap().sources(), expected);
}
