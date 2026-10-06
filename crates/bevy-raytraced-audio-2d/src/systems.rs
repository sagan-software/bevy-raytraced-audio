//! Transform conversion, propagation queries, and Bevy sink updates for 2d.

use super::emitter::RaytracedAudioEmitter2d;
use super::listener::RaytracedAudioListener2d;
use super::reflection_paths::RaytracedAudioReflectionPaths2d;
use super::response::RaytracedAudioResponse2d;
use super::settings::RaytracedAudioSettings;
use super::surface::RaytracedAudioSurface2d;
use super::volume::RaytracedAudioBaseVolume;
use bevy::audio::Volume;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use bevy_raytraced_audio::{
    AcousticResponse, AcousticScene2d, BandGain, Emitter2d, Listener2d, Point2, Segment2d,
};

/// Source count at and above which tracing uses the compute task pool.
const PARALLEL_EMITTER_THRESHOLD: usize = 16;

/// Bevy components read or updated for each 2D acoustic emitter.
type EmitterQueryData = (
    Entity,
    &'static GlobalTransform,
    Option<&'static mut AudioSink>,
    Option<&'static mut SpatialAudioSink>,
    Option<&'static mut RaytracedAudioBaseVolume>,
    Option<&'static mut RaytracedAudioResponse2d>,
    Option<&'static mut RaytracedAudioReflectionPaths2d>,
);

/// Selects entities marked as 2D emitters and their adapter state.
type EmitterQuery<'w, 's> = Query<'w, 's, EmitterQueryData, With<RaytracedAudioEmitter2d>>;

/// Caches unchanged geometry, updates responses, and applies direct-path transmission to sinks.
pub(super) fn update_raytraced_audio(
    commands: ParallelCommands<'_, '_>,
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
    emitters: EmitterQuery<'_, '_>,
) {
    let listener = unique_listener(&listeners);
    refresh_scene(
        listener.is_some(),
        &surfaces,
        &changed_surfaces,
        &surfaces_without_transform,
        &mut removed_surfaces,
        &mut scene,
        &mut scene_is_cached,
    );
    process_emitters(commands, &scene, listener, settings.occluded_gain, emitters);
}

/// Returns the unique finite 2D listener, disabling tracing for missing or ambiguous listeners.
fn unique_listener(
    listeners: &Query<'_, '_, &GlobalTransform, With<RaytracedAudioListener2d>>,
) -> Option<Listener2d> {
    let mut listener_iter = listeners.iter();
    let listener_transform = listener_iter.next()?;
    if listener_iter.next().is_some() {
        return None;
    }
    let translation = listener_transform.translation();
    Point2::try_new(translation.x, translation.y)
        .ok()
        .map(Listener2d::new)
}

/// Clears invalid scenes and rebuilds cached geometry after a surface change.
fn refresh_scene(
    listener_is_valid: bool,
    surfaces: &Query<'_, '_, (&RaytracedAudioSurface2d, &GlobalTransform)>,
    changed_surfaces: &Query<
        '_,
        '_,
        (),
        (
            With<RaytracedAudioSurface2d>,
            Or<(Changed<RaytracedAudioSurface2d>, Changed<GlobalTransform>)>,
        ),
    >,
    surfaces_without_transform: &Query<
        '_,
        '_,
        (),
        (With<RaytracedAudioSurface2d>, Without<GlobalTransform>),
    >,
    removed_surfaces: &mut RemovedComponents<'_, '_, RaytracedAudioSurface2d>,
    scene: &mut Local<'_, AcousticScene2d>,
    scene_is_cached: &mut Local<'_, bool>,
) {
    // Removed surfaces are absent from the changed query, so read their event cursor separately.
    let surface_was_removed = removed_surfaces.read().count() != 0;
    let surface_changed = !changed_surfaces.is_empty()
        || !surfaces_without_transform.is_empty()
        || surface_was_removed;

    if !listener_is_valid {
        if **scene_is_cached {
            scene.clear();
        }
        **scene_is_cached = false;
    } else if !**scene_is_cached || surface_changed {
        scene.clear();
        for (surface, transform) in surfaces {
            if let Some(segment) = transformed_segment(surface.segment, transform) {
                scene.add_segment(segment);
            }
        }
        **scene_is_cached = true;
    }
}

/// Chooses Bevy's parallel query only for large scenes with an initialized compute pool.
fn process_emitters(
    commands: ParallelCommands<'_, '_>,
    scene: &AcousticScene2d,
    listener: Option<Listener2d>,
    occluded_gain: f32,
    mut emitters: EmitterQuery<'_, '_>,
) {
    // This upper bound avoids a second full source traversal before dispatch.
    let emitter_count_upper_bound = emitters.iter().size_hint().1.unwrap_or(usize::MAX);
    let compute_pool_available = ComputeTaskPool::try_get().is_some();
    let update_one = |emitter| {
        update_emitter(&commands, scene, listener.as_ref(), occluded_gain, emitter);
    };

    if compute_pool_available && emitter_count_upper_bound >= PARALLEL_EMITTER_THRESHOLD {
        emitters.par_iter_mut().for_each(update_one);
    } else {
        emitters.iter_mut().for_each(update_one);
    }
}

/// Traces one source and updates its response, sink, and optional reflection-path components.
fn update_emitter(
    commands: &ParallelCommands<'_, '_>,
    scene: &AcousticScene2d,
    listener: Option<&Listener2d>,
    occluded_gain: f32,
    (
        entity,
        transform,
        audio_sink,
        spatial_sink,
        base_volume,
        response_state,
        mut reflection_paths,
    ): QueryItem<'_, '_, EmitterQueryData>,
) {
    let translation = transform.translation();
    let response = Point2::try_new(translation.x, translation.y)
        .ok()
        .zip(listener)
        .map(|(position, listener)| {
            let emitter = Emitter2d::new(position);
            if let Some(paths) = reflection_paths.as_deref_mut() {
                scene.trace_with_reflection_paths(emitter, *listener, &mut paths.paths)
            } else {
                scene.trace(emitter, *listener)
            }
        });

    // Invalid endpoints clear reflection paths left by a previous valid frame.
    if response.is_none()
        && let Some(paths) = reflection_paths.as_deref_mut()
    {
        paths.paths.clear();
    }

    update_sink_volume(
        commands,
        entity,
        response,
        occluded_gain,
        audio_sink,
        spatial_sink,
        base_volume,
    );
    update_response_component(commands, entity, response, response_state);
}

/// Applies direct-path gain while preserving the sink's requested base volume.
fn update_sink_volume(
    commands: &ParallelCommands<'_, '_>,
    entity: Entity,
    response: Option<AcousticResponse>,
    occluded_gain: f32,
    mut audio_sink: Option<Mut<'_, AudioSink>>,
    mut spatial_sink: Option<Mut<'_, SpatialAudioSink>>,
    base_volume: Option<Mut<'_, RaytracedAudioBaseVolume>>,
) {
    let current_volume = audio_sink
        .as_deref()
        .map(AudioSinkPlayback::volume)
        .or_else(|| spatial_sink.as_deref().map(AudioSinkPlayback::volume));
    if let Some(current_volume) = current_volume {
        let current_linear = current_volume.to_linear();
        let base_linear = match base_volume.as_deref() {
            Some(previous) if current_linear.to_bits() == previous.last_output_linear.to_bits() => {
                previous.base_linear
            }
            _ => current_linear,
        };
        let visibility_gain = response.map_or(1.0, |response| {
            let direct = response.direct;
            let transmission = direct.gain();
            if direct.is_occluded() && transmission == BandGain::ZERO {
                occluded_gain
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
        update_base_volume(
            commands,
            entity,
            response.is_some(),
            base_volume,
            base_linear,
            output_linear,
        );
    }
}

/// Inserts, updates, or removes the component that restores a sink's base volume.
fn update_base_volume(
    commands: &ParallelCommands<'_, '_>,
    entity: Entity,
    response_is_valid: bool,
    mut base_volume: Option<Mut<'_, RaytracedAudioBaseVolume>>,
    base_linear: f32,
    output_linear: f32,
) {
    if response_is_valid {
        if let Some(previous) = base_volume.as_deref_mut() {
            previous.base_linear = base_linear;
            previous.last_output_linear = output_linear;
        } else {
            commands.command_scope(|mut commands| {
                commands.entity(entity).insert(RaytracedAudioBaseVolume {
                    base_linear,
                    last_output_linear: output_linear,
                });
            });
        }
    } else if base_volume.is_some() {
        commands.command_scope(|mut commands| {
            commands.entity(entity).remove::<RaytracedAudioBaseVolume>();
        });
    }
}

/// Publishes a fresh response or removes stale response state for one emitter.
fn update_response_component(
    commands: &ParallelCommands<'_, '_>,
    entity: Entity,
    response: Option<AcousticResponse>,
    mut response_state: Option<Mut<'_, RaytracedAudioResponse2d>>,
) {
    if let Some(response) = response {
        if let Some(existing) = response_state.as_deref_mut() {
            existing.response = response;
        } else {
            commands.command_scope(|mut commands| {
                commands
                    .entity(entity)
                    .insert(RaytracedAudioResponse2d { response });
            });
        }
    } else if response_state.is_some() {
        commands.command_scope(|mut commands| {
            commands.entity(entity).remove::<RaytracedAudioResponse2d>();
        });
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
