# Example catalogue

Use tutorial-style Rust examples with comments that explain how each entity, component, and resource affects the sound. Each example should compile independently, use deterministic generated audio where possible, and state what the user should hear and see.

## First set

| Example | Setup and purpose | Expected demonstration |
| --- | --- | --- |
| `minimal_2d` | Add the 2D plugin, one listener, a tone emitter, and a few explicit planar surfaces | Compare direct sound as an emitter passes behind a wall; render a top-down path view |
| `minimal_3d` | Add the 3D plugin to `DefaultPlugins`, use `SpatialListener`, `AudioPlayer`, `PlaybackSettings`, and a small acoustic room | Move an emitter around a listener and hear direct attenuation and room response |
| `brownfield_bevy_audio` | Start with ordinary Bevy playback, then opt one source and surfaces into the plugin | Show unchanged unmarked playback beside a processed source and document any source-type adapter |
| `materials` | Compare concrete, wood, glass, and open surfaces using explicit acoustic properties | Show and hear material-dependent filtering/transmission; expose current coefficients in diagnostics |
| `moving_door` | Change a door's transform between open and closed states | Show scene invalidation and the resulting direct-path change without rebuilding unrelated geometry |
| `reflections_and_reverb` | Place an impulse or short tone in a room with adjustable dimensions | Compare first reflection delay, reflection directions, and late decay |
| `ambience_and_transmission` | Place rain or room tone outside a room and a source behind a transmissive material | Separate ambient filtering, occlusion, and transmission behavior |
| `debug_visualization` | Draw source/listener markers, current rays, reflection paths, surface IDs, and backend status | Explain the visualization is an inspection aid and pair it with A/B audio |
| `cpu_gpu_compare` | Run matched seeded inputs through CPU and GPU backends | Show selected backend, fallback reason, numerical difference, and elapsed simulation time |
| `many_emitters` | Add many moving emitters and static/dynamic geometry | Show budget controls, simulation age, queue pressure, and measured work scaling |

## Later examples

- `custom_material`: define and register project-specific material coefficients.
- `custom_geometry`: feed geometry that is not a Bevy `Mesh3d` or standard 2D shape.
- `seedling_graph`: route simulation results into a Firewheel graph when the optional adapter is available.
- `no_gpu`: force CPU execution and prove startup without a render plugin or GPU device.
- `asset_loading`: use a custom decoded source with Bevy's asset loader while retaining `PlaybackSettings` semantics.
- `quality_profiles`: tune ray count, update rate, reflection count, and latency budget while displaying their cost.

## Visual and listening review

Review desktop and mobile layout only if an example includes a responsive web page. For native demos, inspect default window size, overlay readability, keyboard controls, and whether the first frame shows useful content. A visual ray display does not substitute for recorded listening comparisons or numerical tests.
