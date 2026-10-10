//! Transform conversion, propagation queries, and Bevy sink updates for 3d.

use super::emitter::RaytracedAudioEmitter3d;
use super::listener::RaytracedAudioListener3d;
use super::ray_tracing::{
    RaytracedAudioListenerTrace3d, RaytracedAudioRayResponse3d, RaytracedAudioTracing3d,
};
use super::reflection_paths::RaytracedAudioReflectionPaths3d;
use super::response::RaytracedAudioResponse3d;
use super::settings::RaytracedAudioSettings;
use super::surface::RaytracedAudioSurface3d;
use super::volume::RaytracedAudioBaseVolume;
use crate::processed_audio::RaytracedAudioPlayer;
use bevy::audio::Volume;
use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use bevy_raytraced_audio::{
    AcousticResponse, AcousticScene3d, BandGain, Emitter3d, Listener3d, MuffleFilter, Point3,
    ReverbEstimate, SourceRayResponse, Triangle3d,
};

/// Source count at and above which tracing uses the compute task pool.
const PARALLEL_EMITTER_THRESHOLD: usize = 16;

/// Bevy components read or updated for each 3D acoustic emitter.
type EmitterQueryData = (
    Entity,
    &'static GlobalTransform,
    Option<&'static mut AudioSink>,
    Option<&'static mut SpatialAudioSink>,
    Option<&'static mut RaytracedAudioBaseVolume>,
    Option<&'static mut RaytracedAudioResponse3d>,
    Option<&'static mut RaytracedAudioReflectionPaths3d>,
    Option<&'static mut RaytracedAudioRayResponse3d>,
    Option<&'static RaytracedAudioPlayer>,
);

/// Selects entities marked as 3D emitters and their adapter state.
type EmitterQuery<'w, 's> = Query<'w, 's, EmitterQueryData, With<RaytracedAudioEmitter3d>>;

/// Caches unchanged geometry, updates responses, and applies direct-path transmission to sinks.
pub(super) fn update_raytraced_audio(
    commands: ParallelCommands<'_, '_>,
    settings: Res<'_, RaytracedAudioSettings>,
    listeners: Query<'_, '_, &GlobalTransform, With<RaytracedAudioListener3d>>,
    surfaces: Query<'_, '_, (&RaytracedAudioSurface3d, &GlobalTransform)>,
    changed_surfaces: Query<
        '_,
        '_,
        (),
        (
            With<RaytracedAudioSurface3d>,
            Or<(Changed<RaytracedAudioSurface3d>, Changed<GlobalTransform>)>,
        ),
    >,
    surfaces_without_transform: Query<
        '_,
        '_,
        (),
        (With<RaytracedAudioSurface3d>, Without<GlobalTransform>),
    >,
    mut removed_surfaces: RemovedComponents<'_, '_, RaytracedAudioSurface3d>,
    mut scene: Local<'_, AcousticScene3d>,
    mut scene_is_cached: Local<'_, bool>,
    tracing: Res<'_, RaytracedAudioTracing3d>,
    mut listener_trace: ResMut<'_, RaytracedAudioListenerTrace3d>,
    mut trace_scratch: Local<'_, TraceScratch>,
    real_time: Option<Res<'_, Time<Real>>>,
    emitters: EmitterQuery<'_, '_>,
) {
    let listener = unique_listener(&listeners);
    // Pose updates run every frame, independently of the more expensive ray-tracing cadence.
    // Ignore transform scale: direction is in head coordinates, distance stays in world units.
    let head = listeners.single().ok().filter(|_| listener.is_some());
    for (_, transform, _, _, _, _, _, _, player) in &emitters {
        if let Some(params) = player.and_then(RaytracedAudioPlayer::binaural_params) {
            if let Some(head) = head {
                let (_, rotation, translation) = head.to_scale_rotation_translation();
                let local = rotation.inverse() * (transform.translation() - translation);
                params.set_position(local.to_array());
            } else {
                params.clear_listener();
            }
        }
    }
    let scene_changed = refresh_scene(
        listener.is_some(),
        &surfaces,
        &changed_surfaces,
        &surfaces_without_transform,
        &mut removed_surfaces,
        &mut scene,
        &mut scene_is_cached,
    );
    let delta_s = real_time.map_or(f32::INFINITY, |time| time.delta_secs());
    trace_scratch.since_trace_s += delta_s;
    let due = tracing.is_changed()
        || scene_changed
        || trace_scratch.since_trace_s >= tracing.interval_s
        || has_untraced_emitter(&emitters, &trace_scratch.indices)
        || listener_trace.valid != listener.is_some();
    if due {
        trace_scratch.since_trace_s = 0.0;
        trace_listener(
            &scene,
            listener.filter(|_| tracing.enabled),
            *tracing,
            &emitters,
            &mut listener_trace,
            &mut trace_scratch,
        );
    }
    let traced = TracedResponses {
        refresh_legacy: due || !tracing.enabled,
        indices: &trace_scratch.indices,
        trace: listener_trace.trace(),
    };
    process_emitters(
        commands,
        &scene,
        listener,
        settings.occluded_gain,
        &traced,
        emitters,
    );
}

/// Reused emitter positions and entity-to-trace-index lookup.
#[derive(Default)]
pub(super) struct TraceScratch {
    /// Positions of emitters passed to the listener trace.
    sources: Vec<Emitter3d>,
    /// Trace source index for each traced emitter entity.
    indices: EntityHashMap<usize>,
    /// Real seconds since the last trace.
    since_trace_s: f32,
}

/// Returns whether any emitter with a finite position is missing from the last trace.
fn has_untraced_emitter(emitters: &EmitterQuery<'_, '_>, indices: &EntityHashMap<usize>) -> bool {
    emitters.iter().any(|(entity, transform, ..)| {
        let translation = transform.translation();
        translation.is_finite() && !indices.contains_key(&entity)
    })
}

/// Read-only view of the latest listener trace used while updating emitters.
struct TracedResponses<'a> {
    /// Keep image-source queries on the same update cadence as the listener trace.
    refresh_legacy: bool,
    /// Trace source index for each traced emitter entity.
    indices: &'a EntityHashMap<usize>,
    /// Latest valid trace.
    trace: Option<&'a bevy_raytraced_audio::ListenerTrace3d>,
}

impl TracedResponses<'_> {
    /// Returns the traced response for one emitter.
    fn response(&self, entity: Entity) -> Option<SourceRayResponse> {
        let index = *self.indices.get(&entity)?;
        self.trace?.sources().get(index).copied()
    }

    /// Returns the latest listener reverb estimate.
    fn reverb(&self) -> ReverbEstimate {
        self.trace.map_or(
            ReverbEstimate::DRY,
            bevy_raytraced_audio::ListenerTrace3d::reverb,
        )
    }
}

/// Runs one listener ray trace over every emitter with a finite position.
fn trace_listener(
    scene: &AcousticScene3d,
    listener: Option<Listener3d>,
    tracing: RaytracedAudioTracing3d,
    emitters: &EmitterQuery<'_, '_>,
    listener_trace: &mut RaytracedAudioListenerTrace3d,
    scratch: &mut TraceScratch,
) {
    scratch.sources.clear();
    scratch.indices.clear();
    let Some(listener) = listener else {
        listener_trace.valid = false;
        return;
    };
    for (entity, transform, ..) in emitters {
        let translation = transform.translation();
        if let Ok(position) = Point3::try_new(translation.x, translation.y, translation.z) {
            scratch.indices.insert(entity, scratch.sources.len());
            scratch.sources.push(Emitter3d::new(position));
        }
    }
    scene.trace_listener(
        listener,
        &scratch.sources,
        tracing.settings,
        &mut listener_trace.trace,
    );
    listener_trace.valid = true;
}

/// Returns the unique finite 3D listener, disabling tracing for missing or ambiguous listeners.
fn unique_listener(
    listeners: &Query<'_, '_, &GlobalTransform, With<RaytracedAudioListener3d>>,
) -> Option<Listener3d> {
    let mut listener_iter = listeners.iter();
    let listener_transform = listener_iter.next()?;
    if listener_iter.next().is_some() {
        return None;
    }
    let translation = listener_transform.translation();
    Point3::try_new(translation.x, translation.y, translation.z)
        .ok()
        .map(Listener3d::new)
}

/// Clears invalid scenes and rebuilds cached geometry after a surface change.
fn refresh_scene(
    listener_is_valid: bool,
    surfaces: &Query<'_, '_, (&RaytracedAudioSurface3d, &GlobalTransform)>,
    changed_surfaces: &Query<
        '_,
        '_,
        (),
        (
            With<RaytracedAudioSurface3d>,
            Or<(Changed<RaytracedAudioSurface3d>, Changed<GlobalTransform>)>,
        ),
    >,
    surfaces_without_transform: &Query<
        '_,
        '_,
        (),
        (With<RaytracedAudioSurface3d>, Without<GlobalTransform>),
    >,
    removed_surfaces: &mut RemovedComponents<'_, '_, RaytracedAudioSurface3d>,
    scene: &mut Local<'_, AcousticScene3d>,
    scene_is_cached: &mut Local<'_, bool>,
) -> bool {
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
        false
    } else if !**scene_is_cached || surface_changed {
        scene.clear();
        for (surface, transform) in surfaces {
            if let Some(triangle) = transformed_triangle(surface.triangle, transform) {
                scene.add_triangle(triangle);
            }
        }
        **scene_is_cached = true;
        true
    } else {
        false
    }
}

/// Chooses Bevy's parallel query only for large scenes with an initialized compute pool.
fn process_emitters(
    commands: ParallelCommands<'_, '_>,
    scene: &AcousticScene3d,
    listener: Option<Listener3d>,
    occluded_gain: f32,
    traced: &TracedResponses<'_>,
    mut emitters: EmitterQuery<'_, '_>,
) {
    // This upper bound avoids a second full source traversal before dispatch.
    let emitter_count_upper_bound = emitters.iter().size_hint().1.unwrap_or(usize::MAX);
    let compute_pool_available = ComputeTaskPool::try_get().is_some();
    let update_one = |emitter| {
        update_emitter(
            &commands,
            scene,
            listener.as_ref(),
            occluded_gain,
            traced,
            emitter,
        );
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
    scene: &AcousticScene3d,
    listener: Option<&Listener3d>,
    occluded_gain: f32,
    traced: &TracedResponses<'_>,
    (
        entity,
        transform,
        audio_sink,
        spatial_sink,
        base_volume,
        response_state,
        mut reflection_paths,
        ray_response_state,
        player,
    ): QueryItem<'_, '_, EmitterQueryData>,
) {
    let translation = transform.translation();
    let response = Point3::try_new(translation.x, translation.y, translation.z)
        .ok()
        .zip(listener)
        .map(|(position, listener)| {
            if !traced.refresh_legacy
                && let Some(cached) = response_state.as_deref()
            {
                return cached.response;
            }
            let emitter = Emitter3d::new(position);
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

    let ray_response = response.and_then(|_| traced.response(entity));
    if let Some(player) = player {
        let filter = ray_response.map_or(MuffleFilter::CLEAR, SourceRayResponse::filter);
        let reverb = ray_response.map_or(ReverbEstimate::DRY, |_| traced.reverb());
        player.params().set_filter(filter);
        player.params().set_reverb(reverb, player.reverb_send());
    }
    // Processed players apply muffling to samples, so their sink volume stays untouched.
    let visibility_gain = if player.is_some() && ray_response.is_some() {
        Some(1.0)
    } else {
        response.map(|response| visibility_gain(response, ray_response, occluded_gain))
    };
    update_sink_volume(
        commands,
        entity,
        visibility_gain,
        audio_sink,
        spatial_sink,
        base_volume,
    );
    update_response_component(commands, entity, response, response_state);
    update_ray_response_component(commands, entity, ray_response, ray_response_state);
}

/// Chooses the sink-volume scale for an emitter played through Bevy's unprocessed sink.
///
/// Traced emitters use the mean of the filter's low and high gains; untraced emitters use the
/// mean direct transmission. Fully silent paths fall back to the configured occluded gain.
fn visibility_gain(
    response: AcousticResponse,
    ray_response: Option<SourceRayResponse>,
    occluded_gain: f32,
) -> f32 {
    if let Some(ray_response) = ray_response {
        let filter = ray_response.filter();
        let gain = f32::midpoint(filter.gain_lf(), filter.gain_hf());
        return if gain <= 0.0 { occluded_gain } else { gain };
    }
    let direct = response.direct;
    let transmission = direct.gain();
    if direct.is_occluded() && transmission == BandGain::ZERO {
        occluded_gain
    } else {
        (transmission.low() + transmission.mid() + transmission.high()) / 3.0
    }
}

/// Publishes a fresh traced response or removes stale traced state for one emitter.
fn update_ray_response_component(
    commands: &ParallelCommands<'_, '_>,
    entity: Entity,
    response: Option<SourceRayResponse>,
    mut response_state: Option<Mut<'_, RaytracedAudioRayResponse3d>>,
) {
    if let Some(response) = response {
        if let Some(existing) = response_state.as_deref_mut() {
            existing.response = response;
        } else {
            commands.command_scope(|mut commands| {
                commands
                    .entity(entity)
                    .insert(RaytracedAudioRayResponse3d { response });
            });
        }
    } else if response_state.is_some() {
        commands.command_scope(|mut commands| {
            commands
                .entity(entity)
                .remove::<RaytracedAudioRayResponse3d>();
        });
    }
}

/// Applies direct-path gain while preserving the sink's requested base volume.
fn update_sink_volume(
    commands: &ParallelCommands<'_, '_>,
    entity: Entity,
    visibility_gain: Option<f32>,
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
        let output_linear = base_linear * visibility_gain.unwrap_or(1.0);
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
            visibility_gain.is_some(),
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
    mut response_state: Option<Mut<'_, RaytracedAudioResponse3d>>,
) {
    if let Some(response) = response {
        if let Some(existing) = response_state.as_deref_mut() {
            existing.response = response;
        } else {
            commands.command_scope(|mut commands| {
                commands
                    .entity(entity)
                    .insert(RaytracedAudioResponse3d { response });
            });
        }
    } else if response_state.is_some() {
        commands.command_scope(|mut commands| {
            commands.entity(entity).remove::<RaytracedAudioResponse3d>();
        });
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
