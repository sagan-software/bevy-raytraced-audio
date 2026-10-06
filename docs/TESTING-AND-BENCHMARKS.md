# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 71 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#dylint` and the Sagan personal Rust lint workflow also passed.

The coverage report counted 1,637 lines and missed 15, for 99.08% line coverage. It counted 2,512 regions and missed 37, for 98.53% region coverage. All 167 functions executed. Two defensive geometry branches remain uncovered: a 2D non-collinear parallel case is rejected by an earlier bounds check, and the 3D image-source ray cannot meet the reflecting plane outside the source-to-listener interval for a valid candidate. Six `?` error paths in adapter test helpers construct valid surfaces and remain uncovered. The added reflection-path query and adapter output lines have full line coverage.

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

The path-output query measured about 96 ns in 2D and 174 ns in 3D. The 128-source/256-surface adapter schedule measured 1.14 ms in 2D and 2.89 ms in 3D. The 256-source/1,024-surface schedule measured 15.84 ms in 2D and 30.60 ms in 3D. The single-query 1,024-surface miss measured about 58.3 microseconds in 2D and 128.9 microseconds in 3D. These quick runs are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The 90 FPS target allows 11.11 ms per frame. The 128/256 schedule measurements fit inside that budget on this CPU, but the 256/1,024 measurements do not. These schedules exclude rendering, audio output, and browser presentation. The visible browser check ran through SwiftShader in Xvfb and reported about 1–5 FPS. Native display validation was unavailable on this host. Overall 90 FPS is unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the reflection-path overlays. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The previous local Chromium run loaded each route. After a click, Bevy's Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device. This revision's WebAssembly modules compile, but no browser session was available for visual or audio playback review.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter changes sink volume for direct occlusion and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering and 90 FPS have not been verified on a physical GPU.
