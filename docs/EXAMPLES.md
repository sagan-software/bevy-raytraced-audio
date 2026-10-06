# Examples

The Bevy native and browser examples use Bevy 0.19.1. Each browser route has its own WebAssembly module and WAV asset. Select **Enable spatial audio** after the scene loads; this user gesture resumes Bevy's Web Audio context.

| Example | Scene and controls | What it demonstrates |
| --- | --- | --- |
| `minimal_2d` | One listener, a looping spatial sound, one wall; arrow keys move the listener. | Add the 2D plugin, mark a listener, emitter, and explicit surface, then inspect direct and first-order reflection paths. |
| `minimal_3d` | One listener, a moving sound, a finite wall made from two triangles, and a floor. | Configure Bevy spatial audio, attach response and reflection-path components, and inspect both path types. |
| `stress_2d` | 128 moving emitters, 256 wall segments, one looping sound. | Measure the scheduled update and display a frame-rate readout. |
| `stress_3d` | 128 moving emitters, 256 wall triangles, one looping sound. | Measure the 3D update with the same source and surface counts. |

The minimal examples attach `RaytracedAudioReflectionPaths2d` or
`RaytracedAudioReflectionPaths3d` to the emitter. The adapter fills each
component with the current valid paths in surface insertion order. Call
`paths()` to borrow the slice. Each path exposes its surface index, reflection
point, image source, total distance in meters, and relative energy for each
frequency band. The Bevy examples draw the source-to-reflection and
reflection-to-listener segments in cyan.

Path collection is opt-in. Without the component, the adapter still publishes
the aggregate response and updates direct-path sink volume. Reflection paths
remain geometric response data and do not add audible reflection taps.

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

In local Chromium, all routes loaded and their Web Audio contexts changed from `suspended` to `running` after a click. This verifies browser activation, not audible output on a physical device.

The visible browser check used SwiftShader in Xvfb and reported about 1–5 FPS. That software-rendered result does not establish hardware browser performance. The full 90 FPS target remains unverified.

The stress scenes trace every marked emitter and play one sound voice. The displayed FPS includes Bevy rendering and scheduling. The response reports direct visibility and first-order reflections, but only direct occlusion changes the Bevy sink volume.
