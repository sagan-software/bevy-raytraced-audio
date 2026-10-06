# Delivery plan

## Local implementation

The workspace contains a CPU acoustic propagation core, separate Bevy 2D and 3D adapters, four compatibility crates, native and browser examples, a Markdown book, and Nix workflows. The adapters preserve Bevy's audio plugin and player model. They use explicit acoustic surfaces and update existing sink volume for direct-path occlusion. The core exposes lazy first-order reflection path data in both dimensions; the adapter does not yet apply those paths to audio samples.

The local test, coverage, and Sagan Dylints gates pass. Criterion ran all 24 named workloads. The 256-emitter/1,024-surface schedule exceeds the 90 FPS budget in both dimensions. Native rendering and 90 FPS remain unverified on physical hardware.

## Publication gates

- Push the current implementation to `sagan-software/bevy-raytraced-audio`.
- Verify the gallery and book deployment for the current revision.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Record animated browser examples and preserve accurate hardware and frame-rate caveats.

## Audio and performance gates

- Add a processed-audio path that can apply material filtering, early reflections, and late reverb without blocking or allocating in the real-time audio callback.
- Add browser scenes that visualize and explain the individual 2D and 3D reflection paths while preserving browser audio activation.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure the full application frame on supported hardware and sustain 90 FPS for named workloads.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
