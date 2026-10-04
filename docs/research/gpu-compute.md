# GPU compute research

Research date: 2026-10-04.

The [WebGPU specification](https://gpuweb.github.io/gpuweb/) defines general GPU compute and rendering APIs. [`wgpu::ComputePipeline`](https://docs.rs/wgpu/latest/wgpu/struct.ComputePipeline.html) provides the Rust abstraction for dispatching compute shaders. These are sufficient references for a portable compute-shader prototype, but they do not establish that the project has a portable hardware ray-tracing acceleration API.

Proposed scope: GPU ray traversal in compute shaders over project-owned acceleration data, initially BVH nodes and primitive arrays. CPU remains the reference and fallback. Hardware RT extensions are outside the first design until Bevy and wgpu support and test the same behavior across target systems.

Bevy's [Solari example](https://github.com/bevyengine/bevy/blob/main/examples/3d/solari.rs) is an engine renderer precedent for GPU ray-traced lighting. It is not an audio propagation implementation and does not remove the need to profile a separate acoustic workload.

The API spike must verify access to Bevy's render device from a plugin without adding `bevy_render` to CPU-only builds. It must also measure upload and synchronization costs, device loss, shader compilation errors, workgroup limits, dispatch size, and the crossover point where GPU execution beats CPU for representative scenes.
