# Tests and benchmark results

## Verification snapshot

The local verification run used Rust 1.99.0 on Linux 6.18.49, on an Intel Core i7-8565U laptop CPU. On 2026-10-06, `nix run .#check` passed formatting, strict Clippy, Bevy 0.17–0.20 release-candidate checks, 81 workspace tests, doctests, rustdoc, benchmark compilation, and the coverage threshold. `nix run .#dylint` passed. The scoped personal Rust lint for the changed core package passed; the full personal-lint dispatcher reported repository-wide documentation diagnostics and could not find ALSA for its Bevy 0.18 phase. Its Markdown phase also reported four paragraph-length findings in unchanged documents.

The current coverage report counted 2,021 lines and missed 19, for 99.06% line coverage. It counted 3,016 regions and missed 50, for 98.34% region coverage. It executed 203 of 204 functions, for 99.51% function coverage. The 3D image-source ray's out-of-range parameter branch remains unreachable for a valid candidate: eligibility mirrors the source across the reflector plane, placing the plane crossing between the image source and listener. Adapter helper error paths that construct validated geometry also remain uncovered. The BVH broad phase and cached 3D triangle paths are exercised by unit and public integration tests.

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

The reusable path-output query measured 87 ns in 2D and 135 ns in 3D. The 128-source/256-surface adapter schedule measured 0.85 ms in 2D and 1.25 ms in 3D. The 256-source/1,024-surface schedule measured 9.79 ms in 2D and 13.02 ms in 3D. The single-query 1,024-surface miss measured 40.6 microseconds in 2D and 45.7 microseconds in 3D. These quick runs are local estimates; they do not establish a cross-machine performance baseline or allocation rate.

The 90 FPS target allows 11.11 ms per frame. Both 128/256 schedule measurements and the 2D 256/1,024 schedule fit inside that budget on this CPU. The 3D 256/1,024 schedule measures 13.02 ms and exceeds it. These schedules exclude rendering, audio output, and browser presentation. The visible browser check ran through SwiftShader in Xvfb and reported about 1–5 FPS. Native display validation was unavailable on this host. Overall rendered 90 FPS is unverified.

## Browser checks

`nix run .#web-build` built the gallery, Markdown book, and four separate WebAssembly examples with the reflection-path overlays. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The previous local Chromium run loaded each route. After a click, Bevy's Web Audio context changed to `running`. This confirms browser audio activation, not audible output on a physical device. This revision's WebAssembly modules compile, but no browser session was available for visual or audio playback review.

## Scope not verified

- GPU tracing and CPU/GPU parity are not implemented; CPU fallback behavior is therefore not applicable yet.
- Reflections are response data only. The adapter changes sink volume for direct occlusion and does not process samples with filters, reflection taps, or late reverb.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native example rendering and 90 FPS have not been verified on a physical GPU.
