# Delivery plan

## Local implementation

The 2026-10-10 goal replaces the flat sandbox with the [acoustic village](SANDBOX-AUDIT.md):
CC0 modular houses, working openings, full XYZ floors and stairs, speech, NPC
footsteps, stream/bridge comparisons, and quieter independent sound controls.
The older verification counts below describe earlier checkpoints.

The workspace now includes listener ray tracing, a Bevy processed-audio source,
animated 2D/3D debug rays, an editable sandbox, and focused door, reverb,
ambience, and permeation demos. Nine browser routes are generated with
highlighted source from a single catalogue. CC0 recordings replace the demo
chime. Direct transmission and optional image-source paths remain available.

The 2026-10-09 local run passed 214 Nextest entries (180 tests and 34 benchmark
cases), eight doctests, strict Clippy and private rustdoc, and 52 website tests.
Regression checks cover processed-audio resets, delayed reverb tails, absorbing
rooms, all 18 recordings, and sandbox movement after browser inactivity. These
checks do not replace new coverage or physical audio/performance measurements.
The complete nine-example browser build and all nine route startup/audio-graph
checks also passed, including the embedded sandbox and its compact mobile HUD.

The sandbox now has an F-key first-person view, a directional humanoid,
camera-relative controls, and an explicit speaker-bearing readout. A pinned
rodio 0.22.2 backport fixes inverted panning. Follow-up tests and browser PCM
checks confirm that source screen-side and channel dominance agree; see
[the verification details](TESTING-AND-BENCHMARKS.md).

The following measurements predate that update and describe the earlier
volume-only implementation:

The 2026-10-06 `nix run .#check` passes 108 workspace tests. Coverage reports 99.07% line coverage (2,445 of 2,468), 98.43% region coverage (3,447 of 3,502), and 99.57% function coverage (233 of 234). The quick Criterion run completed all 34 named workloads. At the stress workload of 16 emitters and 32 surfaces, task-pool updates measured 73.98 microseconds in 2D and 78.62 microseconds in 3D; serial updates measured 77.20 microseconds and 81.42 microseconds. At 128/256, the schedule measured 0.445 ms in 2D and 0.621 ms in 3D.

At 256/1,024, the schedule measured 4.684 ms and 5.632 ms. Each measured update fits within 11.11 ms on the local CPU, but rendering and audio output are excluded. The local WebAssembly build produced the gallery, book, and five browser examples. Each route loaded, and its audio graph reached a running Web Audio destination after a click. The bundled WAV now has an explicit Bevy decoder feature and a regression test. The README GIFs record continuous forest, 2D stress, and 3D stress-scene motion.

On Intel UHD Graphics 620 through ANGLE Vulkan, uncapped headless browser animation-frame measurements stayed above 90 per second in both stress scenes. The physical display refresh rate and native window presentation remain unverified.

## Publication gates

- Keep the gallery and book published on GitHub Pages from `main`, then verify the deployment after each release.
- Keep each WebAssembly example reachable by a stable route and confirm its audio context starts after a user click.
- Keep the continuous 2D and 3D motion captures in the README current with their matching stress scenes.

## Audio and performance gates

- Validate the new processed path under real audio-device load. The DSP sample processor is allocation-free; encoded loop restarts are not. Individual delayed reflection taps remain future work.
- Repeat frame-time and coverage measurements for the nine updated examples; do not reuse the older five-scene results as current evidence. Browser startup and audio-graph checks passed on 2026-10-09.
- Prototype optional GPU compute against the CPU reference. Add adapter absence, device failure, device loss, CPU fallback, and parity tests.
- Add incremental scene updates and a measured acceleration structure for large dynamic scenes.
- Measure rendered frames on a physical display and confirm that both stress scenes sustain 90 FPS at its refresh rate.
- Extend audio-output checks to cover impulse response, timing, clipping, and physical browser output.

## Compatibility maintenance

Bevy 0.20 is currently tested at `0.20.0-rc.2`. Replace that release candidate when stable 0.20 is published and rerun the Bevy and Rust matrix. Keep the MSRV at Rust 1.96.1 unless every Bevy version and documented feature set passes an earlier compiler.
