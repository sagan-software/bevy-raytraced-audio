# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 108 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#features` compiled all four Bevy selections. `nix run .#dylint` passed the `correctness`, `perf`, and `suspicious` lint groups.

The additional Liamc fast Rust lint failed on repository-wide findings. Its `src/lib.rs` module-fan-out rule allows 7 modules; the file had 24 declarations at `HEAD` and has 25 now. It also reported documentation findings in unchanged files.

The coverage report counted 2,468 lines and missed 23, for 99.07% line coverage. It counted 3,502 regions and missed 55, for 98.43% region coverage. It executed 233 of 234 functions, for 99.57% function coverage. Every added material-transmission function and BVH candidate-traversal path is covered.

Remaining misses include error exits in successful test-fixture construction and two geometry branches. A valid 3D image-source candidate cannot reach the out-of-range ray-parameter branch because mirror construction places the plane crossing between the image source and listener. Current fixtures do not reach the 2D near-parallel branch. Unit and public integration tests exercise the BVH broad phase and cached 3D triangle paths.

Run the suite and coverage report with:

```sh
nix run .#test
nix run .#coverage
```

## Benchmark workload coverage

`nix run .#bench -- --quick --noplot` completed all 34 Criterion workloads declared by the suite (34/34, 100% of the named workload set):

- Six one-source/one-listener traces: open, occluded, and first-order reflection paths in both 2D and 3D.
- Four first-order reflection workloads: two lazy iterator queries and two queries that write into reused path-output storage.
- Six miss-heavy scene queries: 32, 256, and 1,024 surfaces in both 2D and 3D.
- Ten Bevy schedule workloads: 2D and 3D scenes with 1/0, 1/1, 64/64, 128/256, and 256/1,024 sources/surfaces.
- Eight serial/task-pool comparisons: both dimensions at 16/32 and 64/64 sources/surfaces.

The 2026-10-06 quick run measured the reusable path-output query at 111.89 ns in 2D and 128.98 ns in 3D. The 64-source/64-surface adapter schedule measured 88.982 microseconds in 2D and 102.50 microseconds in 3D. The 128/256 schedule measured 0.361 ms and 0.427 ms. The 256/1,024 schedule measured 3.657 ms in 2D and 4.719 ms in 3D. A single-query miss over 1,024 surfaces measured 40.546 microseconds in 2D and 44.193 microseconds in 3D. Criterion's reported intervals are short quick-run estimates; they do not establish a cross-machine performance baseline or allocation rate.

At the 16-emitter/32-surface stress workload, the serial path measured 65.379 microseconds in 2D and 64.512 microseconds in 3D. The task-pool path measured 62.148 microseconds and 62.780 microseconds. At 64/64, serial measured 98.357 microseconds in 2D and 113.82 microseconds in 3D; task-pool measured 93.387 microseconds and 115.05 microseconds. These are quick estimates without a paired statistical comparison. The 16-emitter threshold remains unchanged.

The adapter uses sequential iteration below 16 emitters and when Bevy's compute task pool is not initialized. Otherwise, it uses `Query::par_iter_mut`. The benchmark harness enables Bevy's `multi_threaded` feature and initializes `TaskPoolPlugin`. A separate 30-sample comparison of the 256/1,024 schedule measured 10.726 ms in 2D and 13.043 ms in 3D on the serial path, then 5.508 ms and 6.006 ms on the parallel path. Criterion reported both reductions at `p < 0.05`.

The 90 FPS target allows 11.11 ms per frame. Every measured adapter update schedule fits within that time on this CPU. These schedules exclude rendering, audio output, and browser presentation, so they do not establish a full-game frame rate.

The measured Bevy update schedules exclude browser rendering and presentation. A separate hardware-backed browser run measured uncapped frame callbacks on the Intel UHD Graphics 620 through ANGLE Vulkan. Chromium ran with `--disable-frame-rate-limit` and `--disable-gpu-vsync` at a 1,134 × 638 canvas size.

The run collected 15 consecutive one-second bins per stress scene. The 3D samples ranged from 131 to 153 callbacks per second, with a 143.47 mean. The 2D samples ranged from 163 to 174, with a 168.67 mean. Each bin exceeded the 90 FPS target.

These are uncapped browser animation-frame callbacks, not physical display presentations. With normal headless synchronization, the on-canvas diagnostic showed 55–61 FPS. The physical display refresh rate and native window presentation remain unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the material-transmission example positions. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The local Chromium review loaded all four routes. After a click, each Bevy Web Audio context changed to `running` at 44.1 kHz. A DevTools Web Audio trace of the 3D stress page showed 59 `AudioBufferSource` nodes connected to a running `AudioDestination`; the reported callback interval mean was 10.66 ms. This verifies the browser output graph, but it does not verify sound from physical speakers.

The example package enables Bevy's `wav` feature for the shared fixture. The `audio_fixture` integration test calls Bevy's decoder and passes. The latest browser build included the WAV decoder, and all four routes loaded without WebAssembly exceptions. Bevy also requested optional `.wav.meta` sidecars; those requests returned 404 while the WAV files loaded with HTTP 200.

At a 390-pixel viewport, the gallery and both stress pages had no horizontal overflow. The README GIFs contain 12 sampled frames from each hardware-rendered stress route.

## Scope not verified

- GPU tracing is not implemented. `AudioBackendPreference::Auto` logs that GPU compute is unavailable and keeps CPU tracing active. The new adapter tests cover that no-renderer fallback, but they do not exercise GPU device detection, pipeline failures, device loss, readback, or CPU/GPU parity.
- Reflections are response data only. The adapter scales sink volume by the mean direct amplitude gain and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native window rendering and sound from a physical audio device have not been verified. The browser frame samples exceeded 90 callbacks per second with VSync disabled; that run does not test a physical display's refresh rate.
