# Architecture and API

## Crates

| Crate                                              | Responsibility                                                                              | Bevy dependency                                    |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `bevy-raytraced-audio`                             | Validated acoustic materials, geometry, 2D/3D scenes, CPU path queries, and response values | None                                               |
| `bevy-raytraced-audio-2d`                          | 2D listener, emitter, explicit segment surfaces, plugin, and response components            | Select one `bevy_0_17` through `bevy_0_20` feature |
| `bevy-raytraced-audio-3d`                          | 3D listener, emitter, explicit triangle surfaces, plugin, and response components           | Select one `bevy_0_17` through `bevy_0_20` feature |
| `bevy-raytraced-audio-compat-0-17` through `-0-20` | Version-specific Bevy audio sink and processed decoder integration                          | One exact Bevy minor per crate                     |

The 2D and 3D crates are separate workspace members. One project uses one Bevy
minor feature on each adapter. The core has no Bevy, renderer, or audio-device
dependency.

## Drop-in setup

Keep Bevy's existing `AudioPlugin`, `AudioPlayer`, asset handle, and
`PlaybackSettings`. Add the dimension-specific plugin, then mark the listener,
selected `AudioPlayer` entity, and acoustic surfaces. Unmarked audio is not
changed. Surface geometry is explicit; the adapter does not guess which render
meshes are acoustic barriers.

```rust,ignore
app.add_plugins(RaytracedAudio2dPlugin::default());

commands.spawn((
    RaytracedAudioEmitter2d,
    AudioPlayer::new(sound),
    PlaybackSettings::LOOP.with_spatial(true),
    Transform::from_xyz(2.0, 0.0, 0.0),
));
```

`RaytracedAudio2dPlugin::with_occluded_gain` and its 3D equivalent select the
linear sink fallback for fully silent traced or direct-only paths.
The default is zero. Invalid gains return `GeometryError` before plugin
construction.

## Propagation model

The core estimates direct transmission and first-order image-source
reflections. It accepts 2D line segments and 3D triangles with validated
coordinates and acoustic materials. The 2D adapter projects positions onto XY;
z is render ordering only. The 3D adapter uses transformed world positions.
Surface materials are explicit and do not inherit Bevy rendering materials.

For each frequency band, `A` is the fraction of incident energy absorbed and
`T` is the amplitude fraction transmitted. Both values are dimensionless and
must satisfy `A + T² <= 1`. The remaining reflected energy fraction is
`R = 1 - A - T²`. A direct ray multiplies each crossed segment or triangle's
amplitude transmission by band. The path is marked occluded if any band gain
differs from one.

`AcousticMaterial::default()` has zero absorption and zero transmission. It
therefore blocks direct transmission and leaves all incident energy available
for reflection. A fully transmitting surface has zero absorption and unit
transmission, so it contributes no reflected energy.

These direct/image-source responses remain available as geometry data. The
adapter also fires a configurable set of rays from the listener, following
specular and diffuse bounces, source visibility, echo returns, escaped paths,
and per-band wall transmission. It derives a low/high-frequency muffle filter,
a room reverb estimate, and outdoor ambience direction from these samples.

Use `RaytracedAudioPlayer` instead of `AudioPlayer` for audible filtering and
reverb. Its per-entity asset decodes through a smoothed two-band filter and a
Schroeder network with traced pre-delay, wet gain, and decay time. Bevy still
owns the audio device, sinks, spatialization, and playback settings. Loops restart
inside the processed decoder so parameter changes remain audible after the
first repetition. One-shots retain their reverb tails, bounded to six seconds.

Plain `AudioPlayer` emitters use the mean of the traced low/high gains for sink
volume. `without_ray_tracing()` restores direct-only behavior using the mean
of all three direct transmission bands. The configured occluded gain applies
to fully silent scalar paths. Disabling tracing or losing a valid listener
clears stale DSP filtering and reverb before direct-only or unmodified playback.
The processing bypass remains under the caller's control.

## Per-emitter reflection paths

The 2D and 3D adapters expose individual first-order paths through optional
`RaytracedAudioReflectionPaths2d` and `RaytracedAudioReflectionPaths3d`
components. Each component owns reusable storage for one emitter, and
`paths()` returns a borrowed slice ordered by surface insertion. The slice
reports valid paths for the current adapter update. Ambiguous listeners,
invalid transforms, and coincident emitter/listener positions clear stale
entries.

The component is absent by default. Existing projects can add it only to
emitters whose path data they consume. The forest example uses it to draw
reflection polylines. Each candidate path is omitted if another registered
surface crosses either open reflection leg, regardless of that surface's
transmission.

The reflecting surface itself is excluded from those tests. This output does
not create individual audible reflection taps. The processed player uses the
listener trace's aggregate reverb estimate, not a separate sample delay per
image-source path.

## Update flow

Each adapter reads marked emitters, listeners, and surfaces after transform
propagation. It retains a local core scene and rebuilds that scene when a
surface component or transform changes, a surface is added or removed, a
surface lacks a transform, or the listener becomes invalid. Core queries build
and cache a BVH after the scene changes. The adapter does not incrementally
refit the hierarchy. Listener tracing runs at up to 30 Hz by default, with
immediate retracing after configuration or surface changes and for new emitters.
Debug drawing replays a recorded trace at a user-selected visual speed; it does
not slow the audio stream or modify acoustic travel times.

## Backend boundary

The core uses CPU ray queries. It does not require a render plugin or GPU
device. Both dimension plugins expose `with_backend_preference`. The default
`AudioBackendPreference::Cpu` selects CPU tracing. `Auto` currently logs that
the GPU backend is unavailable, then uses CPU tracing. GPU dispatch and
automatic GPU-to-CPU fallback are not implemented.

The Nix and browser builds compile the CPU backend.

## Runtime constraints

Audio output and asset loading remain Bevy responsibilities. The core DSP
processor preallocates its delay lines and reads atomic parameters without
locking or allocating in `process_sample`. This guarantee does not cover Bevy's
codec decoder or the allocation involved in restarting an encoded loop. The
adapter registers a Bevy `Decodable` source and leaves device and sink lifecycle
controls with Bevy.

The current plugin retains a scene built from explicit ECS surfaces between
surface changes. It processes fewer than 16 emitters sequentially. At 16 or
more emitters, it uses
`Query::par_iter_mut` when `ComputeTaskPool::try_get` finds an initialized
compute pool; otherwise, it processes emitters sequentially. Each emitter owns
its mutable response and sink components, while every trace reads the shared
cached scene. Bevy 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2 expose this parallel
query API in their ECS source ([0.17.3](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_ecs/src/system/query.rs),
[0.18.1](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_ecs/src/system/query.rs),
[0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/src/system/query.rs),
[0.20.0-rc.2](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_ecs/src/system/query.rs)).

The 2026-10-06 quick benchmark, before listener tracing and DSP were added, measured the 16-emitter/32-surface stress
schedule at 77.20 microseconds in 2D and 81.42 microseconds in 3D on the
serial path. The task-pool path measured 73.98 microseconds and 78.62
microseconds. At 128 emitters and 256 surfaces, it measured 0.445 ms in 2D and
0.621 ms in 3D. At 256 emitters and 1,024 surfaces, it measured 4.684 ms and
5.632 ms. These results exclude rendering, audio output, and browser
presentation, so they do not establish rendered 90 FPS.
