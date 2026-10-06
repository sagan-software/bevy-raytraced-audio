//! Exercises both adapters through their consumer-facing Bevy APIs.

use bevy::audio::{AudioSinkPlayback, SpatialAudioSink, Volume};
use bevy::prelude::{App, Transform, TransformPlugin, Vec2, Vec3};
use bevy_raytraced_audio::{AcousticMaterial, GeometryError, Point2, Point3};
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
    RaytracedAudioReflectionPaths2d, RaytracedAudioResponse2d, RaytracedAudioSurface2d,
};
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioReflectionPaths3d, RaytracedAudioResponse3d, RaytracedAudioSurface3d,
};
use std::num::{NonZeroU16, NonZeroU32};

/// The public 2D plugin traces one registered segment after Bevy propagates transforms.
#[test]
fn public_2d_plugin_publishes_an_occluded_response() -> Result<(), GeometryError> {
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
        .expect("the public adapter inserts its response component")
        .response();
    assert!(response.direct.is_occluded());
    assert!(
        app.world()
            .get::<RaytracedAudioReflectionPaths2d>(emitter)
            .is_none()
    );
    Ok(())
}

/// The 2D plugin exposes first-order paths only when the emitter opts in.
#[test]
fn public_2d_plugin_updates_opt_in_reflection_paths() -> Result<(), GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut().spawn((
        RaytracedAudioListener2d,
        Transform::from_xyz(-3.0, 0.0, 0.0),
    ));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter2d,
            RaytracedAudioReflectionPaths2d::default(),
            Transform::from_xyz(-1.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -2.0),
            Vec2::new(0.0, 2.0),
            AcousticMaterial::default(),
        )?,
        Transform::default(),
    ));

    app.update();

    let paths = app
        .world()
        .get::<RaytracedAudioReflectionPaths2d>(emitter)
        .expect("the opt-in component remains on its emitter")
        .paths();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].surface_index().index(), 0);
    assert_eq!(paths[0].reflection_point().x_m(), 0.0);
    assert_eq!(paths[0].reflection_point().y_m(), 0.0);
    assert_eq!(paths[0].distance_m(), 4.0);

    let duplicate_listener = app
        .world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths2d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        0
    );
    assert!(
        app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .is_none()
    );
    assert!(app.world_mut().despawn(duplicate_listener));
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths2d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        1
    );

    app.world_mut()
        .get_mut::<Transform>(emitter)
        .expect("the emitter retains its transform")
        .translation = Vec3::new(-3.0, 0.0, 0.0);
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths2d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        0
    );
    Ok(())
}

/// The 2D surface accessor returns validated local endpoints and rejects invalid geometry.
#[test]
fn public_2d_surface_exposes_its_segment() -> Result<(), GeometryError> {
    let surface = RaytracedAudioSurface2d::new(
        Vec2::new(-2.0, 3.0),
        Vec2::new(4.0, 5.0),
        AcousticMaterial::default(),
    )?;
    assert_eq!(surface.segment().start(), Point2::try_new(-2.0, 3.0)?);
    assert_eq!(surface.segment().end(), Point2::try_new(4.0, 5.0)?);
    assert_eq!(
        RaytracedAudioSurface2d::new(
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(1.0, 0.0),
            AcousticMaterial::default(),
        )
        .unwrap_err(),
        GeometryError::NonFiniteValue,
    );
    Ok(())
}

/// Two 2D listeners disable tracing because the adapter requires one unique listener.
#[test]
fn public_2d_plugin_skips_ambiguous_listeners() {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter2d, Transform::from_xyz(-1.0, 0.0, 0.0)))
        .id();

    app.update();

    assert!(
        app.world()
            .get::<RaytracedAudioResponse2d>(emitter)
            .is_none()
    );
}

/// The 2D adapter changes the live spatial sink and restores its requested gain.
#[test]
fn public_2d_plugin_controls_spatial_sink_volume() -> Result<(), GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener2d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let mut sink = test_spatial_audio_sink();
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
    assert_eq!(spatial_audio_volume(&app, emitter), 0.0);
    assert!(app.world_mut().despawn(wall));
    app.update();
    assert_eq!(spatial_audio_volume(&app, emitter), 0.4);
    Ok(())
}

/// The public 3D plugin traces one registered triangle after transform propagation.
#[test]
fn public_3d_plugin_publishes_an_occluded_response() -> Result<(), GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio3dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener3d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter3d, Transform::from_xyz(-1.0, 0.0, 0.0)))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface3d::new(
            [
                Vec3::new(0.0, -1.0, -1.0),
                Vec3::new(0.0, 1.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            AcousticMaterial::default(),
        )?,
        Transform::default(),
    ));

    app.update();

    let response = app
        .world()
        .get::<RaytracedAudioResponse3d>(emitter)
        .expect("the public adapter inserts its response component")
        .response();
    assert!(response.direct.is_occluded());
    assert!(
        app.world()
            .get::<RaytracedAudioReflectionPaths3d>(emitter)
            .is_none()
    );
    Ok(())
}

/// The 3D plugin exposes first-order paths only when the emitter opts in.
#[test]
fn public_3d_plugin_updates_opt_in_reflection_paths() -> Result<(), GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio3dPlugin::default());
    app.world_mut().spawn((
        RaytracedAudioListener3d,
        Transform::from_xyz(-3.0, 0.0, 0.0),
    ));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter3d,
            RaytracedAudioReflectionPaths3d::default(),
            Transform::from_xyz(-1.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        RaytracedAudioSurface3d::new(
            [
                Vec3::new(0.0, -1.0, -1.0),
                Vec3::new(0.0, 1.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            AcousticMaterial::default(),
        )?,
        Transform::default(),
    ));

    app.update();

    let paths = app
        .world()
        .get::<RaytracedAudioReflectionPaths3d>(emitter)
        .expect("the opt-in component remains on its emitter")
        .paths();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].surface_index().index(), 0);
    assert_eq!(paths[0].reflection_point().x_m(), 0.0);
    assert_eq!(paths[0].reflection_point().y_m(), 0.0);
    assert_eq!(paths[0].reflection_point().z_m(), 0.0);
    assert_eq!(paths[0].distance_m(), 4.0);

    let duplicate_listener = app
        .world_mut()
        .spawn((RaytracedAudioListener3d, Transform::from_xyz(1.0, 0.0, 0.0)))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths3d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        0
    );
    assert!(
        app.world()
            .get::<RaytracedAudioResponse3d>(emitter)
            .is_none()
    );
    assert!(app.world_mut().despawn(duplicate_listener));
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths3d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        1
    );

    app.world_mut()
        .get_mut::<Transform>(emitter)
        .expect("the emitter retains its transform")
        .translation = Vec3::new(-3.0, 0.0, 0.0);
    app.update();
    assert_eq!(
        app.world()
            .get::<RaytracedAudioReflectionPaths3d>(emitter)
            .expect("the opt-in component remains on its emitter")
            .paths()
            .len(),
        0
    );
    Ok(())
}

/// The 3D surface accessor returns validated local vertices and rejects degenerate geometry.
#[test]
fn public_3d_surface_exposes_its_triangle() -> Result<(), GeometryError> {
    let vertices = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 2.0, 3.0),
        Vec3::new(1.0, 5.0, 3.0),
    ];
    let surface = RaytracedAudioSurface3d::new(vertices, AcousticMaterial::default())?;
    let segment_vertices = surface.triangle().vertices();
    assert_eq!(segment_vertices[0], Point3::try_new(1.0, 2.0, 3.0)?);
    assert_eq!(segment_vertices[1], Point3::try_new(4.0, 2.0, 3.0)?);
    assert_eq!(segment_vertices[2], Point3::try_new(1.0, 5.0, 3.0)?);
    assert_eq!(
        RaytracedAudioSurface3d::new(
            [Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0)],
            AcousticMaterial::default(),
        )
        .unwrap_err(),
        GeometryError::DegeneratePrimitive,
    );
    Ok(())
}

/// Two 3D listeners disable tracing because the adapter requires one unique listener.
#[test]
fn public_3d_plugin_skips_ambiguous_listeners() {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio3dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener3d, Transform::default()));
    app.world_mut()
        .spawn((RaytracedAudioListener3d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let emitter = app
        .world_mut()
        .spawn((RaytracedAudioEmitter3d, Transform::from_xyz(-1.0, 0.0, 0.0)))
        .id();

    app.update();

    assert!(
        app.world()
            .get::<RaytracedAudioResponse3d>(emitter)
            .is_none()
    );
}

/// The 3D adapter changes the live spatial sink and restores its requested gain.
#[test]
fn public_3d_plugin_controls_spatial_sink_volume() -> Result<(), GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(RaytracedAudio3dPlugin::default());
    app.world_mut()
        .spawn((RaytracedAudioListener3d, Transform::from_xyz(1.0, 0.0, 0.0)));
    let mut sink = test_spatial_audio_sink();
    sink.set_volume(Volume::Linear(0.4));
    let emitter = app
        .world_mut()
        .spawn((
            RaytracedAudioEmitter3d,
            Transform::from_xyz(-1.0, 0.0, 0.0),
            sink,
        ))
        .id();
    let wall = app
        .world_mut()
        .spawn((
            RaytracedAudioSurface3d::new(
                [
                    Vec3::new(0.0, -1.0, -1.0),
                    Vec3::new(0.0, 1.0, -1.0),
                    Vec3::new(0.0, 0.0, 1.0),
                ],
                AcousticMaterial::default(),
            )?,
            Transform::default(),
        ))
        .id();

    app.update();
    assert_eq!(spatial_audio_volume(&app, emitter), 0.0);
    assert!(app.world_mut().despawn(wall));
    app.update();
    assert_eq!(spatial_audio_volume(&app, emitter), 0.4);
    Ok(())
}

/// Creates a device-free spatial sink using a local rodio mixer.
fn test_spatial_audio_sink() -> SpatialAudioSink {
    let channel_count = NonZeroU16::new(2).expect("the test mixer has two channels");
    let sample_rate = NonZeroU32::new(48_000).expect("the test mixer has a positive sample rate");
    let (mixer, _mixer_source) = rodio::mixer::mixer(channel_count, sample_rate);
    let player = rodio::SpatialPlayer::connect_new(
        &mixer,
        [0.0, 0.0, 0.0],
        [-0.1, 0.0, 0.0],
        [0.1, 0.0, 0.0],
    );
    SpatialAudioSink::new(player)
}

/// Reads the current Bevy spatial sink volume in linear units.
fn spatial_audio_volume(app: &App, entity: bevy::prelude::Entity) -> f32 {
    app.world()
        .get::<SpatialAudioSink>(entity)
        .expect("emitter retains its spatial audio sink")
        .volume()
        .to_linear()
}
