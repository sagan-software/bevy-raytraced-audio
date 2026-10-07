# Examples

The Bevy native and browser examples use Bevy 0.19.1. Each browser route has its own WebAssembly module and WAV asset. The gallery's forest preview is muted; select `Run the 3D demo` to load the Bevy scene in the page. In the standalone route, click `Enable sound` or click the canvas to resume Bevy's Web Audio context.

The example package enables Bevy's optional `wav` feature for the bundled sound. The `audio_fixture` integration test decodes that WAV with Bevy's audio decoder.

| Example | Scene and controls | What it demonstrates |
| --- | --- | --- |
| `minimal_2d` | One listener; looping sound starts across a partially transmitting wall; arrow keys move the listener | 2D plugin; explicit surface; 20%, 40%, and 60% amplitude by band; mean sink gain |
| `minimal_3d` | One listener; moving sound starts across a two-triangle opaque wall; rendered floor | 3D plugin; response and reflection-path components; clear, wall-crossing, and reflected paths |
| `stress_2d` | 16 moving emitters; 32 wall segments; one looping sound | Scheduled update measurement; frame-rate readout |
| `stress_3d` | 16 moving emitters; 32 wall triangles; one looping sound | 3D update measurement with the same source and surface counts |
| `forest_3d` | Walkable forest; moving chime; stone and brush acoustic panels | Interactive camera and listener; material profiles; direct response and first-order reflection paths |

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

The core crate also provides `cargo run --example trace_2d` inside `nix develop`. It prints the direct response and each visible first-order reflection, including the reflection point, image source, distance, and per-band relative energy. The browser catalogue contains each Bevy scene in the table.

Each scene has comments describing plugin setup, entity markers, material configuration, and the systems that read responses. Start with the minimal example for the dimension you need, then compare the stress scene's higher entity count.

## Run native examples

```sh
nix run .#demo-2d
nix run .#demo-3d
nix run .#stress-2d
nix run .#stress-3d
nix run .#forest-3d
```

Native examples require a working display, graphics adapter, and audio output. They build in Nix even when those runtime devices are unavailable.

## Build browser examples

```sh
nix run .#web-build
nix run .#web-serve
```

The local server listens on `http://127.0.0.1:8000`. The [GitHub Pages gallery](https://sagan-software.github.io/bevy-raytraced-audio/) links to all five scenes and the Markdown book. Each Bevy scene builds into its own WebAssembly route. The website build uses Bevy 0.19.1 with WebAssembly and WebGL2 features; the other Bevy versions are tested through native compatibility crates.

## Observed behavior and limits

The site checks WebGL2 before loading Bevy and waits for canvas sizing and successive WebGL draw calls before it reports that the scene is ready. When WebGL2 is unavailable, the page reports a browser-specific recovery message and keeps audio controls disabled. The loader prevents Bevy's canvas focus from scrolling the audio control out of view. The `Enable sound` button and canvas pointer or keyboard gestures resume browser-suspended audio.

The local Chromium 154 route sweep reached the ready state on all five routes at 1,215 × 700 pixels. Each route created a Bevy audio source and destination connection. Clicking `Enable sound` changed the Web Audio context from `suspended` to `running` and changed the button to `Mute sound`. The browser confirmed output-graph connections; physical speaker output was not tested.

The README includes 12-frame GIFs from each stress scene and a 36-frame forest walk. The stress captures used Intel UHD Graphics 620 through ANGLE Vulkan. The forest capture used Chromium 154 with SwiftShader WebGL2 at 1280 × 720 pixels. Its GIF lasts 4.51 seconds; the paired MP4 lasts 4.5 seconds at 8 FPS. It shows the direct response change from 75%, 65%, and 52% to 100% in each band as the listener crosses the brush screen. One floor-reflection path remains visible. The browser audio context reached `running` after a trusted click; physical speaker output was not tested.

For an uncapped hardware measurement, Chromium ran with `--disable-frame-rate-limit` and `--disable-gpu-vsync`. It collected three consecutive one-second bins for each route at 1,215 × 700 pixels. Every bin exceeded 90 browser animation-frame callbacks per second.

| Route | Callback range per second |
| --- | ---: |
| Forest 3D | 120–127 |
| Minimal 2D | 136–145 |
| Minimal 3D | 93–100 |
| Stress 2D | 127–143 |
| Stress 3D | 119–129 |

The uncapped callback count measures browser frame scheduling. It does not measure physical display presentation or native window output. Normal headless synchronization capped the on-canvas diagnostic near 60 FPS. Firefox 157 headless could not create a WebGL2 context; the route displayed its Firefox recovery steps. The muted gallery preview played in Firefox headless, but Bevy and Web Audio could not start without WebGL2. Normal Firefox with hardware acceleration, native window rendering, and physical audio output remain unverified.

The browser stress scenes trace 16 marked emitters against 32 surfaces and play one looping sound. The 2026-10-06 quick Criterion run measured this schedule at 73.98 microseconds in 2D and 78.62 microseconds in 3D with the task pool. The serial path measured 77.20 microseconds and 81.42 microseconds.

At 128 emitters and 256 surfaces, the schedule measured 0.445 ms and 0.621 ms. At 256 emitters and 1,024 surfaces, it measured 4.684 ms and 5.632 ms. The displayed FPS includes Bevy rendering and scheduling. These schedule measurements do not verify steady rendered 90 FPS.

The response reports direct transmission and first-order reflections. The adapter applies direct transmission through sink volume only.
