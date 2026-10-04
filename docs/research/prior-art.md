# Rust and Bevy prior art

Research date: 2026-10-04.

## AudioNimbus

[AudioNimbus](https://github.com/MaxenceMaire/audionimbus) is an idiomatic Rust wrapper around Steam Audio. Its Bevy feature synchronizes scene and source state and runs simulations on dedicated threads. The repository has a current Bevy example at [`audionimbus/examples/bevy`](https://github.com/MaxenceMaire/audionimbus/tree/master/audionimbus/examples/bevy).

The example uses Bevy for scene entities and transforms, [`bevy_seedling`](https://github.com/CorvusPrudens/bevy_seedling) for Firewheel playback, and a custom Firewheel node to combine direct, reflection, and reverb results. Its README states that simulation runs asynchronously and the audio graph uses the newest output without blocking. This is strong evidence for separating ECS synchronization, acoustic simulation, and audio processing.

AudioNimbus wraps Steam Audio rather than implementing a new solver. Its Bevy path therefore does not satisfy a clean-room CPU/GPU implementation by itself. It is an integration reference, not a dependency commitment.

The archived [AudioNimbus demo repository](https://github.com/MaxenceMaire/audionimbus-demo) points users to the maintained Bevy integration. Its scene demonstrates occlusion/direct sound, reflections, and reverb as separately understandable effects.

## `bevy_steam_audio`

[`bevy_steam_audio`](https://github.com/janhohenheim/bevy_steam_audio) is marked WIP and integrates AudioNimbus, Seedling, and Firewheel. Its API has `SteamAudioPlugin`, listener/pool components, and a scene adapter that collects entities with mesh/material components. The cloned HEAD is `bbcd1f220bdce5565b26ecdc01ba1d8c5f3ec777`, dated 2026-08-06; its workspace manifests select Bevy 0.18, Seedling 0.7, and pre-release plugin crates. Its README compatibility table records earlier Bevy 0.17 and 0.18 releases. Treat that table as historical, not proof of the cloned HEAD's compatibility.

Useful comparisons: explicit listener and sound-pool components; a scene-backend plugin; entity-to-geometry extraction; Seedling as the audio graph; optional gizmo diagnostics; material opt-in.

## Bevy Seedling and Firewheel

[`bevy_seedling`](https://github.com/CorvusPrudens/bevy_seedling) is a Bevy integration for the Firewheel audio engine. The cloned HEAD is `ab4a88b`, the 0.8.0 release dated 2026-08-10. Its [compatibility table](https://github.com/CorvusPrudens/bevy_seedling#bevy-version-compatibility) lists Seedling 0.8 for Bevy 0.19, 0.7 for 0.18, and 0.6 for 0.17. It provides an audio graph, custom audio nodes, effect routing, HRTF/ITD examples, and diagnostics. Its README says projects must disable Bevy's built-in `bevy_audio` feature and select Bevy features manually.

The [custom node example](https://github.com/CorvusPrudens/bevy_seedling/blob/master/examples/custom_node.rs), [reverb tails](https://github.com/CorvusPrudens/bevy_seedling/blob/master/examples/reverb_tails.rs), [spatial HRTF](https://github.com/CorvusPrudens/bevy_seedling/blob/master/examples/spatial_hrtf.rs), and [sends](https://github.com/CorvusPrudens/bevy_seedling/blob/master/examples/sends.rs) help define audio-thread and graph test cases. Seedling is a viable optional adapter, but using it as the only backend conflicts with the drop-in built-in-audio goal.

## Bevy discussion and renderer precedent

- [Bevy discussion 11322](https://github.com/bevyengine/bevy/discussions/11322) proposes ray-traced sound propagation for Bevy. It is a design discussion, not an implementation.
- [Bevy Solari example](https://github.com/bevyengine/bevy/blob/main/examples/3d/solari.rs) is a renderer-side ray-traced lighting precedent. It demonstrates that Bevy has an experimental ray-tracing renderer path, but it does not provide an audio solver or portable GPU audio backend.

## Pure Rust propagation engine: `omg-audio`

[`omg-audio`](https://github.com/JustGoscha/omg-audio) is the closest implementation precedent found. Its README describes a Rust path-traced propagation engine with native and WebAssembly targets, separate simulation and audio clocks, deterministic simulation interfaces, and CPU DSP. The source tree separates core propagation, scene simulation, DSP, applications, and optional integrations. Its package manifest declares Apache-2.0 for `omg-core`; review every package and asset license separately before reusing any material.

The README describes image-source early reflections, stochastic ray tracing, three frequency bands, room portals, wall transmission, diffraction, binaural rendering, and late reverb. These are upstream project claims, not independently validated acoustics results. Its authored demos and tests are useful for shaping test scenes and diagnostic views.

GPU status needs particular care. The cloned snapshot includes `omg-gpu` with `wgpu = "30.0.0"` and a detailed GPU port plan. The README still lists GPU ray workloads and directional late-field work among remaining milestones, while the plan describes work to implement. Treat GPU implementation and measured speedup as unverified until the current source and actual tests establish them. Its wgpu version aligns with the Bevy 0.20 release-candidate family, not Bevy 0.17–0.19's renderer dependency versions.

This project should not silently become a Bevy wrapper around `omg-audio`. The owner should review two valid directions before implementation: keep a new solver and public API independent while using OMG as a research reference, or perform a deeper license, algorithm, API, test, and performance audit before proposing an adapter or code reuse. The current recommendation is independent implementation because the request is for a rewrite from scratch; no OMG code is copied into this repository.

## Other cloned references

[`AudioGroupCologne/wavefront`](https://github.com/AudioGroupCologne/wavefront) is a Rust 2D acoustic wave simulation using a transmission-line-matrix method, with a Bevy 0.14.2 UI. It is useful for understanding where a physical wave model differs from a geometric ray model. It is not a drop-in ray-tracing solver or a current Bevy integration.

[`Firewheel`](https://github.com/BillyDM/firewheel) is a modular Rust audio graph engine with custom nodes and real-time processing constraints. [`bevy_seedling`](https://github.com/CorvusPrudens/bevy_seedling) integrates it with Bevy. Together they offer a plausible optional full-effects output route, but Seedling asks applications to disable Bevy's `bevy_audio` path and use its own player types.

See [local cloned repositories](local-clones.md) for exact paths, revisions, and the purpose of each checkout. Cloned references are not approved dependencies. Before adopting code or assets, review the exact license and maintenance state.
