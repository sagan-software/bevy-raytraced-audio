# Delivery plan

## Local implementation

The workspace contains a CPU acoustic propagation core, separate Bevy 2D and 3D adapters, four compatibility crates, native and browser examples, a Markdown book, and Nix workflows. The adapters preserve Bevy's audio plugin and player model. They use explicit acoustic surfaces and update existing sink volume for direct-path occlusion. Callers can attach an optional per-emitter component to read first-order reflection paths. The minimal examples draw those paths; the adapter does not apply them to audio samples.

The local test, coverage, Sagan Dylints, and WebAssembly build gates pass. Criterion measured all 26 named workloads. The example-sized 128-emitter/256-surface schedule measures 0.85 ms in 2D and 1.25 ms in 3D. At 256 emitters and 1,024 surfaces, 2D measures 9.79 ms and 3D measures 13.02 ms; only the 3D result exceeds the 11.11 ms budget. Updated browser capture, native rendering, and rendered 90 FPS remain unverified on physical hardware.

## Publication gates

- Keep the gallery and book deployment on GitHub Pages current with `main`.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Refresh the 2D and 3D animated browser captures after a visible browser session is available.

## Audio and performance gates

- Add a processed-audio path that can apply material filtering, early reflections, and late reverb without blocking or allocating in the real-time audio callback.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure the full application frame on supported hardware and sustain 90 FPS for named workloads.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
