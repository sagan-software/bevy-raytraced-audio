# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. `nix run .#check` passed formatting, strict Clippy, the Bevy 0.17–0.20 release-candidate checks, workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold.

The coverage report counted 1,926 executable lines. It missed 35 lines, for 98.18% line coverage. Regions were 127/127 (100.00%); functions were 1,262/1,275 (98.98%). The remaining uncovered lines are mostly adapter fallback and edge paths. `src/scene3d.rs:147` is also uncovered. These gaps do not affect the measured aggregate threshold, but they need focused branch tests before a release-quality coverage claim.

Run the suite and coverage report with:

```sh
nix run .#test
nix run .#coverage
```

## Benchmark workload coverage

`nix run .#bench -- --quick --noplot` completed all 22 named Criterion scenarios (22/22, 100% workload coverage):

- Six one-source/one-listener queries: open, occluded, and first-order reflection paths in both 2D and 3D.
- Six miss-heavy scene queries: 32, 256, and 1,024 surfaces in both 2D and 3D.
- Ten Bevy schedule workloads: 2D and 3D scenes with 1/0, 1/1, 64/64, 128/256, and 256/1,024 sources/surfaces.

The 128-source/256-surface adapter schedule measured 1.52 ms in 2D and 4.51 ms in 3D. The single-query 1,024-surface workload measured about 65.5 microseconds in 2D and 133 microseconds in 3D. Direct queries currently scan the registered surface list, so query cost grows linearly with surface count. These quick runs are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The 90 FPS target allows 11.11 ms per frame. The 128/256 schedule measurements fit inside that budget on this CPU, but they exclude rendering, audio output, and browser presentation. The visible browser check ran through SwiftShader in Xvfb and reported about 1–5 FPS. Native display validation was unavailable on this host. Overall 90 FPS is unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples. In local Chromium, each route loaded its scene. After a click, Bevy's Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device.

GitHub Actions will build and deploy the same static artifact after the repository Pages source is set to GitHub Actions. The published URL and remote workflow result are release gates.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter changes sink volume for direct occlusion and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering and 90 FPS have not been verified on a physical GPU.
