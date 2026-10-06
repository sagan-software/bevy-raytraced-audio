# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 105 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#dylint` passed for the `correctness`, `perf`, and `suspicious` lint groups. Liamc's scoped fast Rust lint passed for all four Bevy compatibility packages.

The coverage report counted 2,448 lines and missed 23, for 99.06% line coverage. It counted 3,488 regions and missed 55, for 98.42% region coverage. It executed 231 of 232 functions, for 99.57% function coverage. Every added material-transmission function and BVH candidate-traversal path is covered.

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

The latest quick run measured the reusable path-output query at 112 ns in 2D and 131 ns in 3D. The 64-source/64-surface adapter schedule measured 90 microseconds in 2D and 107 microseconds in 3D. The 128/256 schedule measured 0.402 ms and 0.460 ms. The 256/1,024 schedule measured 7.510 ms and 4.121 ms. A single-query miss over 1,024 surfaces measured 40.412 microseconds in 2D and 44.093 microseconds in 3D. These quick results are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The adapter uses sequential iteration below 16 emitters and when Bevy's compute task pool is not initialized. Otherwise, it uses `Query::par_iter_mut`. The benchmark harness enables Bevy's `multi_threaded` feature and initializes `TaskPoolPlugin`. A separate 30-sample comparison of the 256/1,024 schedule measured 10.726 ms in 2D and 13.043 ms in 3D on the serial path, then 5.508 ms and 6.006 ms on the parallel path. Criterion reported both reductions at `p < 0.05`.

The 90 FPS target allows 11.11 ms per frame. Every measured adapter update schedule fits within that time on this CPU. These schedules exclude rendering, audio output, and browser presentation, so they do not establish a full-game frame rate.

The headless SwiftShader browser run sampled 334 FPS for 2D stress, 4 FPS for 3D stress, 4 FPS for minimal 2D, and 98 FPS for minimal 3D. These are software-rendered route samples, not sustained measurements on physical graphics hardware. Rendered 90 FPS remains unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the material-transmission example positions. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The local Chromium review loaded all four routes. After a click, each Bevy Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device. At a 390-pixel viewport, the gallery and both stress pages had no horizontal overflow. The committed GIFs alternate between each tutorial page and its stress page; they are route comparisons, not continuous motion recordings.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter scales sink volume by the mean direct amplitude gain and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering, audible output, and steady 90 FPS have not been verified on a physical GPU and audio device.
