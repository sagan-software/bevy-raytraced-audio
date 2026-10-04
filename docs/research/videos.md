# Videos and demos research

Research date: 2026-10-04. Video timestamps and descriptions come from the public video pages. Videos are product demonstrations, not algorithm specifications.

## Vercidium Audio

### [A First Look At Raytraced Audio](https://www.youtube.com/watch?v=u6EuAUjq92k)

Vercidium published this video on 2025-04-05. Its chapters cover perception, the problem, ray tracing, echo, weather, sound permeation, visualization for deaf players, requirements, optimization, and code. It presents an earlier voxel-based approach and explains intended player-facing effects.

Use it to define what a demo should make audible and visible. Treat its voxel representation and performance statements as historical because the later video describes a different implementation.

### [I Built Raytraced Audio for Godot](https://www.youtube.com/watch?v=A6bPUXTlic8)

Vercidium published this video on 2026-09-01. Its chapters cover worlds, ray tracing, sounds, materials, ambience, muffling, and visualization. The description says the newer implementation changed its energy/material model and optimizations and removed voxels. It describes OpenAL Soft or FMOD plugins for Godot and an experimental Unreal Engine 5.7 plugin.

Use the categories and chapter sequence to shape examples: scene and material setup, propagation, ambience/muffling, and visualization. Do not infer the proprietary SDK's exact solver, coefficient ranges, or license terms from the video.

## Rust/Bevy prior art

### [AudioNimbus walkthrough](https://www.youtube.com/watch?v=zlhW1maG0Is)

AudioNimbus links this video from its repository README. Pair the video with the [Bevy example README](https://github.com/MaxenceMaire/audionimbus/blob/master/audionimbus/examples/bevy/README.md), which documents direct sound, HRTF, occlusion, early reflections, and a reverb tail, plus its nonblocking Firewheel graph integration.

### [Steam Audio in Unity: Four simple tests](https://www.youtube.com/watch?v=gd64iEGyY0s)

This third-party demonstration isolates HRTF, reflections, occlusion, and sound propagation. Use it only as a listening/visualization reference; rely on Steam Audio's official documentation for API and algorithm claims.

## Demo design lessons

- Demonstrate one effect at a time before combining them.
- Keep a dry/processed A/B switch and show the active backend.
- Visualize rays, surface material, direct visibility, and reflection paths without making the overlay the proof of audible behavior.
- Include short repeating tones or impulses so onset time, reflection delay, filter change, and decay can be compared.
- Use a 2D overhead scene and a 3D room with matched material labels, but explain that the solvers and geometry differ.
