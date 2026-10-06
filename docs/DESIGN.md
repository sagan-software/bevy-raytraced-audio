# Architecture and API

## Crates

| Crate | Responsibility | Bevy dependency |
| --- | --- | --- |
| `bevy-raytraced-audio` | Validated acoustic materials, geometry, 2D/3D scenes, CPU path queries, and response values | None |
| `bevy-raytraced-audio-2d` | 2D listener, emitter, explicit segment surfaces, plugin, and response components | Select one `bevy_0_17` through `bevy_0_20` feature |
| `bevy-raytraced-audio-3d` | 3D listener, emitter, explicit triangle surfaces, plugin, and response components | Select one `bevy_0_17` through `bevy_0_20` feature |
| `bevy-raytraced-audio-compat-0-17` through `-0-20` | Version-specific Bevy audio sink access | One exact Bevy minor per crate |

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
linear sink volume used when all three accumulated direct-path gains are zero.
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

The response is data, not rendered sound. The Bevy adapter scales the existing
`AudioSink` or `SpatialAudioSink` volume by the arithmetic mean of the three
accumulated amplitude gains. It uses the configured fallback only when all
three gains are zero. The scalar sink cannot apply frequency-dependent gains.
Reflection results are available to the caller but do not change samples.

## Per-emitter reflection paths

The 2D and 3D adapters expose individual first-order paths through optional
`RaytracedAudioReflectionPaths2d` and `RaytracedAudioReflectionPaths3d`
components. Each component owns reusable storage for one emitter, and
`paths()` returns a borrowed slice ordered by surface insertion. The slice
reports valid paths for the current adapter update. Ambiguous listeners,
invalid transforms, and coincident emitter/listener positions clear stale
entries.

The component is absent by default. Existing projects can add it only to
emitters whose path data they consume. The minimal examples use it to draw
reflection polylines. Each candidate path is omitted if another registered
surface crosses either open reflection leg, regardless of that surface's
transmission.

The reflecting surface itself is excluded from those tests. This output does
not change Bevy audio samples. Filtering, reflection playback,
late reverb, and source decoding changes are outside this version.

## Update flow

Each adapter reads marked emitters, listeners, and surfaces after transform
propagation. It clears and rebuilds a local core scene, queries direct paths and
first-order reflections, stores a response component, and updates the built-in
sink volume. The local scene retains collection capacity between frames. Core
queries build and cache a BVH after the surfaces change; the adapter currently
rebuilds that hierarchy each frame because it reconstructs the scene. It does
not use change-tracked surfaces or an incremental refit.

## Backend boundary

The core uses CPU ray queries. It does not require a render plugin or GPU
device. GPU compute and automatic GPU-to-CPU fallback are follow-up work, not
implemented API options. The Nix and browser builds compile the CPU backend.

## Runtime constraints

Audio output and asset loading remain Bevy responsibilities. The adapter does
not own an audio callback and does not add a second sound engine. It changes
volume on existing sink components and leaves playback lifecycle controls with
Bevy.

The current plugin rebuilds a scene from explicit ECS surfaces per update. The
benchmark suite measures core path queries and adapter update schedules. The
128-emitter, 256-surface schedule measured 0.814 ms in 2D and 1.157 ms in 3D on
the local CPU. Those results exclude rendering, audio output, and browser
presentation, so they do not establish rendered 90 FPS.
