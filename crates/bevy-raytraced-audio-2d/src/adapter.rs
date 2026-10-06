//! Public facade for the Bevy 2D acoustic adapter.

mod emitter;
mod listener;
mod plugin;
mod reflection_paths;
mod response;
mod settings;
mod surface;
mod systems;
mod volume;

pub use emitter::RaytracedAudioEmitter2d;
pub use listener::RaytracedAudioListener2d;
pub use plugin::RaytracedAudio2dPlugin;
pub use reflection_paths::RaytracedAudioReflectionPaths2d;
pub use response::RaytracedAudioResponse2d;
pub use surface::RaytracedAudioSurface2d;

#[cfg(test)]
mod tests {
    //! Adapter behavior tests compiled against each supported Bevy minor.

    use super::{RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d};
    use super::{RaytracedAudioResponse2d, RaytracedAudioSurface2d};
    use bevy::audio::{AudioSink, AudioSinkPlayback, Volume};
    use bevy::prelude::{App, Entity, Transform, TransformPlugin, Vec2};
    use bevy_raytraced_audio::AcousticMaterial;

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
    fn sink_volume_is_restored_after_occlusion() -> Result<(), bevy_raytraced_audio::GeometryError>
    {
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
        assert_eq!(audio_volume(&app, emitter), 0.0);

        app.world_mut()
            .get_mut::<AudioSink>(emitter)
            .expect("emitter retains its audio sink")
            .set_volume(Volume::Linear(0.2));
        app.update();
        assert_eq!(audio_volume(&app, emitter), 0.0);

        assert!(app.world_mut().despawn(wall));
        app.update();
        assert_eq!(audio_volume(&app, emitter), 0.2);
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
}
