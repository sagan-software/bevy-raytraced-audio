---
stepsCompleted: [1, 2, 3, 4, 5, 6]
inputDocuments: []
workflowType: 'research'
lastStep: 6
research_type: 'technical'
research_topic: 'Bevy GPU compute for ray-traced acoustic propagation'
research_goals: 'Assess optional RenderApp compute, CPU fallback, Bevy 0.17 through 0.20 compatibility, browser support, and upload/readback costs.'
user_name: 'Bill Curry'
date: '2026-10-06'
web_research_enabled: true
source_verification: true
---

# Research Report: Technical

**Date:** 2026-10-06
**Author:** Bill Curry
**Research Type:** Technical

---

## Research Overview

This research evaluates whether Bevy's versioned render sub-app can host an
optional GPU backend for acoustic ray queries while retaining the existing CPU
implementation as the reference and fallback. It separates propagation
computation from audio sample processing because GPU readback and audio callback
timing have different lifecycle and latency requirements.

The scope covers Bevy 0.17 through 0.20, render-device lifecycle, compute
pipeline setup, browser targets, buffer uploads and readback, failure handling,
CPU/GPU parity, and performance crossover. It excludes a claim of hardware
ray-tracing support unless the supported Bevy and wgpu versions expose a
portable and tested acceleration API.

Sources prioritize Bevy and wgpu source or reference documentation, official
release records, and repository compatibility tests. Version-specific APIs
and limitations are cited at the decision they support. Performance claims
remain unverified until measured with representative acoustic scenes and
hardware.

## Technical Research Scope Confirmation

**Research Topic:** Bevy GPU compute for ray-traced acoustic propagation

**Research Goals:** Assess optional RenderApp compute, CPU fallback, Bevy 0.17
through 0.20 compatibility, browser support, and upload/readback costs.

**Technical Research Scope:**

- Architecture analysis of Bevy's render sub-app and GPU resource lifecycle.
- Implementation approaches for optional compute and CPU fallback.
- Technology stack compatibility across Bevy, wgpu, Rust, native, and WebGPU.
- Integration patterns for extraction, compute dispatch, readback, and ECS responses.
- Performance and reliability analysis for uploads, latency, device loss, and fallback.

**Research Methodology:** Current official sources, version-specific source
inspection, direct compatibility tests, and explicit uncertainty where no
portable API or performance result exists.

**Scope Confirmed:** 2026-10-06

---

## Technology Stack Analysis

### Programming Languages

The project is a Rust 2024 workspace with Rust 1.96.1 as its declared minimum.
Rust 1.99.0 became stable on 2026-10-01. The four latest stable minor lines
are 1.96 through 1.99; this repository tests 1.96.1, 1.97.1, 1.98.1, and
1.99.0. The release list confirms these are the latest releases in those four
minor lines ([official Rust release list](https://blog.rust-lang.org/releases/),
[Rust 1.99.0 announcement](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/)).

Rust is the host language for the CPU reference and Bevy integration. WGSL is
the GPU shader language used by Bevy's renderer. Shader code must implement the
same documented acoustic formulas as the Rust core; equivalence is a test
requirement, not an assumed property of translating the algorithms.

### Development Frameworks and Libraries

Bevy 0.19.1 is the latest stable Bevy release in this snapshot. Bevy 0.20.0-rc.2
is the current 0.20 pre-release. The release history lists those versions and
the Bevy news page records the 0.19 stable release ([release history](https://github.com/bevyengine/bevy/releases),
[Bevy 0.19 announcement](https://bevy.org/news/bevy-0-19/)). Bevy 0.17.3,
0.18.1, and 0.19.1 are stable and remain in this project's compatibility
matrix.

The Bevy render crate pins different wgpu major lines: Bevy 0.17.3 uses wgpu
26, Bevy 0.18.1 uses 27, Bevy 0.19.1 uses 29.0.3, and Bevy 0.20.0-rc.2 uses
30. These values come from each tagged
[`bevy_render/Cargo.toml`](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_render/Cargo.toml),
[0.18.1](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_render/Cargo.toml),
[0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_render/Cargo.toml),
and [0.20.0-rc.2](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_render/Cargo.toml).
GPU integration should use the selected Bevy version's render-resource
re-exports inside its versioned compatibility crate. A direct wgpu dependency
could add a second, incompatible wgpu type set.

Bevy provides a render sub-app, `RenderStartup`, and an asynchronous
`GpuReadbackPlugin` in all four selected versions. `Readback` transfers buffer
or texture contents asynchronously and reports completion to the main world
through `ReadbackComplete` ([0.17.3 source](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_render/src/gpu_readback.rs),
[0.18.1 source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_render/src/gpu_readback.rs),
[0.19.1 source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_render/src/gpu_readback.rs),
[0.20.0-rc.2 source](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_render/src/gpu_readback.rs)).
The adapter must keep CPU processing available while GPU responses are pending.

Bevy Solari is prior art for real-time GPU ray-traced lighting, but it does
not implement acoustic propagation or audio DSP ([official Solari example](https://github.com/bevyengine/bevy/blob/main/examples/3d/solari.rs)).
The first GPU backend should implement software BVH traversal in a compute
shader. This research selects no hardware ray-tracing acceleration API.

### Database and Storage

The project has no database or remote data service. Acoustic scenes and the BVH
are in-memory Rust values. A GPU backend would upload a packed snapshot of
geometry and acceleration nodes, then read back compact responses. Whether
that upload and asynchronous readback cost is lower than CPU tracing is unknown
until representative workloads are measured.

### Development Tools and Platforms

Cargo manages workspace features, lockfiles, tests, and benchmark builds. The
Nix flake supplies the Rust toolchain, native libraries, coverage, lint, and
browser-build commands. Criterion benchmarks CPU propagation and Bevy adapter
schedules. The four WebAssembly demos compile the CPU backend and publish as
static routes on GitHub Pages.

Bevy defines `webgl2` and `webgpu` target features in each selected release's
root manifest. WebGL2 and GLES 3.0 do not support compute shaders according to
wgpu's `COMPUTE_SHADERS` downlevel capability, while WebGPU defines compute
pipelines ([wgpu capabilities](https://docs.rs/wgpu/latest/wgpu/struct.DownlevelFlags.html),
[WebGPU specification](https://gpuweb.github.io/gpuweb/)). The GPU feature must
therefore require compute capability on WebAssembly; the CPU path remains
available on WebGL2. Native GPU support also depends on a Bevy render sub-app,
a usable device, and the required storage-buffer and workgroup limits. Keeping
`bevy_render` optional preserves audio-only and CPU-only projects.

### Cloud Infrastructure and Deployment

The current deployment has no server-side compute service. Browser examples run
in the user's page and use Bevy's browser renderer. GPU compute is an optional
local capability. Users without WebGPU compute, without a Bevy render sub-app,
or with an unavailable device must continue on the CPU path.

### Technology Adoption Trends

The verified trend relevant to this project is versioned renderer dependencies,
not a general adoption claim: supported Bevy minor versions bring different
wgpu major versions. Existing per-minor compatibility crates match that
boundary. Confidence is high for release and dependency versions because they
come from official release records and tagged manifests. Performance crossover,
browser compute behavior on target browsers, and CPU/GPU parity remain
unmeasured project questions.

### Technology Stack Decision

Keep the core and CPU adapter free of `bevy_render` and wgpu dependencies. Gate
renderer access behind an optional feature in each versioned compatibility
crate. Preserve the CPU tracer as the reference and fallback when the renderer,
compute pipeline, device, or compute capability is unavailable. Return GPU
results asynchronously and associate each completion with the scene and
emitter generation that produced it so stale results cannot overwrite current
ECS state.

The next research step compares extraction, dispatch, and readback boundaries
before fixing a public backend-selection API.

## Integration Patterns Analysis

### Bevy Application and Render Sub-app

The public adapter lives in the main Bevy app and continues to own listeners,
emitters, surface components, response components, and audio sink updates. GPU
work belongs in Bevy's `RenderApp`, which is created by `RenderPlugin`. The
`ExtractSchedule` transfers read-only simulation data from the main world to
the render world; Bevy documents that render-world work may process a prior
frame while the next frame is simulated ([Bevy `Extract`](https://docs.rs/bevy_render/latest/bevy_render/struct.Extract.html),
[Bevy render module](https://docs.rs/bevy/latest/bevy/render/)). This means a
GPU response is asynchronous state, not a same-frame ECS function result.

The selected Bevy releases expose `RenderApp`, `ExtractSchedule`, and
`RenderStartup` in their tagged `bevy_render` source. A compatibility crate can
register render resources and systems for its exact Bevy minor. If no render
sub-app exists, the public adapter must leave CPU processing enabled.

### Compute Pipeline and Data Exchange

The GPU backend should extract a bounded snapshot of scene and emitter data,
write it to version-specific render resources, dispatch one compute workload,
and publish compact per-emitter responses. Bevy's `PipelineCache` supports
queued compute-pipeline creation and reports when a pipeline is usable; a
queued or failed pipeline cannot be treated as ready ([Bevy `PipelineCache`](https://docs.rs/bevy/latest/bevy/render/render_resource/struct.PipelineCache.html)).
Keep compute dispatch and buffer preparation in the render world so they share
Bevy's device, queue, and render schedule.

### GPU Readback and Audio Sink Boundary

Bevy's `GpuReadbackPlugin` maps readback buffers asynchronously and emits
`ReadbackComplete` when data reaches the main world. The wgpu buffer API also
requires asynchronous mapping before CPU access. On WebAssembly, wgpu documents
additional possible copies between WebAssembly memory and browser/driver
buffers ([Bevy readback source](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_render/src/gpu_readback.rs),
[wgpu `Buffer`](https://docs.rs/wgpu/latest/wgpu/struct.Buffer.html)).

Bevy's audio sink volume is still changed from the main world. GPU computation
therefore cannot directly update the CPU audio callback or guarantee a
same-frame sink change. The readback record must identify its scene generation
and emitter generation. When a completion is older than current ECS state, the
adapter must discard it. This stale-result rule is an inference from the
render/main-world schedule boundary and asynchronous mapping behavior.

One aggregate result buffer per dispatch is preferable to one GPU readback
component per emitter because it bounds readback registration and mapping work.
This is a design candidate; measure the aggregate path against CPU tracing and
Bevy's built-in component readback before committing to buffer size or frame
ring depth. Do not block the audio callback while mapping or waiting for the GPU.

### Platform and Failure Interoperability

The GPU path is eligible only when the application has a Bevy render sub-app,
the device exposes `COMPUTE_SHADERS`, a compute pipeline is ready, and buffer
limits accommodate the packed scene. Wgpu lists WebGL2 and GLES 3.0 as lacking
compute shaders ([wgpu `DownlevelFlags`](https://docs.rs/wgpu/latest/wgpu/struct.DownlevelFlags.html)).
When those conditions fail, the CPU adapter must keep publishing current
responses. On device recovery, `RenderStartup` is designed to initialize GPU
resources again whenever Bevy acquires a new render device ([Bevy render module](https://docs.rs/bevy/latest/bevy/render/)).

There is no HTTP API, broker, microservice, or network protocol in this design.
The integration boundary is Bevy ECS extraction and render resources. WebGPU is
the browser GPU API; WebAssembly buffer readback remains asynchronous and may
copy data. The site host does not participate in runtime audio computation.

### Integration Test Matrix

| Boundary | Required observation |
| --- | --- |
| CPU-only app without `RenderApp` | CPU responses still update sinks. |
| WebGL2 or device without compute support | CPU responses continue without a failed dispatch. |
| Render app with a ready compute pipeline | GPU responses identify their source scene and emitter generations. |
| Pipeline queued, invalid, or unavailable | CPU remains active until GPU processing is ready. |

| Boundary | Required observation |
| --- | --- |
| Readback completion after a newer scene update | The older response is discarded. |
| Emitter despawn before readback completion | The completion does not recreate or mutate the entity. |
| Device loss followed by render-device startup | GPU resources rebuild and CPU output remains valid through recovery. |
| Empty emitter set and limits below one workgroup | Dispatch is skipped and CPU state remains valid. |

The matrix is a proposed conformance set. The repository does not yet contain
the GPU backend or these tests.

The next step defines component boundaries, backend states, and resource
ownership before any GPU implementation begins.

## Architectural Patterns and Design

### System Architecture Pattern

Use a hybrid architecture with the CPU core and main-world Bevy adapters as
the stable reference path. Add optional compute work to each versioned Bevy
render compatibility crate. Keep `bevy_render` out of the core and out of
default CPU-only dependency graphs. The renderer provides its own `RenderApp`,
device, queue, and resource-recovery lifecycle ([Bevy renderer](https://docs.rs/bevy/latest/bevy/render/),
[Bevy `RenderPlugin`](https://docs.rs/bevy_render/latest/bevy_render/struct.RenderPlugin.html)).

Three approaches were considered:

| Approach | Decision | Reason |
| --- | --- | --- |
| CPU-only ECS tracing | Retain as the default and fallback. | Works without a renderer and publishes current-frame sink volume. |
| Independent wgpu device | Reject. | It would create a second device and bypass Bevy's renderer resource and recovery lifecycle. |
| Optional compute in Bevy `RenderApp` | Select for GPU work. | It uses the application's renderer and shares Bevy's render scheduling. |

GPU compute accelerates propagation queries only. The adapter applies returned
responses to Bevy sink volume in the main world. This design does not place
GPU work in Bevy's sample-processing callback, and it does not implement
frequency filters, reflection playback, or late reverb.

### Public API and Crate Boundaries

Keep `RaytracedAudio2dPlugin` and `RaytracedAudio3dPlugin` as the user-facing
entry points. A builder setting should select `Cpu` or `Auto`; `Auto` tries
GPU only when the optional `gpu` feature is compiled and the render device
supports compute. Every failure returns processing to CPU tracing. The
default dependency set remains CPU-only, so brownfield apps do not acquire a
render dependency unless they opt into GPU support.

Each Bevy compatibility crate owns its renderer-specific imports because
supported Bevy releases use wgpu 26, 27, 29, and 30. Do not expose
`RenderDevice`, `wgpu::Device`, or GPU buffer types from the public adapter API.
There is no runtime negotiation between Bevy versions: Cargo features select
one Bevy minor at build time. Host layouts and WGSL are a private, lockstep
schema. A changed layout requires changing both sides and the shader ABI tests
in one release.

### Runtime Modes and Transitions

The public preference vocabulary is closed: `Cpu` and `Auto`. The private
runtime state vocabulary is also closed:

| Runtime state | Work allowed | Transition |
| --- | --- | --- |
| `CpuOnly` | Trace and publish current CPU responses. | Remains CPU-only because the feature is disabled or `Cpu` was selected. |
| `GpuPending` | Keep publishing CPU responses while device or pipeline setup completes. | Move to `GpuActive` after a validated GPU response arrives; move to `CpuFallback` after setup failure. |

| Runtime state | Work allowed | Transition |
| --- | --- | --- |
| `GpuActive` | Submit bounded GPU batches and apply the newest valid completed response. | Return to `CpuFallback` after device loss, invalid output, or readback failure. |
| `CpuFallback(reason)` | Trace and publish current CPU responses. | Retry from `RenderStartup` after Bevy acquires a replacement device. |

`CpuFallback` carries one closed reason: `RenderAppMissing`,
`ComputeUnsupported`, `PipelineFailure`, `DeviceLost`, `BufferLimitExceeded`,
`ReadbackFailure`, `InvalidGpuOutput`, or `GenerationExhausted`. A disabled GPU
feature or explicit `Cpu` preference selects `CpuOnly`. Saturating the bounded
in-flight queue is a per-frame CPU fallback and does not change the backend
state. While the backend is `GpuPending`, CPU responses remain authoritative
until the first validated GPU result is accepted.
Bevy's plugin lifecycle allows a plugin to finish after other plugins are
ready, and its `RenderStartup` schedule reruns for a newly acquired device
([Bevy `Plugin`](https://docs.rs/bevy/latest/bevy/app/trait.Plugin.html),
[render recovery resources](https://docs.rs/bevy/latest/bevy/render/trait.GpuResourceAppExt.html)).

### Acoustic Data and Freshness

Upload the packed BVH and primitive data only when the scene revision changes.
Upload emitter and listener positions for each batch. The shader writes one
direct-path response per input emitter index. Reflection paths remain on CPU in
the first GPU slice and continue to be produced when their optional ECS
components are present.

Define private `SceneRevision(u64)` and `BatchSequence(u64)` counters. Advance
each with `checked_add` before publishing its new value. If either counter is
exhausted, enter `CpuFallback(GenerationExhausted)` for the rest of the app
run; do not wrap and reuse an identity that an old readback could carry.

Use two reusable in-flight readback slots. Each slot retains its batch
sequence, scene revision, and ordered `Entity` mapping. The shader sees only a
compact emitter index; Rust retains the generational entity identity. Accept a
completion only when its scene revision still matches, its sequence is newer
than the last accepted result, each entity still exists, and its age is at
most two main-world updates.

Otherwise discard it. When both slots are busy,
trace the current update on CPU and skip submitting a new GPU batch. This
keeps retained readback memory bounded and avoids waiting on the render queue.

Every returned gain must be finite and within the core's valid gain range.
Output count and emitter indices must match the submitted batch before any
response is applied. Invalid bytes fail the entire batch and trigger CPU
fallback; partial application could mix response generations.

### Cost and Performance Boundaries

Let `E` be emitter count, `S` be surface count, and `K` be the number of
broad-phase surface candidates visited across emitters. The CPU BVH topology
has logarithmic depth because it splits each range at its median; query work is
`O(K)` after endpoint and bounds setup, with `K` near `O(E log S)` only when
scene bounds prune most surfaces. The worst case remains `O(E S)`. A GPU batch
has the same scene-dependent traversal bound, plus `O(S)` geometry upload
when the scene changes, `O(E)` emitter upload per batch, and `O(E)` compact
response readback. Retained GPU and readback storage is `O(S + E)` per scene
plus two `O(E)` in-flight response slots.

Do not claim that GPU is faster from dispatch time alone. Benchmarks must
include geometry upload, emitter upload, compute dispatch, readback completion,
main-world application, scene-change frequency, and browser memory copies.
Choose crossover policy only from representative 2D and 3D workloads on native
and WebGPU targets.

### Failure, Data, and Deployment Architecture

| Failure | Observable result |
| --- | --- |
| GPU feature is not compiled or `Cpu` is selected | CPU computes each response. |
| Render sub-app or compute capability is absent | `CpuFallback` computes each response. |
| Pipeline is queued | CPU computes each response until the pipeline is ready. |
| Pipeline validation or device creation fails | Record a diagnostic reason and use CPU. |

| Failure | Observable result |
| --- | --- |
| Device is lost | Reject in-flight batches and use CPU until `RenderStartup` rebuilds resources. |
| Readback fails or returns invalid bounds | Reject the full batch and use CPU. |
| Two readback slots are occupied | Skip the GPU submission and use CPU for that update. |
| A completion is too old or has a different scene revision | Discard it without changing ECS state. |

No scene data leaves the local application. There is no external service,
database, network API, or user credential. The native app requires only the
optional Bevy render feature for GPU mode. WebAssembly compute requires a
compute-capable WebGPU adapter; WebGL2 and GLES 3.0 use CPU fallback because
wgpu documents that they lack compute shaders ([wgpu compute capability](https://docs.rs/wgpu/latest/wgpu/struct.DownlevelFlags.html)).
The static GitHub Pages host serves the application files and does not perform
GPU work.

### Prototype Invariant Scratchpad

- Legal public preferences: `Cpu`, `Auto`.
- Legal private states: `CpuOnly`, `GpuPending`, `GpuActive`,
  `CpuFallback(reason)`.
- The renderer, device, queue, bind groups, pipeline, and output buffers remain
  private to the selected Bevy compatibility crate.
- Public core points and Bevy transforms store finite `f32` coordinates; CPU
  tracing promotes those values to `f64` for geometry operations. WGSL receives
  the same `f32` inputs, so parity risk comes from intermediate arithmetic and
  intersection boundaries rather than a separate input downcast.
- Every GPU result is untrusted until output length, emitter index, entity
  generation, scene revision, batch sequence, age, gain range, and finiteness
  pass validation.
- Scene revision, batch sequence, reflection energy, and sink output each have
  one authoritative source. Scene revision and batch sequence are private
  checked counters; reflection energy comes from the CPU path; sink volume is
  derived from base volume and the accepted direct gain.
- Malformed shader output is an `InvalidGpuOutput` backend failure. A valid
  result for a stale scene, emitter, sequence, or age is discarded without
  changing ECS state. GPU initialization and device failures change backend
  state and keep CPU processing active.

### Implementation Readiness

The architecture is ready for a GPU prototype, not a release claim. Before
the GPU feature is implemented, the prototype must establish cross-version
render resource setup, exact shader buffer layout, CPU/GPU gain tolerance,
compute dispatch limits, bounded non-panicking readback, and device-loss
recovery. It must also measure whether the 2D and 3D paths beat the parallel
CPU adapter after transfer and freshness costs.

## Implementation Approaches and Technology Adoption

### Technology Adoption Strategy

Keep CPU tracing as the default and add GPU compute as an opt-in Cargo feature.
The plugin preference is a closed `Cpu` or `Auto` enum. A build without the GPU
feature always selects CPU processing; `Auto` selects GPU only after Bevy has a
render sub-app, a compute-capable device, and a ready validated pipeline. CPU
responses remain authoritative while the GPU backend starts and whenever a
batch is pending or rejected.

This staged boundary fits brownfield projects because the current components,
audio sink integration, and surface registration remain in the dimension crate.
The renderer backend stays inside the versioned compatibility crate. Each
consumer still selects one Bevy minor feature at build time, and no GPU type
appears in the public API.

### Development Workflow and Tooling

Use Bevy's compute examples as the renderer integration reference. They extract
main-world resources, queue the compute pipeline in `RenderStartup`, prepare
bind groups, and dispatch from render systems ([compute shader example](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/examples/shader/compute_shader_game_of_life.rs)).
The GPU readback example adds a `Readback` component to a buffer handle and
handles `ReadbackComplete` asynchronously ([GPU readback example](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/examples/shader/gpu_readback.rs)).
Each tagged `GpuReadbackPlugin` implementation calls `expect` on
`map_async` failure and can panic instead of returning the adapter to CPU
processing.

The production path must use a bounded custom readback callback
that reports mapping errors to the main-world backend state; the Bevy example
remains a reference for extraction and staging order. Use Bevy's renderer
re-exports. Do not add another wgpu major dependency to a compatibility crate.

Queue one compute pipeline per dimensional adapter and retain it through
Bevy's render-device lifecycle. `PipelineCache` defers compute pipeline
creation, returns no usable pipeline before success, and exposes the cached
state for failure handling ([Bevy `PipelineCache`](https://docs.rs/bevy_render/latest/bevy_render/render_resource/struct.PipelineCache.html)).

Use a packed immutable surface/BVH snapshot when the scene changes, upload
emitter endpoints per submitted batch, and read back one compact result per
emitter. Keep at most two submitted copies waiting for `map_async`; do not leave
Bevy's repeating `Readback` component attached to a buffer, because it requests
another copy each render frame.

Each staging slot records its batch identity until mapping succeeds or returns
an error. The current CPU scene builds a balanced BVH lazily. The GPU snapshot
must preserve original surface indices because material and entity mappings
depend on insertion order.

### Testing and Quality Assurance

Treat the CPU tracer as the reference implementation. Test each WGSL direct-ray
result against the matching CPU direct response for endpoint-on-surface,
parallel, near-parallel, intersecting, missed, zero-transmission, and
multi-surface cases in both dimensions. Compare distance, three band gains,
and occlusion independently. Set the accepted numeric tolerance from measured
WGSL `f32` arithmetic error against the CPU's `f64` geometry operations; do not
accept GPU results outside that tolerance.

Test state transitions without a GPU by injecting pipeline, device, custom
readback mapping,
queue-saturated, and invalid-output outcomes. Test out-of-order completion,
scene revision change, emitter despawn, two occupied slots, and counter
exhaustion. Keep one hardware integration test for an available compute
adapter; report the adapter name and limits with its result. The existing
coverage target applies to reachable Rust branches and does not claim shader
instruction coverage.

Build and test all four Bevy feature selections on the declared MSRV and the
three newer stable minor lines. A package feature must include `bevy_render`
only in the selected compatibility crate. Browser builds that select WebGL2
must use CPU tracing because the backend lacks compute shader support. A
separate WebGPU build is needed to test browser GPU dispatch and readback.

### Deployment and Runtime Practices

Publish the current five CPU-backed WebAssembly examples on GitHub Pages. Add
a separate WebGPU route only after its GPU build compiles and a browser run
confirms adapter selection, shader dispatch, completion handling, and audible
output. The static host serves files only; compute and audio remain in the
browser. Browser feature detection must select CPU fallback when WebGPU or
compute support is absent.

Keep asynchronous readback out of the audio sink callback. A two-slot ring
bounds retained data and keeps dispatch nonblocking, but it does not guarantee
same-frame response. Measure update-to-result latency, dropped batches, and
CPU fallback frequency alongside throughput. The 90 FPS target applies to the
complete interactive example at 11.11 ms per rendered frame; CPU/GPU query
benchmarks alone do not establish it.

### Resource and Risk Assessment

The CPU path has no renderer cost and stays available for devices without
compute. GPU work adds renderer initialization, geometry packing, shader
compilation, resource limits, readback memory, and asynchronous latency. For
small or frequently changing scenes, transfer cost can exceed the work saved
by compute. For sparse larger scenes, BVH traversal can reduce candidate
intersections; its worst-case query work remains linear in surface count.

The portability risk is numerical: public scene coordinates are finite `f32`
values, CPU geometry operations promote them to `f64`, and portable WGSL uses
`f32` arithmetic. The GPU backend must reject non-finite intermediate values
and near-degenerate cases whose intersection classification differs from the
CPU reference. Shader parity tests need boundary fixtures and a conservative
tolerance measured against the CPU operations. No coordinate normalization or
near-degenerate rejection profile has been implemented; this decision remains
open until measured. GPU acceleration remains experimental until this profile
is implemented and measured.

## Technical Research Recommendations

### Implementation Roadmap

1. Preserve the CPU implementation as the always-available reference and
   fallback.
2. Define and test packed 2D/3D BVH records and exact WGSL storage-buffer
   layouts before wiring dispatch into the adapters.
3. Implement opt-in compute dispatch and bounded asynchronous readback in the
   four Bevy compatibility crates.
4. Gate GPU activation on compute support, pipeline success, representable
   input, output count, finite values, and matching scene/batch identities.
5. Keep CPU reflections enabled in the first compute slice; compare only the
   direct-path fields that the shader calculates.
6. Measure complete CPU and GPU paths on representative native hardware and
   WebGPU browsers, including all transfer and readback costs.
7. Enable a browser GPU gallery route only after GPU/CPU parity and audible
   output are verified in the browser.

### Technology and API Recommendations

Keep `AudioBackendPreference::{Cpu, Auto}` as the closed public preference.
Keep runtime states private: `CpuOnly`, `GpuPending`, `GpuActive`, and
`CpuFallback(reason)`. `Cpu` or a build without the feature selects
`CpuOnly`. `Auto` begins at `GpuPending`, keeps CPU output active, and enters
`GpuActive` only after the first valid result. Any backend error uses CPU
fallback; device recovery re-enters `GpuPending` from Bevy's render startup.

Use private `SceneRevision(u64)` and `BatchSequence(u64)` types with checked
advancement. Overflow enters `CpuFallback(GenerationExhausted)` for the
remainder of the app run. Each output maps an ordered batch of Bevy `Entity`
values and is applied only if its scene revision matches, its sequence exceeds
the last accepted sequence, every entity still exists, and its age is at most
two main-world updates. Reject an entire malformed result batch before
changing any emitter.

Do not advertise hardware ray tracing. The researched path is software BVH
traversal implemented in WGSL compute. Hardware acceleration support has not
been demonstrated for the selected Bevy/wgpu compatibility set.

### Success Measures

- CPU coverage remains above 90% after optional GPU code is added.
- CPU/GPU boundary fixtures meet the documented per-field numeric tolerance.
- Every backend fallback and stale-result condition has direct test evidence.
- The optional GPU feature compiles for Bevy 0.17.3, 0.18.1, 0.19.1, and
  0.20.0-rc.2.
- Representative GPU timings include uploads, dispatch, readback, validation,
  and main-world application.
- The complete 2D and 3D demo runs at or above 90 FPS on the named test
  hardware, with the scene dimensions and backend reported.
- Browser GPU claims include a named browser, WebGPU adapter, audible output
  check, and saved browser recording.

## Research Synthesis

### Executive Summary

Bevy's renderer exposes the compute-pipeline, render-world extraction, device
startup, and asynchronous readback boundaries needed for an optional acoustic
compute backend. The repository already has a CPU reference that caches scene
geometry and applies responses to Bevy audio sinks. The GPU implementation can
remain optional in the four version-specific adapter crates, but it must not
share state or timing with the audio callback.

This research does not establish that GPU compute will outperform the current
parallel CPU path. Every selected Bevy minor uses a different wgpu major, and
WebGL2/GLES 3.0 lack compute shaders. CPU/GPU parity is also constrained by
f64 CPU geometry operations and f32 portable shader arithmetic. The
implementation must reject unsafe GPU input or output and retain CPU responses during pipeline
startup, pending readback, unsupported devices, and all failures.

### Contents

1. Research scope and method
2. Bevy and Rust compatibility
3. Render extraction, compute dispatch, and readback
4. Runtime states, freshness, and failure behavior
5. Implementation workflow and testing
6. Cost, portability, and success measures
7. Sources

### Technical Decisions

The selected integration is compute in Bevy's existing `RenderApp`. A second
wgpu device would duplicate resource lifecycle and is rejected. The first GPU
slice calculates direct-path transmission; the current CPU path continues to
calculate first-order reflections. CPU remains the default and fallback.

The protocol is internal ECS state exchange. There is no network boundary,
database, or external service. Scene records use one private host/shader ABI
per release, and one whole batch is accepted or rejected together. Readback
completions are asynchronous and must pass scene, sequence, entity, age, and
numeric validation before they change ECS state.

### Sources and Verification Limits

- [Bevy 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2 releases](https://github.com/bevyengine/bevy/releases)
- [Bevy 0.20.0-rc.2 compute shader example](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/examples/shader/compute_shader_game_of_life.rs)
- [Bevy 0.20.0-rc.2 GPU readback example](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/examples/shader/gpu_readback.rs)
- [Bevy 0.19.1 `PipelineCache`](https://docs.rs/bevy_render/latest/bevy_render/render_resource/struct.PipelineCache.html)
- [wgpu `Buffer` mapping API](https://docs.rs/wgpu/latest/wgpu/struct.Buffer.html)
- [wgpu compute-shader downlevel support](https://docs.rs/wgpu/latest/wgpu/struct.DownlevelFlags.html)
- [Rust official release announcements](https://blog.rust-lang.org/releases/)
- Versioned `bevy_render` and `gpu_readback.rs` source links in the
  compatibility analysis above.

The report verifies APIs and capability declarations from official sources.
It does not contain a running GPU implementation, a measured CPU/GPU crossover,
or an audible hardware/browser verification. Those remain implementation
gates.

**Research date:** 2026-10-06
**Scope:** Bevy 0.17.3 through 0.20.0-rc.2; Rust 1.96 through 1.99; native and
WebAssembly targets.
