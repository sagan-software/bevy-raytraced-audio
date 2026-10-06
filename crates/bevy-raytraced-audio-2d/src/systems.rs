//! Transform conversion, propagation queries, and Bevy sink updates for 2d.

use super::emitter::RaytracedAudioEmitter2d;
use super::listener::RaytracedAudioListener2d;
use super::reflection_paths::RaytracedAudioReflectionPaths2d;
use super::response::RaytracedAudioResponse2d;
use super::settings::RaytracedAudioSettings;
use super::surface::RaytracedAudioSurface2d;
use super::volume::RaytracedAudioBaseVolume;
use bevy::audio::Volume;
use bevy::prelude::*;
use bevy_raytraced_audio::{AcousticScene2d, BandGain, Emitter2d, Listener2d, Point2, Segment2d};

/// Caches unchanged geometry, updates responses, and applies direct-path transmission to sinks.
pub(super) fn update_raytraced_audio(
    mut commands: Commands<'_, '_>,
    settings: Res<'_, RaytracedAudioSettings>,
    listeners: Query<'_, '_, &GlobalTransform, With<RaytracedAudioListener2d>>,
    surfaces: Query<'_, '_, (&RaytracedAudioSurface2d, &GlobalTransform)>,
    changed_surfaces: Query<
        '_,
        '_,
        (),
        (
            With<RaytracedAudioSurface2d>,
            Or<(Changed<RaytracedAudioSurface2d>, Changed<GlobalTransform>)>,
        ),
    >,
    surfaces_without_transform: Query<
        '_,
        '_,
        (),
        (With<RaytracedAudioSurface2d>, Without<GlobalTransform>),
    >,
    mut removed_surfaces: RemovedComponents<'_, '_, RaytracedAudioSurface2d>,
    mut scene: Local<'_, AcousticScene2d>,
    mut scene_is_cached: Local<'_, bool>,
    mut emitters: Query<
        '_,
        '_,
        (
            Entity,
            &GlobalTransform,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            Option<&mut RaytracedAudioBaseVolume>,
            Option<&mut RaytracedAudioResponse2d>,
            Option<&mut RaytracedAudioReflectionPaths2d>,
        ),
        With<RaytracedAudioEmitter2d>,
    >,
) {
    // A missing, ambiguous, or invalid listener disables tracing for this frame.
    let mut listener_iter = listeners.iter();
    let listener = listener_iter.next().and_then(|listener_transform| {
        if listener_iter.next().is_some() {
            return None;
        }
        let translation = listener_transform.translation();
        Point2::try_new(translation.x, translation.y)
            .ok()
            .map(Listener2d::new)
    });

    // Removal events need their own cursor because a deleted surface is absent from the changed query.
    let surface_was_removed = removed_surfaces.read().count() != 0;
    // A surface without a world transform is omitted from `surfaces` until its transform returns.
    let surface_changed = !changed_surfaces.is_empty()
        || !surfaces_without_transform.is_empty()
        || surface_was_removed;

    // Invalid listeners clear the scene; valid listeners rebuild after surface changes or missing transforms.
    if listener.is_none() {
        if *scene_is_cached {
            scene.clear();
        }
        *scene_is_cached = false;
    } else if !*scene_is_cached || surface_changed {
        scene.clear();
        for (surface, transform) in &surfaces {
            if let Some(segment) = transformed_segment(surface.segment, transform) {
                scene.add_segment(segment);
            }
        }
        *scene_is_cached = true;
    }

    // Each source uses the shared scene and owns its own response and sink volume state.
    for (
        entity,
        transform,
        mut audio_sink,
        mut spatial_sink,
        mut base_volume,
        mut response_state,
        mut reflection_paths,
    ) in &mut emitters
    {
        let translation = transform.translation();
        let response = Point2::try_new(translation.x, translation.y)
            .ok()
            .zip(listener.as_ref())
            .map(|(position, listener)| {
                let emitter = Emitter2d::new(position);
                if let Some(paths) = reflection_paths.as_deref_mut() {
                    scene.trace_with_reflection_paths(emitter, *listener, &mut paths.paths)
                } else {
                    scene.trace(emitter, *listener)
                }
            });

        // Invalid or ambiguous endpoints clear any path data left from the previous frame.
        if response.is_none()
            && let Some(paths) = reflection_paths.as_deref_mut()
        {
            paths.paths.clear();
        }

        // A sink volume change since the prior update is treated as a user's new base volume.
        let current_volume = audio_sink
            .as_deref()
            .map(AudioSinkPlayback::volume)
            .or_else(|| spatial_sink.as_deref().map(AudioSinkPlayback::volume));
        if let Some(current_volume) = current_volume {
            let current_linear = current_volume.to_linear();
            let base_linear = match base_volume.as_deref() {
                Some(previous)
                    if current_linear.to_bits() == previous.last_output_linear.to_bits() =>
                {
                    previous.base_linear
                }
                _ => current_linear,
            };
            // The sink accepts one dimensionless linear amplitude gain, so use the bands' arithmetic mean.
            let visibility_gain = response.map_or(1.0, |response| {
                let direct = response.direct;
                let transmission = direct.gain();
                if direct.is_occluded() && transmission == BandGain::ZERO {
                    settings.occluded_gain
                } else {
                    (transmission.low() + transmission.mid() + transmission.high()) / 3.0
                }
            });
            let output_linear = base_linear * visibility_gain;
            let output_volume = Volume::Linear(output_linear);
            if let Some(sink) = audio_sink.as_deref_mut() {
                sink.set_volume(output_volume);
            }
            if let Some(sink) = spatial_sink.as_deref_mut() {
                sink.set_volume(output_volume);
            }

            // Persistent state lets the adapter restore the source's own volume after occlusion.
            if response.is_some() {
                if let Some(previous) = base_volume.as_deref_mut() {
                    previous.base_linear = base_linear;
                    previous.last_output_linear = output_linear;
                } else {
                    commands.entity(entity).insert(RaytracedAudioBaseVolume {
                        base_linear,
                        last_output_linear: output_linear,
                    });
                }
            } else if base_volume.is_some() {
                commands.entity(entity).remove::<RaytracedAudioBaseVolume>();
            }
        }

        // Exposing the result supports custom processing without replacing Bevy's audio plugin.
        if let Some(response) = response {
            if let Some(existing) = response_state.as_deref_mut() {
                existing.response = response;
            } else {
                commands
                    .entity(entity)
                    .insert(RaytracedAudioResponse2d { response });
            }
        } else if response_state.is_some() {
            commands.entity(entity).remove::<RaytracedAudioResponse2d>();
        }
    }
}

/// Applies the entity's world transform and rejects transformed degenerate geometry.
fn transformed_segment(local_segment: Segment2d, transform: &GlobalTransform) -> Option<Segment2d> {
    let start = local_segment.start();
    let end = local_segment.end();
    let world_start = transform.transform_point(Vec3::new(start.x(), start.y(), 0.0));
    let world_end = transform.transform_point(Vec3::new(end.x(), end.y(), 0.0));
    let world_start = Point2::try_new(world_start.x, world_start.y).ok()?;
    let world_end = Point2::try_new(world_end.x, world_end.y).ok()?;
    Segment2d::try_new(world_start, world_end, local_segment.material()).ok()
}
