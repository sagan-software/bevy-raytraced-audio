//! Transform conversion, propagation queries, and Bevy sink updates for 3d.

use super::emitter::RaytracedAudioEmitter3d;
use super::listener::RaytracedAudioListener3d;
use super::reflection_paths::RaytracedAudioReflectionPaths3d;
use super::response::RaytracedAudioResponse3d;
use super::settings::RaytracedAudioSettings;
use super::surface::RaytracedAudioSurface3d;
use super::volume::RaytracedAudioBaseVolume;
use bevy::audio::Volume;
use bevy::prelude::*;
use bevy_raytraced_audio::{AcousticScene3d, Emitter3d, Listener3d, Point3, Triangle3d};

/// Rebuilds a reusable scene, updates responses, and applies direct-path occlusion.
pub(super) fn update_raytraced_audio(
    mut commands: Commands<'_, '_>,
    settings: Res<'_, RaytracedAudioSettings>,
    listeners: Query<'_, '_, &GlobalTransform, With<RaytracedAudioListener3d>>,
    surfaces: Query<'_, '_, (&RaytracedAudioSurface3d, &GlobalTransform)>,
    mut scene: Local<'_, AcousticScene3d>,
    mut emitters: Query<
        '_,
        '_,
        (
            Entity,
            &GlobalTransform,
            Option<&mut AudioSink>,
            Option<&mut SpatialAudioSink>,
            Option<&mut RaytracedAudioBaseVolume>,
            Option<&mut RaytracedAudioResponse3d>,
            Option<&mut RaytracedAudioReflectionPaths3d>,
        ),
        With<RaytracedAudioEmitter3d>,
    >,
) {
    // A missing, ambiguous, or invalid listener disables tracing for this frame.
    let mut listener_iter = listeners.iter();
    let listener = listener_iter.next().and_then(|listener_transform| {
        if listener_iter.next().is_some() {
            return None;
        }
        let translation = listener_transform.translation();
        Point3::try_new(translation.x, translation.y, translation.z)
            .ok()
            .map(Listener3d::new)
    });

    // Clearing retains the local scene's allocated capacity between frames.
    scene.clear();
    if listener.is_some() {
        for (surface, transform) in &surfaces {
            if let Some(triangle) = transformed_triangle(surface.triangle, transform) {
                scene.add_triangle(triangle);
            }
        }
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
        let response = Point3::try_new(translation.x, translation.y, translation.z)
            .ok()
            .zip(listener.as_ref())
            .map(|(position, listener)| {
                let emitter = Emitter3d::new(position);
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
            let visibility_gain = response.map_or(1.0, |response| {
                if response.direct.is_occluded() {
                    settings.occluded_gain
                } else {
                    1.0
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
                    .insert(RaytracedAudioResponse3d { response });
            }
        } else if response_state.is_some() {
            commands.entity(entity).remove::<RaytracedAudioResponse3d>();
        }
    }
}

/// Applies the entity's world transform and rejects transformed degenerate geometry.
fn transformed_triangle(
    local_triangle: Triangle3d,
    transform: &GlobalTransform,
) -> Option<Triangle3d> {
    let [first, second, third] = local_triangle.vertices();
    let first = transform.transform_point(Vec3::new(first.x(), first.y(), first.z()));
    let second = transform.transform_point(Vec3::new(second.x(), second.y(), second.z()));
    let third = transform.transform_point(Vec3::new(third.x(), third.y(), third.z()));
    let vertices = [
        Point3::try_new(first.x, first.y, first.z).ok()?,
        Point3::try_new(second.x, second.y, second.z).ok()?,
        Point3::try_new(third.x, third.y, third.z).ok()?,
    ];
    Triangle3d::try_new(vertices, local_triangle.material()).ok()
}
