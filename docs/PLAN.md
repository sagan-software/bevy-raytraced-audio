# Delivery plan

## Local implementation

The workspace contains a CPU acoustic propagation core, separate Bevy 2D and 3D adapters, four compatibility crates, native and browser examples, a Markdown book, and Nix workflows. The adapters preserve Bevy's audio plugin and player model. They use explicit acoustic surfaces, multiply per-band direct amplitude transmission, and apply the arithmetic mean to existing sink volume. Callers can attach an optional per-emitter component to read first-order reflection paths. The minimal examples draw those paths; the adapter does not apply them to audio samples.

The 2026-10-06 `nix run .#check` passes 108 workspace tests. Coverage reports 99.07% line coverage (2,445 of 2,468), 98.43% region coverage (3,447 of 3,502), and 99.57% function coverage (233 of 234). The quick Criterion run completed all 34 named workloads. At the stress workload of 16 emitters and 32 surfaces, task-pool updates measured 73.98 microseconds in 2D and 78.62 microseconds in 3D; serial updates measured 77.20 microseconds and 81.42 microseconds. At 128/256, the schedule measured 0.445 ms in 2D and 0.621 ms in 3D.

At 256/1,024, the schedule measured 4.684 ms and 5.632 ms. Each measured update fits within 11.11 ms on the local CPU, but rendering and audio output are excluded. The local WebAssembly build produced the gallery, book, and five browser examples. Each route loaded, and its audio graph reached a running Web Audio destination after a click. The bundled WAV now has an explicit Bevy decoder feature and a regression test. The README GIFs record continuous forest, 2D stress, and 3D stress-scene motion.

On Intel UHD Graphics 620 through ANGLE Vulkan, uncapped headless browser animation-frame measurements stayed above 90 per second in both stress scenes. The physical display refresh rate and native window presentation remain unverified.

## Publication gates

- Keep the gallery and book published on GitHub Pages from `main`, then verify the deployment after each release.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Keep the continuous 2D and 3D motion captures in the README current with their matching stress scenes.

## Audio and performance gates

- Add a processed-audio path that can apply material filtering, early reflections, and late reverb without blocking or allocating in the real-time audio callback.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure rendered frames on a physical display and confirm that both stress scenes sustain 90 FPS at its refresh rate.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
