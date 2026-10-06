//! Verifies the public CPU fallback preference without a render sub-app.

use bevy::prelude::{App, Transform, TransformPlugin, Vec2, Vec3};
use bevy_raytraced_audio::{AcousticMaterial, AudioBackendPreference};
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
    RaytracedAudioResponse2d, RaytracedAudioSurface2d,
};
use bevy_raytraced_audio_3d::{
    RaytracedAudio3dPlugin, RaytracedAudioEmitter3d, RaytracedAudioListener3d,
    RaytracedAudioResponse3d, RaytracedAudioSurface3d,
};

/// `Auto` keeps the CPU path active when the application has no Bevy renderer.
#[test]
fn auto_preference_uses_cpu_without_render_app() -> Result<(), bevy_raytraced_audio::GeometryError>
{
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(
        RaytracedAudio2dPlugin::default().with_backend_preference(AudioBackendPreference::Auto),
    );
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
        .expect("the CPU fallback publishes a response without RenderApp")
        .response();
    assert!(response.direct.is_occluded());
    Ok(())
}

/// `Auto` keeps the CPU path active for 3D when the application has no renderer.
#[test]
fn auto_preference_uses_cpu_without_render_app_3d()
-> Result<(), bevy_raytraced_audio::GeometryError> {
    let mut app = App::new();
    app.add_plugins(TransformPlugin);
    app.add_plugins(
        RaytracedAudio3dPlugin::default().with_backend_preference(AudioBackendPreference::Auto),
    );
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
        .expect("the 3D CPU fallback publishes without RenderApp")
        .response();
    assert!(response.direct.is_occluded());
    Ok(())
}
