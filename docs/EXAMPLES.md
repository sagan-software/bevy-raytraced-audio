# Examples

The Bevy native and browser examples use Bevy 0.19.1. Each browser route has its own WebAssembly module and WAV asset. Select **Enable spatial audio** after the scene loads to resume Bevy's Web Audio context.

The example package enables Bevy's optional `wav` feature for the bundled sound. The `audio_fixture` integration test decodes that WAV with Bevy's audio decoder.

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

Local Chromium loaded all four routes. A click changed each browser audio context to `running`. The 3D stress route created 59 `AudioBufferSource` nodes connected to a running 44.1 kHz `AudioDestination`. This verifies that Bevy decoded and scheduled the WAV into the browser output graph. Physical speakers were not tested.

The README GIFs show continuous motion from the 2D and 3D stress scenes. Their captures used the Intel UHD Graphics 620 through ANGLE Vulkan at the browser's normal 60 Hz frame cap.

For an uncapped hardware measurement, Chromium ran with `--disable-frame-rate-limit` and `--disable-gpu-vsync`. The 1,134 × 638 canvas delivered 15 consecutive one-second bins of 131–153 browser animation-frame callbacks in 3D, with a 143.47 mean. The 2D scene delivered 163–174 callbacks per second, with a 168.67 mean. Every measured bin exceeded 90 callbacks per second.

The uncapped callback count measures browser frame scheduling. It does not measure physical display presentation or native window output. With normal headless synchronization, the on-canvas diagnostic showed 55–61 FPS. Native window rendering and physical audio output remain unverified.

The browser stress scenes trace 16 marked emitters against 32 surfaces and play one looping sound. The 2026-10-06 quick Criterion run measured this schedule at 62.15 microseconds in 2D and 62.78 microseconds in 3D with the task pool. The serial path measured 65.38 microseconds and 64.51 microseconds.

At 128 emitters and 256 surfaces, the schedule measured 0.361 ms and 0.427 ms. At 256 emitters and 1,024 surfaces, it measured 3.657 ms and 4.719 ms. The displayed FPS includes Bevy rendering and scheduling. These schedule measurements do not verify steady rendered 90 FPS.

The response reports direct transmission and first-order reflections. The adapter applies direct transmission through sink volume only.
