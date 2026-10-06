# GPU compute research

Research date: 2026-10-06.

The project does not yet implement a GPU backend. The current 2D and 3D
adapters use the CPU tracer and keep it available without a Bevy renderer.
The [technical GPU report](technical-bevy-gpu-compute-acoustics-2026-10-06.md)
records the Bevy render integration, compatibility sources, proposed state
machine, numerical limits, testing contract, and implementation roadmap.

Use an optional Bevy `RenderApp` compute backend over project-owned BVH data.
Keep CPU processing as the default and fallback. The proposed first GPU slice
computes direct-path transmission; first-order reflections continue on CPU.
WebGL2 uses CPU fallback because wgpu documents that WebGL2 and GLES3 do not
support compute shaders.

The GPU design remains experimental until it passes CPU parity, GPU resource
recovery, readback freshness, and full transfer-inclusive performance checks.
