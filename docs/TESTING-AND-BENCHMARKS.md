# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 99 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#dylint` passed. The scoped personal Rust lint for the changed core package passed; the full personal-lint dispatcher reported repository-wide documentation diagnostics and could not find ALSA for its Bevy 0.18 phase. Its Markdown phase also reported four paragraph-length findings in unchanged documents.

The current coverage report counted 2,194 lines and missed 25, for 98.86% line coverage. It counted 3,290 regions and missed 61, for 98.15% region coverage. It executed 209 of 210 functions, for 99.52% function coverage. Every added material-transmission function and BVH candidate-traversal path is covered.

Remaining misses include error exits in successful test-fixture construction and two geometry branches. A valid 3D image-source candidate cannot reach the out-of-range ray-parameter branch because mirror construction places the plane crossing between the image source and listener. Current fixtures do not reach the 2D near-parallel branch. Unit and public integration tests exercise the BVH broad phase and cached 3D triangle paths.

Run the suite and coverage report with:

```sh
nix run .#test
nix run .#coverage
```

## Benchmark workload coverage

`nix run .#bench -- --quick --noplot` completed all 26 named Criterion scenarios (26/26, 100% workload coverage):

- Six one-source/one-listener traces: open, occluded, and first-order reflection paths in both 2D and 3D.
- Four first-order reflection workloads: two lazy iterator queries and two queries that write into reused path-output storage.
- Six miss-heavy scene queries: 32, 256, and 1,024 surfaces in both 2D and 3D.
- Ten Bevy schedule workloads: 2D and 3D scenes with 1/0, 1/1, 64/64, 128/256, and 256/1,024 sources/surfaces.

The reusable path-output query measured 119 ns in 2D and 137 ns in 3D. The 128-source/256-surface adapter schedule measured 0.842 ms in 2D and 1.269 ms in 3D. The 256-source/1,024-surface schedule measured 9.939 ms in 2D and 13.444 ms in 3D. The single-query 1,024-surface miss measured 42.541 microseconds in 2D and 47.542 microseconds in 3D. These quick runs are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The 90 FPS target allows 11.11 ms per frame. Both 128/256 schedule measurements and the 2D 256/1,024 schedule fit inside that budget on this CPU. The 3D 256/1,024 schedule measures 13.444 ms and exceeds it. These schedules exclude rendering, audio output, and browser presentation.

The earlier SwiftShader check in Xvfb reported about 1–5 FPS. Native display validation is unavailable on this host. Overall rendered 90 FPS is unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the material-transmission example positions. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

An earlier local Chromium run loaded each route. After a click, Bevy's Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device. The example emitter positions changed in this revision; no browser session was available for visual or audio playback review, and the GIFs have not been refreshed.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter scales sink volume by the mean direct amplitude gain and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering and 90 FPS have not been verified on a physical GPU.
