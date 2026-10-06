# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 107 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#features` compiled all four Bevy selections. `nix run .#dylint` passed the `correctness`, `perf`, and `suspicious` lint groups. The additional Liamc fast Rust lint failed on repository-wide findings. Its `src/lib.rs` module-fan-out rule allows 7 modules; the file had 24 declarations at `HEAD` and has 25 now. It also reported documentation findings in unchanged files.

The coverage report counted 2,468 lines and missed 23, for 99.07% line coverage. It counted 3,502 regions and missed 55, for 98.43% region coverage. It executed 233 of 234 functions, for 99.57% function coverage. Every added material-transmission function and BVH candidate-traversal path is covered.

Remaining misses include error exits in successful test-fixture construction and two geometry branches. A valid 3D image-source candidate cannot reach the out-of-range ray-parameter branch because mirror construction places the plane crossing between the image source and listener. Current fixtures do not reach the 2D near-parallel branch. Unit and public integration tests exercise the BVH broad phase and cached 3D triangle paths.

Run the suite and coverage report with:

```sh
nix run .#test
nix run .#coverage
```

## Benchmark workload coverage

`nix run .#bench -- --quick --noplot` completed all 26 named Criterion scenarios (26/26, 100% of the named workload set):

- Six one-source/one-listener traces: open, occluded, and first-order reflection paths in both 2D and 3D.
- Four first-order reflection workloads: two lazy iterator queries and two queries that write into reused path-output storage.
- Six miss-heavy scene queries: 32, 256, and 1,024 surfaces in both 2D and 3D.
- Ten Bevy schedule workloads: 2D and 3D scenes with 1/0, 1/1, 64/64, 128/256, and 256/1,024 sources/surfaces.

The 2026-10-06 quick run measured the reusable path-output query at 113 ns in 2D and 129 ns in 3D. The 64-source/64-surface adapter schedule measured 93.690 microseconds in 2D and 111.370 microseconds in 3D. The 128/256 schedule measured 0.381 ms and 0.436 ms. The 256/1,024 schedule measured 4.036 ms in 2D and 4.281 ms in 3D. A single-query miss over 1,024 surfaces measured 40.210 microseconds in 2D and 44.276 microseconds in 3D. Criterion's reported intervals are short quick-run estimates; they do not establish a cross-machine performance baseline or allocation rate.

The adapter uses sequential iteration below 16 emitters and when Bevy's compute task pool is not initialized. Otherwise, it uses `Query::par_iter_mut`. The benchmark harness enables Bevy's `multi_threaded` feature and initializes `TaskPoolPlugin`. A separate 30-sample comparison of the 256/1,024 schedule measured 10.726 ms in 2D and 13.043 ms in 3D on the serial path, then 5.508 ms and 6.006 ms on the parallel path. Criterion reported both reductions at `p < 0.05`.

The 90 FPS target allows 11.11 ms per frame. Every measured adapter update schedule fits within that time on this CPU. These schedules exclude rendering, audio output, and browser presentation, so they do not establish a full-game frame rate.

The headless SwiftShader browser run sampled 334 FPS for 2D stress, 4 FPS for 3D stress, 4 FPS for minimal 2D, and 98 FPS for minimal 3D. These are software-rendered route samples, not sustained measurements on physical graphics hardware. Rendered 90 FPS remains unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the material-transmission example positions. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The local Chromium review loaded all four routes. After a click, each Bevy Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device. At a 390-pixel viewport, the gallery and both stress pages had no horizontal overflow. The committed GIFs alternate between each tutorial page and its stress page; they are route comparisons, not continuous motion recordings.

## Scope not verified

- GPU tracing is not implemented. `AudioBackendPreference::Auto` logs that GPU compute is unavailable and keeps CPU tracing active. The new adapter tests cover that no-renderer fallback, but they do not exercise GPU device detection, pipeline failures, device loss, readback, or CPU/GPU parity.
- Reflections are response data only. The adapter scales sink volume by the mean direct amplitude gain and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering, audible output, and steady 90 FPS have not been verified on a physical GPU and audio device.
