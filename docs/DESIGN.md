# Architecture and API

## Crates

| Crate | Responsibility | Bevy dependency |
| --- | --- | --- |
| `bevy-raytraced-audio` | Validated acoustic materials, geometry, 2D/3D scenes, CPU path queries, and response values. | None |
| `bevy-raytraced-audio-2d` | 2D listener, emitter, explicit segment surfaces, plugin, and response components. | Select one `bevy_0_17` through `bevy_0_20` feature. |
| `bevy-raytraced-audio-3d` | 3D listener, emitter, explicit triangle surfaces, plugin, and response components. | Select one `bevy_0_17` through `bevy_0_20` feature. |
| `bevy-raytraced-audio-compat-0-17` through `-0-20` | Version-specific Bevy audio sink access. | One exact Bevy minor per crate. |

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
linear sink volume used when a direct path is occluded. The default is zero.
Invalid gains return `GeometryError` before plugin construction.

## Propagation model

The core estimates direct visibility and first-order image-source reflections.
It accepts 2D line segments and 3D triangles with validated coordinates and
acoustic materials. The 2D adapter projects positions onto XY; z is render
ordering only. The 3D adapter uses transformed world positions. Surface
materials are explicit and do not inherit Bevy rendering materials.

The response is data, not rendered sound. The Bevy adapter uses direct-path
occlusion to scale the existing `AudioSink` or `SpatialAudioSink` volume.
Reflection results are available to the caller but do not change samples.
Filtering, reflection playback, late reverb, and source decoding changes are
outside this version.

## Update flow

Each adapter reads marked emitters, listeners, and surfaces after transform
propagation. It builds the corresponding core scene, queries direct paths and
first-order reflections, stores a response component, and updates the built-in
sink volume for direct occlusion. The current scene construction does not use an
incremental BVH or scene refit.

## Backend boundary

The core uses CPU ray queries. It does not require a render plugin or GPU
device. GPU compute and automatic GPU-to-CPU fallback are follow-up work, not
implemented API options. The Nix and browser builds compile the CPU backend.

## Runtime constraints

Audio output and asset loading remain Bevy responsibilities. The adapter does
not own an audio callback and does not add a second sound engine. It changes
volume on existing sink components and leaves playback lifecycle controls with
Bevy.

The current plugin rebuilds a small scene from explicit ECS surfaces per update.
The benchmark suite measures core path queries and adapter update schedules;
the 90 FPS target is not guaranteed across renderers, devices, or browsers.
