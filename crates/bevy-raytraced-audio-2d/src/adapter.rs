//! Public facade for the Bevy 2D acoustic adapter.

#[cfg(feature = "debug_draw")]
mod debug_draw;
mod emitter;
mod listener;
mod plugin;
mod ray_tracing;
mod reflection_paths;
mod response;
mod settings;
mod surface;
mod systems;
mod volume;

pub use crate::processed_audio::{
    RaytracedAudioPlayer, RaytracedAudioPrepareSystems, RaytracedAudioPrepared,
    RaytracedAudioProcessingPlugin, RaytracedAudioSource,
};
#[cfg(feature = "debug_draw")]
pub use debug_draw::{RayKindMask, RaytracedAudioDebugDraw2d};
#[cfg(feature = "debug_draw")]
pub(crate) use debug_draw::{kind_alpha, visible_fractions};
pub use emitter::RaytracedAudioEmitter2d;
pub use listener::RaytracedAudioListener2d;
#[cfg(feature = "debug_draw")]
pub use plugin::RaytracedAudio2dDebugPlugin;
pub use plugin::RaytracedAudio2dPlugin;
pub use ray_tracing::{
    RaytracedAudioListenerTrace2d, RaytracedAudioRayResponse2d, RaytracedAudioTracing2d,
};
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
    fn sink_volume_is_restored_after_occlusion() -> Result<(), bevy_raytraced_audio::GeometryError>
    {
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
                AcousticMaterial::default()
                    .try_with_transmission(BandGain::try_new(0.2, 0.4, 0.6)?)?,
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
        assert!(
            app.world()
                .resource::<RaytracedAudioListenerTrace2d>()
                .trace()
                .unwrap()
                .sources()
                .is_empty()
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
}
