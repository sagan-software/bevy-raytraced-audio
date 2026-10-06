# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-05, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#dylint` also passed with the Sagan Dylints Quick Start configuration.

The coverage report counted 1,516 executable lines and missed 14, for 99.08% line coverage. It counted 2,290 regions and missed 36, for 98.43% region coverage. All 159 functions executed. Two core geometry locations remain uncovered: the 2D non-collinear parallel case is rejected by the new bounds check first, and a valid mirrored 3D source ray cannot intersect its plane outside the source-listener segment. Six uncovered branches are `?` error paths inside tests that construct valid adapter surfaces. New reflection-path code has full line and function coverage.

Run the suite and coverage report with:

```sh
nix run .#test
nix run .#coverage
```

## Benchmark workload coverage

`nix run .#bench -- --quick --noplot` completed all 24 named Criterion scenarios (24/24, 100% workload coverage):

- Six one-source/one-listener traces: open, occluded, and first-order reflection paths in both 2D and 3D.
- Two lazy first-order reflection iterator workloads, one in each dimension.
- Six miss-heavy scene queries: 32, 256, and 1,024 surfaces in both 2D and 3D.
- Ten Bevy schedule workloads: 2D and 3D scenes with 1/0, 1/1, 64/64, 128/256, and 256/1,024 sources/surfaces.

The 128-source/256-surface adapter schedule measured 0.96 ms in 2D and 2.45 ms in 3D. The 256-source/1,024-surface schedule measured 13.96 ms in 2D and 25.47 ms in 3D. The single-query 1,024-surface miss measured 51.2 microseconds in 2D and 107.8 microseconds in 3D. These quick runs are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The 90 FPS target allows 11.11 ms per frame. The 128/256 schedule measurements fit inside that budget on this CPU, but the 256/1,024 measurements do not. These schedules exclude rendering, audio output, and browser presentation. The visible browser check ran through SwiftShader in Xvfb and reported about 1–5 FPS. Native display validation was unavailable on this host. Overall 90 FPS is unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples. GitHub Pages is configured for workflow deployment at [the published gallery](https://sagan-software.github.io/bevy-raytraced-audio/). The latest published workflow passed for the preceding revision; this code change still needs a successful deployment run.

The previous local Chromium run loaded each route. After a click, Bevy's Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter changes sink volume for direct occlusion and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering and 90 FPS have not been verified on a physical GPU.
