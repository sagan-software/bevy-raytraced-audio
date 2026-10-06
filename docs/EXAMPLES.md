# Examples

The Bevy native and browser examples use Bevy 0.19.1. Each browser route has its own WebAssembly module and WAV asset. Select **Enable spatial audio** after the scene loads to resume Bevy's Web Audio context.

| Example | Scene and controls | What it demonstrates |
| --- | --- | --- |
| `minimal_2d` | One listener; looping sound starts across a partially transmitting wall; arrow keys move the listener | 2D plugin; explicit surface; 20%, 40%, and 60% amplitude by band; mean sink gain |
| `minimal_3d` | One listener; moving sound starts across a two-triangle opaque wall; rendered floor | 3D plugin; response and reflection-path components; clear, wall-crossing, and reflected paths |
| `stress_2d` | 16 moving emitters; 32 wall segments; one looping sound | Scheduled update measurement; frame-rate readout |
| `stress_3d` | 16 moving emitters; 32 wall triangles; one looping sound | 3D update measurement with the same source and surface counts |

The minimal examples attach `RaytracedAudioReflectionPaths2d` or
`RaytracedAudioReflectionPaths3d` to the emitter. The adapter fills each
component with the current valid paths in surface insertion order. Call
`paths()` to borrow the slice. Each path exposes its surface index, reflection
point, image source, total distance in meters, and relative energy for each
frequency band. The Bevy examples draw the source-to-reflection and
reflection-to-listener segments in cyan.

Path collection is opt-in. Without the component, the adapter still publishes
the aggregate response and scales direct-path sink volume. Reflection paths
remain geometric response data and do not add audible reflection taps. A
surface crossing either open reflection leg blocks that path even when the
surface transmits part of the direct signal.

The core crate also provides `cargo run --example trace_2d` inside `nix develop`. It prints the direct response and each visible first-order reflection, including the reflection point, image source, distance, and per-band relative energy. The browser catalogue currently contains the four Bevy scenes in the table.

Each scene has comments describing plugin setup, entity markers, material configuration, and the systems that read responses. Start with the minimal example for the dimension you need, then compare the stress scene's higher entity count.

## Run native examples

```sh
nix run .#demo-2d
nix run .#demo-3d
nix run .#stress-2d
nix run .#stress-3d
```

Native examples require a working display, graphics adapter, and audio output. They build in Nix even when those runtime devices are unavailable.

## Build browser examples

```sh
nix run .#web-build
nix run .#web-serve
```

The local server listens on `http://127.0.0.1:8000`. The [GitHub Pages gallery](https://sagan-software.github.io/bevy-raytraced-audio/) links to all four scenes and the Markdown book. Each Bevy scene builds into its own WebAssembly route. The website build uses Bevy 0.19.1 with WebAssembly and WebGL2 features; the other Bevy versions are tested through native compatibility crates.

## Observed behavior and limits

The local Chromium review loaded all routes and changed each Web Audio context from `suspended` to `running` after a click. This verifies browser activation, not audible output on a physical device. The README GIFs alternate between tutorial and stress routes; they do not record continuous scene motion.

The headless SwiftShader browser run sampled 334 FPS for 2D stress, 4 FPS for 3D stress, 4 FPS for minimal 2D, and 98 FPS for minimal 3D. Those software-rendered samples do not establish hardware browser performance. The full 90 FPS target remains unverified.

The browser stress scenes trace 16 marked emitters against 32 surfaces and play one looping sound. Criterion measures the adapter schedule with 128 emitters and 256 surfaces at 0.402 ms in 2D and 0.460 ms in 3D. At 256 emitters and 1,024 surfaces, it measures 7.510 ms and 4.121 ms. The displayed FPS includes Bevy rendering and scheduling. These schedule measurements do not verify steady rendered 90 FPS. The response reports direct transmission and first-order reflections; the adapter applies direct transmission through sink volume only.
