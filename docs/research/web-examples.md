# Browser examples and GitHub Pages

Research date: 2026-10-05.

## Bevy browser support

Bevy 0.19.1 builds for `wasm32-unknown-unknown` with `wasm-bindgen --target web`. Its examples document WebGL2 and browser limitations, and its manifest provides `web` and `webgl2` features. The four examples enable those features for browser builds.

Each page supplies a canvas element. The example selects that canvas and fits it to its parent container so the Bevy view stays inside the gallery layout.

## Audio startup

Bevy uses the browser's Web Audio context. Browsers can keep an `AudioContext` suspended until a user gesture. The page waits for Bevy to create its context, then enables an audio button. A click calls `AudioContext.resume()` on Bevy's context. In local Chromium, all four routes loaded and the contexts changed from `suspended` to `running` after a click. This check does not verify audible output on a physical device.

The examples use Bevy's built-in audio plugin and a generated WAV asset. The pages do not create a second audio engine.

## Build and publish

Cargo.lock pins `wasm-bindgen` 0.2.129. The web build installs that CLI version in ignored `target/` output, compiles four example targets, binds each module separately, copies the audio asset to each route, and builds the Markdown book.

The local `nix run .#web-build` completed and produced the gallery, book, and four browser routes. The GitHub Pages workflow builds one static artifact and deploys it with GitHub's Pages actions. The repository Pages source must be set to GitHub Actions before its first deployment.

The local Chromium check used SwiftShader through Xvfb, where the examples displayed roughly 1–5 FPS. This software-rendered value does not establish hardware browser performance. The 90 FPS target remains unverified.

## Sources

- [Bevy v0.19.1 examples README: WebAssembly and WebGL2](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md)
- [Bevy v0.19.1 feature definitions](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml)
- [Bevy v0.19.1 `Window` canvas properties](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_window/src/window.rs)
- [Bevy v0.19.1 WebAssembly runner source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_winit/src/winit_windows.rs)
- [MDN `AudioContext.resume()`](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/resume)
- [GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
- [Official Nix installer action](https://github.com/cachix/install-nix-action)
