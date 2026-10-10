# Tests and benchmark results

The current checks cover the listener-ray and DSP update. The older performance
and coverage measurements are retained separately below; they do not measure
the new sandbox, processed audio, or nine-example gallery.

## Current verification: 2026-10-09

The local Rust 1.99.0 run passed:

- `cargo nextest run --locked --workspace --all-targets`: 214 entries, including
  180 tests and 34 benchmark cases. The benchmark cases ran alongside browser
  compilation, so this run does not establish a new timing baseline.
- `cargo test --locked --workspace --doc`: eight doctests across the four Bevy
  compatibility packages.
- Strict workspace Clippy, followed by a strict examples check after the final
  sandbox changes.
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps --document-private-items`.
- `node --test website/tests/*.test.mjs`: 52 tests.

The regressions cover immediate filter/reverb reset when tracing is disabled,
delayed echoes surviving their initial silent pre-delay, live filter changes
across audio loops, fully absorbing rooms producing no reverb send, finite DSP
parameters, and bounded player movement after an inactive browser frame.
All 18 Ogg recordings decoded fully to finite, unclipped mono samples at
44.1 kHz. The real-recording muffling test retained bass while reducing treble.
The sandbox movement regression also passed after the final HUD changes.

The complete browser builder produced nine optimized WebAssembly examples,
19 gallery/example/embed pages, and the Markdown book. All nine routes rendered
with running Web Audio graphs and no observed console errors or failed requests.
Browser input checks covered doors, windows, room size, clutter, source
switching, ray visibility/speed/pause, processing bypass, wall construction and
removal, and player movement while ray animation was paused. The gallery's
embedded sandbox also loaded successfully.

Nonzero output samples were observed in the sandbox, 3D doorway, and clap demo.
The gallery and sandbox had no page-level horizontal overflow at 390 pixels;
the sandbox uses a compact status panel at narrow widths. These are browser
signal and layout checks, not a physical listening test. The collaborative
browser throttled inactive frames, so no new FPS claim is made.

The first-person sandbox follow-up passed 167 workspace library/integration
tests, five sandbox tests, strict workspace and final examples Clippy, and 55
website tests. Its stereo regression renders rodio output for 192 combinations
of view mode, heading, pitch, side, and distance. It also checks camera-relative
movement, wall collision, nearest-wall crosshair targeting, Escape restoration,
and cursor capture being requested only on mode transitions. The workspace
backports rodio's upstream two-line panning correction without updating the
published 0.22.2 dependency stack.

The sandbox WASM was rebuilt for that follow-up. Browser checks covered the
humanoid, first-person mouse look, crosshair wall removal/construction, and
returning overhead. With rain muted and processing bypassed, captured browser
PCM had a right/left RMS ratio of 1.27 with the speaker on the right, then a
left/right ratio of 1.49 after turning to put it on the left. These are signal
checks, not headphone listening. The embedded preview rejects pointer lock;
the loader handles that rejection and explains the Q/E and PgUp/PgDn fallback.

Fresh coverage, physical-device listening, and full rendered-frame benchmarks
remain outstanding. The checks below describe the earlier implementation.

## Acoustic village follow-up (2026-10-10)

The village follow-up passes **178 workspace library/integration tests**, **9
sandbox tests**, and **56 website tests**. Strict workspace Clippy and the final
examples Clippy pass. The audio fixtures decode all 20 Ogg recordings. The
sandbox tests use its actual module geometry for aperture and floor checks and
run the real movement system up and down both stairs with structural colliders.
They also check camera independence, rotated-door collision, solid stair bulk,
bridge clearance, category isolation, and actual rodio stereo samples.

Core regressions cover shared 2D segment joins and 3D triangle diagonals without
merging separate wall layers. Update-cadence regressions compile against all
four supported Bevy versions. A rodio test checks that positional downmixing
averages mono, stereo and six-channel input instead of summing it. Generated
website source now includes the sandbox's local Rust modules as well as its
entry point.

Browser verification loaded the actual CC0 models, furnishings and recordings;
checked mouse-facing rotation with an independent overhead camera; opened
single/double doors and a window; removed and rebuilt a marked wall section;
entered first person; walked through open
double doors; inspected basement and upstairs cutaways; and compared the stream
on the bridge with the water's edge. Speech changes from strongly attenuated
treble behind closed leaves to 100% along the open direct path. The water has a
blocked path above the bridge deck and a clear path beside it. No runtime or
asset-fetch errors were observed during these checks. Pointer lock rejection
in the embedded preview remains handled, with Q/R and PgUp/PgDn as fallback.

A separate output-muted verification tab captured 172 consecutive scheduled
buffers over eight seconds in the bridge scenario: no late buffers, peak 0.0192.
Master mute plus processing bypass subsequently produced 217,088 exact-zero
samples, confirming that bypass cannot unmute excluded sounds. The earlier
first-pass speech probe peaked at 0.3455 with no clipping. Embedded-preview
view transitions can still stall the main-thread Web Audio scheduling; these
checks do not claim glitch-free playback under arbitrary rendering load.
See the [audit](SANDBOX-AUDIT.md) for reproducible presets, confirmed fixes and
physical-model limitations. Physical headphone listening and HRTF elevation
are not covered by these signal checks.

All nine WebGL2 WebAssembly examples were rebuilt and passed browser startup
checks with active audio graphs and no observed runtime or asset-fetch errors.
The generated gallery, book, example routes, and sampled assets passed 56 HTTP
checks; all nine compressed WebAssembly files passed integrity checks. The
34 bundled GLB models resolve their texture references and retain the three
original pack licenses. The final sandbox check also confirmed that basement
interaction prompts no longer target doors or editable walls on the floor above.

## Elevation and material update (2026-10-10)

The elevation/material change passed 205 Rust tests: core geometry and DSP,
all four compatibility crates, public adapter tests, decoded audio fixtures,
and eleven showcase regressions. The 56 website tests passed. Focused Clippy
checks reported no new project warnings (the vendored rodio warnings remain).
Rust, Markdown/JavaScript and Nix formatting checks passed.

The showcase was built for WebGL2/WebAssembly and exercised in the collaborative
Chromium preview. Checks covered above/below presets, independent HRTF and room
bypasses, tile/carpet/furniture replacement, indoor and outdoor clap/gunshot
playback, and first-person view. No scene runtime errors were observed. The
preview rejects pointer lock; its existing Q/R and PgUp/PgDn fallback remains
available.

A warmed capture covering 2,538 scheduled stereo buffers (5,197,824 frames)
reported no buffers more than 20 ms late and a peak amplitude of 0.0702.
The earlier capture during scene startup did contain late buffers; this is not
a claim that loading or all devices are glitch-free. Tests verify actual
muffled footsteps retain different above/below waveforms, but physical
headphone listening and individual localization accuracy remain subjective.
The [research note](research/vertical-audio-and-materials.md) describes the
measurement range and remaining acoustic approximations.

## Historical verification snapshot

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

The 2026-10-06 quick run measured the reusable path-output query at 159 ns in 2D and 184 ns in 3D. The 64-source/64-surface adapter schedule measured 121.344 microseconds in 2D and 143.926 microseconds in 3D. The 128/256 schedule measured 0.445 ms and 0.621 ms. The 256/1,024 schedule measured 4.684 ms in 2D and 5.632 ms in 3D. A single-query miss over 1,024 surfaces measured 56.387 microseconds in 2D and 65.055 microseconds in 3D. Criterion's reported intervals are short quick-run estimates; they do not establish a cross-machine performance baseline or allocation rate.

At the 16-emitter/32-surface stress workload, the serial path measured 77.198 microseconds in 2D and 81.423 microseconds in 3D. The task-pool path measured 73.976 microseconds and 78.618 microseconds. At 64/64, serial measured 121.457 microseconds in 2D and 146.436 microseconds in 3D; task-pool measured 118.734 microseconds and 145.182 microseconds. These are quick estimates without a paired statistical comparison. The 16-emitter threshold remains unchanged.

The adapter uses sequential iteration below 16 emitters and when Bevy's compute task pool is not initialized. Otherwise, it uses `Query::par_iter_mut`. The benchmark harness enables Bevy's `multi_threaded` feature and initializes `TaskPoolPlugin`. A separate 30-sample comparison of the 256/1,024 schedule measured 10.726 ms in 2D and 13.043 ms in 3D on the serial path, then 5.508 ms and 6.006 ms on the parallel path. Criterion reported both reductions at `p < 0.05`.

The 90 FPS target allows 11.11 ms per frame. Every measured adapter update schedule fits within that time on this CPU. These schedules exclude rendering, audio output, and browser presentation, so they do not establish a full-game frame rate.

The measured Bevy update schedules exclude browser rendering and presentation. A hardware-backed browser run measured uncapped animation-frame callbacks on the Intel UHD Graphics 620 through ANGLE Vulkan. Chromium ran with `--disable-frame-rate-limit` and `--disable-gpu-vsync` at a 1,215 × 700 canvas size.

The 2026-10-07 run collected three consecutive one-second bins for each route at 1,215 × 700 pixels, for 15 bins total. Every bin exceeded 90 callbacks per second.

| Route      | Callback range per second |
| ---------- | ------------------------: |
| Forest 3D  |                   120–127 |
| Minimal 2D |                   136–145 |
| Minimal 3D |                    93–100 |
| Stress 2D  |                   127–143 |
| Stress 3D  |                   119–129 |

These are uncapped browser animation-frame callbacks, not physical display presentations. Normal headless synchronization capped the on-canvas diagnostic near 60 FPS. Native window presentation and a 90 Hz display remain unverified.

## Historical browser checks

`nix run .#web-build` built the gallery, Markdown book, and five separate WebAssembly examples. The repository's GitHub Pages workflow deploys the gallery and book at [the published site](https://sagan-software.github.io/bevy-raytraced-audio/).

The local Chromium 154 review loaded all five routes after successive WebGL draw calls. A trusted click changed each route's Bevy Web Audio context from `suspended` to `running`; each button then showed `Mute sound`. In the standalone and inline forest checks, Bevy source-start calls reached the Web Audio destination. These checks verify browser audio graphs, not physical speaker output.

At a 390 × 844-pixel viewport, the gallery and expanded forest demo had no horizontal overflow. The forest canvas resized to 390 × 746 pixels in the standalone route and 356 × 402 pixels inside the gallery card. The camera fits the acoustic scene to the viewport aspect ratio, keeping the listener, chime, direct path, and reflection paths visible. The README contains 12-frame stress captures and a 36-frame forest walkthrough. The forest GIF shows direct gain change as the listener crosses the brush screen.

Firefox 157 headless returned `null` from `canvas.getContext("webgl2")`. The route displayed the Firefox hardware-acceleration recovery message instead of a static scene. This does not verify normal Firefox with hardware acceleration.

## Scope not verified

- GPU tracing is not implemented. `AudioBackendPreference::Auto` logs that GPU compute is unavailable and keeps CPU tracing active. The new adapter tests cover that no-renderer fallback, but they do not exercise GPU device detection, pipeline failures, device loss, readback, or CPU/GPU parity.
- The new processed player applies muffling and aggregate reverb. Individual delayed reflection taps, codec allocation behavior under load, and physical listening quality remain unverified.
- The benchmark suite has no allocation counter and does not measure the audio callback. It measures geometry queries and scheduled Bevy updates.
- Native window rendering and sound from a physical audio device have not been verified. The browser frame samples exceeded 90 callbacks per second with VSync disabled; that run does not test a physical display's refresh rate.
