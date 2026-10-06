# Delivery plan

## Local implementation

The workspace contains a CPU acoustic propagation core, separate Bevy 2D and 3D adapters, four compatibility crates, native and browser examples, a Markdown book, and Nix workflows. The adapters preserve Bevy's audio plugin and player model. They use explicit acoustic surfaces, multiply per-band direct amplitude transmission, and apply the arithmetic mean to existing sink volume. Callers can attach an optional per-emitter component to read first-order reflection paths. The minimal examples draw those paths; the adapter does not apply them to audio samples.

The 2026-10-06 `nix run .#check` passes 107 workspace tests. Coverage reports 99.07% line coverage (2,445 of 2,468), 98.43% region coverage (3,447 of 3,502), and 99.57% function coverage (233 of 234). The quick Criterion run completed all 26 named workloads. At 128 emitters and 256 surfaces, the adapter schedule measured 0.381 ms in 2D and 0.436 ms in 3D. At 256 emitters and 1,024 surfaces, it measured 4.036 ms in 2D and 4.281 ms in 3D. Each update fits within 11.11 ms on the local CPU, but rendering and audio output are excluded. The local WebAssembly build produced the gallery, book, and all four browser examples. The four published routes previously loaded and their audio contexts resumed after a click. README GIFs compare routes; they do not show continuous motion. Physical-device audio and steady rendered 90 FPS remain unverified.

## Publication gates

- Publish the gallery and book on GitHub Pages from `main`, then verify the deployment after each release.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Record continuous 2D and 3D motion captures when a browser capture path can retain animation frames; the current GIFs compare tutorial and stress routes.

## Audio and performance gates

- Add a processed-audio path that can apply material filtering, early reflections, and late reverb without blocking or allocating in the real-time audio callback.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure the full application frame on supported hardware and sustain 90 FPS for named workloads.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
