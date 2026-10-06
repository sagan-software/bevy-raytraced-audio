//! Checks adapter behavior when a Bevy app has no initialized compute task pool.

use bevy::ecs::schedule::{Schedules, SingleThreadedExecutor};
use bevy::prelude::{App, GlobalTransform, PostUpdate, Vec2, Vec3};
use bevy_raytraced_audio::{AcousticMaterial, GeometryError};
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
    RaytracedAudioResponse2d, RaytracedAudioSurface2d,
};
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioResponse3d, RaytracedAudioSurface3d,
};

/// The 2D plugin keeps processing sources when the app omits `TaskPoolPlugin`.
#[test]
fn public_2d_plugin_processes_many_emitters_without_a_task_pool() -> Result<(), GeometryError> {
    assert!(bevy::tasks::ComputeTaskPool::try_get().is_none());
    let mut app = App::new();
    app.add_plugins(RaytracedAudio2dPlugin::default());
    app.world_mut().spawn((
        RaytracedAudioListener2d,
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));
    let emitters = (0..32)
        .map(|index| {
            let is_occluded = index < 16;
            let x = if is_occluded { -1.0 } else { 2.0 };
            let entity = app
                .world_mut()
                .spawn((
                    RaytracedAudioEmitter2d,
                    GlobalTransform::from_xyz(x, 0.0, 0.0),
                ))
                .id();
            (entity, is_occluded)
        })
        .collect::<Vec<_>>();
    app.world_mut().spawn((
        RaytracedAudioSurface2d::new(
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            AcousticMaterial::default(),
        )?,
        GlobalTransform::IDENTITY,
    ));

    run_post_update_without_a_compute_pool(&mut app);

    for (entity, expected_occlusion) in emitters {
        let response = app
            .world()
            .get::<RaytracedAudioResponse2d>(entity)
            .expect("the plugin update publishes every emitter response")
            .response();
        assert_eq!(response.direct.is_occluded(), expected_occlusion);
    }
    Ok(())
}

/// The 3D plugin keeps processing sources when the app omits `TaskPoolPlugin`.
#[test]
fn public_3d_plugin_processes_many_emitters_without_a_task_pool() -> Result<(), GeometryError> {
    assert!(bevy::tasks::ComputeTaskPool::try_get().is_none());
    let mut app = App::new();
    app.add_plugins(RaytracedAudio3dPlugin::default());
    app.world_mut().spawn((
        RaytracedAudioListener3d,
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));
    let emitters = (0..32)
        .map(|index| {
            let is_occluded = index < 16;
            let x = if is_occluded { -1.0 } else { 2.0 };
            let entity = app
                .world_mut()
                .spawn((
                    RaytracedAudioEmitter3d,
                    GlobalTransform::from_xyz(x, 0.0, 0.0),
                ))
                .id();
            (entity, is_occluded)
        })
        .collect::<Vec<_>>();
    app.world_mut().spawn((
        RaytracedAudioSurface3d::new(
            [
                Vec3::new(0.0, -1.0, -1.0),
                Vec3::new(0.0, 1.0, -1.0),
                Vec3::new(0.0, 0.0, 1.0),
            ],
            AcousticMaterial::default(),
        )?,
        GlobalTransform::IDENTITY,
    ));

    run_post_update_without_a_compute_pool(&mut app);

    for (entity, expected_occlusion) in emitters {
        let response = app
            .world()
            .get::<RaytracedAudioResponse3d>(entity)
            .expect("the plugin update publishes every emitter response")
            .response();
        assert_eq!(response.direct.is_occluded(), expected_occlusion);
    }
    Ok(())
}

/// Runs the adapter schedule with Bevy's serial executor and no global compute pool.
fn run_post_update_without_a_compute_pool(app: &mut App) {
    {
        let mut schedules = app.world_mut().resource_mut::<Schedules>();
        let post_update = schedules
            .get_mut(PostUpdate)
            .expect("the adapter plugin adds a PostUpdate schedule");
        post_update.set_executor(SingleThreadedExecutor::new());
    }

    app.world_mut().run_schedule(PostUpdate);
}
