# Delivery plan

## Local implementation

The workspace now contains a CPU acoustic propagation core, separate Bevy 2D and 3D adapters, four compatibility crates, native and browser examples, a Markdown book, and Nix workflows. The adapters preserve Bevy's audio plugin and player model. They use explicit acoustic surfaces and update existing sink volume for direct-path occlusion.

The local test and coverage gates pass. The Sagan Dylints Quick Start groups pass. Criterion ran all 22 named workloads. The separate Liamc personal-lint workflow still reports Rust documentation and style findings; native rendering and 90 FPS remain unverified on physical hardware.

## Publication gates

- Push the implementation to `sagan-software/bevy-raytraced-audio`.
- Set GitHub Pages to deploy through Actions and verify the gallery and book URLs.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Record animated browser examples and preserve accurate hardware and frame-rate caveats.

## Audio and performance gates

- Add a processed-audio path that can apply material filtering, early reflections, and late reverb without blocking or allocating in the real-time audio callback.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure the full application frame on supported hardware and sustain 90 FPS for named workloads.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
