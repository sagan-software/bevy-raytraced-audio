# Browser examples and GitHub Pages

Research date: 2026-10-06.

## Bevy browser support

Bevy 0.19.1 builds for `wasm32-unknown-unknown` with `wasm-bindgen --target web`. Its examples document WebGL2 and browser limitations, and its manifest provides `web` and `webgl2` features. The five browser examples enable those features for browser builds.

Each page supplies a canvas element. The example selects that canvas and fits it to its parent container so the Bevy view stays inside the gallery layout.

## Audio startup

Bevy uses the browser's Web Audio context. Browsers can keep an `AudioContext` suspended until a user gesture. The page waits for Bevy to create its context, then enables an audio button. A click calls `AudioContext.resume()` on Bevy's context. In local Chromium, all five routes loaded and the contexts changed from `suspended` to `running` after a click. This check does not verify audible output on a physical device.

The examples use Bevy's built-in audio plugin and a generated WAV asset. The example manifest enables Bevy's optional `wav` feature; the default audio feature set does not decode this fixture. The `audio_fixture` integration test decodes it through Bevy's `Decodable` implementation.

## Build and publish

Cargo.lock pins `wasm-bindgen` 0.2.129. The web build installs that CLI version in ignored `target/` output, compiles five example targets, binds each module separately, copies the audio asset to each route, and builds the Markdown book.

The local `nix run .#web-build` completed and produced the gallery, book, and five browser routes. The GitHub Pages workflow builds one static artifact and deploys it with GitHub's Pages actions. The published gallery links to each route and the book.

The browser stress routes use 16 emitters and 32 surfaces. Local Chromium loaded all five routes, and each audio context changed to `running` after a click. A DevTools Web Audio trace of the 3D stress route showed 59 `AudioBufferSource` nodes connected to a running `AudioDestination` at 44.1 kHz. This verifies the browser output graph, but not physical speaker output.

The gallery and stress pages had no horizontal overflow at a 390-pixel viewport. The README contains 12 sampled frames from each stress route and a 24-frame forest walk, recorded on Intel UHD Graphics 620 through ANGLE Vulkan.

For the uncapped browser frame measurement, Chromium disabled frame limiting and GPU VSync. At a 1,215 × 700 canvas size, 15 one-second bins ranged from 196 to 277 browser animation-frame callbacks per second across the five routes. Every bin exceeded 90 callbacks per second; physical display presentation remains unverified. The route-specific ranges are recorded in [testing and benchmark results](../TESTING-AND-BENCHMARKS.md).

## Sources

- [Bevy v0.19.1 examples README: WebAssembly and WebGL2](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md)
- [Bevy v0.19.1 feature definitions](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml)
- [Bevy v0.19.1 `Window` canvas properties](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_window/src/window.rs)
- [Bevy v0.19.1 WebAssembly runner source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_winit/src/winit_windows.rs)
- [MDN `AudioContext.resume()`](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/resume)
- [GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
- [Official Nix installer action](https://github.com/cachix/install-nix-action)
