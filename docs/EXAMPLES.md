# Examples

The native and browser examples use Bevy 0.19.1. Each browser route has its own WebAssembly module and WAV asset. Select **Enable spatial audio** after the scene loads; this user gesture resumes Bevy's Web Audio context.

| Example | Scene and controls | What it demonstrates |
| --- | --- | --- |
| `minimal_2d` | One listener, a looping spatial sound, one wall; arrow keys move the listener. | Add the 2D plugin, mark a listener, emitter, and explicit surface, then read the direct-path response. |
| `minimal_3d` | One listener, a moving sound, a finite wall made from two triangles, and a floor. | Configure Bevy spatial audio, attach a response component, and inspect direct paths. |
| `stress_2d` | 128 moving emitters, 256 wall segments, one looping sound. | Measure the scheduled update and display a frame-rate readout. |
| `stress_3d` | 128 moving emitters, 256 wall triangles, one looping sound. | Measure the 3D update with the same source and surface counts. |

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

The local server listens on `http://127.0.0.1:8000`. The [GitHub Pages gallery](https://sagan-software.github.io/bevy-raytraced-audio/) links to all four scenes and the Markdown book. The website build uses Bevy 0.19.1 with WebAssembly and WebGL2 features; the other Bevy versions are tested through native compatibility crates.

## Observed behavior and limits

In local Chromium, all routes loaded and their Web Audio contexts changed from `suspended` to `running` after a click. This verifies browser activation, not audible output on a physical device.

The visible browser check used SwiftShader in Xvfb and reported about 1–5 FPS. That software-rendered result does not establish hardware browser performance. The full 90 FPS target remains unverified.

The stress scenes trace every marked emitter and play one sound voice. The displayed FPS includes Bevy rendering and scheduling. The response reports direct visibility and first-order reflections, but only direct occlusion changes the Bevy sink volume.
