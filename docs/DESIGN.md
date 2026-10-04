# Proposed architecture and API

Status: proposed for owner review. Names and signatures are sketches, not implemented API.

The current implementation recommendation is to create an independent solver and public API. `omg-audio` is close Rust prior art with an Apache-2.0 core, but this repository does not copy its code. The owner may request a separate reuse audit before implementation.

## Domain model

Acoustic simulation estimates how geometry changes sound energy and arrival paths. An audio renderer then applies those estimates to samples. Keep geometry, propagation, audio decoding, DSP, and device output as separate boundaries.

The simulation output should contain the direct path, a bounded set of early reflections, and a compact late-reverb description. It should not contain the source waveform. CPU is the deterministic reference backend. GPU compute is an optional implementation of the same simulation contract.

## Candidate crate layout

| Crate | Responsibility | Bevy dependency |
| --- | --- | --- |
| `bevy_raytraced_audio_core` | Materials, geometry, propagation, simulation results, CPU backend | None |
| `bevy_raytraced_audio_2d` | Planar ECS adapter, 2D geometry and examples | Optional, selected Bevy compatibility adapter |
| `bevy_raytraced_audio_3d` | 3D ECS adapter, mesh conversion and examples | Optional, selected Bevy compatibility adapter |
| `bevy_raytraced_audio_gpu` | Compute backend shared by the 2D and 3D adapters | Optional Bevy render/wgpu integration |
| `bevy_raytraced_audio_seedling` | Optional Firewheel graph integration, if the compatibility spike confirms value | Optional Seedling/Firewheel |

The public 2D and 3D packages remain separate. Keep backend choice as a runtime configuration when practical; avoid requiring both Bevy renderer and GPU crates for CPU-only applications. The Bevy-minor selection and crate dependency layout need a compile spike before they become a Cargo design.

## API sketch

```rust,ignore
app.add_plugins(
    RaytracedAudio3dPlugin::default()
        .with_backend(BackendPolicy::Auto)
        .with_quality(QualityPreset::Balanced),
);
```

An application opts entities into the simulation with components equivalent to:

```rust,ignore
commands.spawn((
    RaytracedAudioEmitter::default(),
    AudioPlayer::new(sound),
    PlaybackSettings::LOOP,
    Transform::from_xyz(2.0, 0.0, -3.0),
));

commands.spawn((
    Mesh3d(wall_mesh),
    RaytracedAudioSurface::new(AcousticMaterial::Concrete),
));
```

The final API must not imply that `AudioPlayer<AudioSource>` receives filtering and reverb merely because it has an emitter component. The compatibility spike decides whether the full-effects path uses a custom `Decodable` asset, a Seedling/Firewheel node, or both.

## Compatibility levels

| Path | Existing Bevy audio plugin | Existing `AudioPlayer<AudioSource>` | Effects available |
| --- | --- | --- | --- |
| Built-in sink integration | Kept | Kept | Direct-path gain and any other controls Bevy exposes through its sink; no general filter/reverb insert |
| Custom `Decodable` source | Kept | Processed sources use a package source type | Depends on the decoder and per-playback parameter mechanism; preserves Bevy's playback host |
| Seedling/Firewheel graph | Replaces Bevy's `bevy_audio` route | Existing players migrate to `SamplePlayer`/samples | Graph-based filters, reflections, and reverb |

Prefer the built-in sink path for a minimal brownfield install when its controls are enough. Use the custom source or graph path for full effects. Do not force all project audio onto a second backend as a side effect of adding the acoustic plugin.

## Closed configuration and runtime states

- Requested acceleration: `Auto`, `Cpu`, or `GpuPreferred`.
- Selected acceleration: `Cpu` or `Gpu`.
- `Auto` and `GpuPreferred` select CPU after adapter discovery, device creation, dispatch, or shader failure. Record the reason in diagnostics.
- CPU remains available when `bevy_render` is absent.
- A strict GPU-required mode is not proposed until there is a concrete user need.

The public material API should validate finite, bounded absorption, scattering, and transmission coefficients. A coefficient range is not a complete physical material model; model choice and frequency bands remain open until research and tests support them.

## Scene and coordinate rules

- 2D mode traces in XY. Transform z is render ordering and does not change propagation in the first design.
- 3D mode uses Bevy world-space positions after `GlobalTransform` application.
- Only entities with an acoustic-surface component participate in the first release. This prevents the plugin from guessing which render meshes are acoustically relevant.
- The 3D adapter converts supported mesh positions and indices into core geometry. It reports unsupported or malformed mesh attributes and does not silently invent faces.
- Changed transforms or mesh handles mark geometry dirty. Rebuild cost and incremental refit behavior require benchmark evidence.
- Surface material comes from the explicit acoustic component. A Bevy render material has no acoustic meaning by default.
- Existing `SpatialListener` can provide listener position/orientation in the built-in audio path. The simulation's listener cardinality and multi-listener support must be explicit before implementation.

## Threading and data flow

Bevy systems snapshot positions, geometry changes, and configuration on the ECS thread. A bounded worker queue transfers simulation requests to CPU or GPU work. A latest-complete immutable result snapshot returns to ECS and the audio processor. Audio processing reads a stable snapshot without waiting for simulation completion.

The real-time audio callback must perform no blocking wait, lock acquisition, unbounded queue growth, or allocation. It may consume bounded immutable state and process a fixed audio block. Simulation output may be stale by one or more frames; expose its age in diagnostics.

## Complexity expectations

For `N` geometry primitives and `R` traced samples per simulation update, an acceleration structure should aim for `O(N log N)` construction and `O(N)` storage. A BVH query is expected to approach `O(log N)` traversal on ordinary scenes, while the worst case remains `O(N)` per ray. Total query work is therefore average-case near `O(R log N)` and worst-case `O(RN)`. Record geometry count, ray count, and update time in benchmarks.

For steady geometry, reuse the acceleration structure. For dynamic geometry, measure rebuilds against refits before selecting an update policy. Do not add scene complexity, buffering, or allocations without workload evidence.
