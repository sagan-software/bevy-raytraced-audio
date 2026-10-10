# Measured optimization rounds

The fixed native suite contains 51 cases. The shared real-browser suite contains
21. Native measurements use Rust 1.99.0, the generic x86-64 target, optimized
Criterion executables and the Intel i7-8750H Linux host. Browser measurements use
the collaborative Chromium 152 / Electron 44 browser, which reports 32 logical
processors; it runs on a different host. Cross-platform absolute times are not
compared. Each before/after ratio uses the same environment and workload.

| Candidate                                                   | Native ratio vs compared baseline | Conservative lower bound |  Browser geometric mean | Accepted round |
| ----------------------------------------------------------- | --------------------------------: | -----------------------: | ----------------------: | -------------- |
| Math and DSP arithmetic                                     |                            1.463× |                   1.430× |                  1.324× | No             |
| Above, plus FIR lanes and visibility termination            |                            1.720× |                   1.684× |                  1.721× | Round 1        |
| Reuse unchanged Bevy traces (vs Round 1)                    |                            1.539× |                   1.506× | Shared kernel unchanged | Round 2        |
| Geometry tree only (vs Round 2)                             |                            1.017× |                   0.999× |       0.960× vs Round 1 | No             |
| Above, plus exact shared trace reuse (vs Round 2)           |                            5.194× |                   5.103× |      52.135× vs Round 1 | Round 3        |
| Cached 2D geometry, path traversal and DSP (vs Round 3)     |                            1.095× |                   1.075× |            Not measured | No             |
| Above, plus parallel-wall reflection rejection (vs Round 3) |                            1.802× |                   1.773× |       1.259× vs Round 3 | Round 4        |

The accepted round retains ray counts, reflection depths, FIR lengths, materials,
source counts and update workloads. It uses a fast norm with scaled-hypot
fallback for extreme values, avoids per-sample software FMA dispatch, groups FIR
sums into four independent lanes, omits coefficient interpolation when the filter
has never changed, stops visibility traversal on its first blocker and reuses
solver-space source positions. The FIR test compares against the scalar reference
with a 1e-6 absolute tolerance and checks identical smoothed coefficient state.
The extreme-range norm tests retain overflow/underflow behavior.

The first native candidate **failed** the requested gate; it was not promoted to
a baseline. Its full comparison is retained, along with both versions' browser
samples. Round 1's full 51-case comparison, estimates, confidence intervals and
raw Criterion samples are committed here. `target/performance` additionally
contains frozen native executables, WASM builds and command logs.

Round 2 caches deterministic Bevy traces while listener, source positions and
surfaces stay unchanged. Settings changes, new sources and surface changes
invalidate the trace; moved sources obey the configured trace interval. Newly
attached reflection-path output is populated even on a cache hit. Gains are
concentrated in stationary adapters (1.25×–961×); simultaneous-change stress
remains approximately level (0.95×–1.02×). The cumulative native ratio is 2.646×.
This round changes adapter scheduling only; it does not claim another browser
kernel speedup. Its 215 workspace tests passed across Bevy 0.17–0.20, including
cache invalidation checks, and adapter Clippy passed.

The first round also passed the [Linux, macOS and Windows CI matrix](https://github.com/sagan-software/bevy-raytraced-audio/actions/runs/38074008460).
These are correctness/build/smoke checks, not hosted-runner timing comparisons.

Round 3 adds exact deterministic reuse to the shared listener-tracing API.
The output retains a scene revision identity, listener, settings and ordered
source positions. A geometry mutation replaces the identity; a changed source,
listener or setting retraces. Unchanged scene clones share the identity safely.
Tests compare the complete cached result with a fresh trace after each input
changes, including scene/output clones and recording changes.

This produces very large warm-cache gains in eight shared cases. It is **not**
a 52× increase in ray-intersection speed. Changing-scene cases still execute the
solver. The geometry-only attempt did not reach the gate, despite improving the
largest native 2D/3D churn cases by 35%/16%. Its failed comparison is retained.
The cumulative native ratio through three accepted rounds is approximately 13.74×;
the shared browser suite is approximately 89.7× its original baseline, dominated
by repeated-input reuse. These are suite ratios, not application frame rates.

Round 4 rejects impossible first-order reflection groups using conservative bounds
in the tangential coordinates of parallel axis-aligned walls. Other orientations
retain exact per-surface tests, and small scenes use the cheaper direct path.
It also caches 2D geometric terms, prunes transmission paths by the ray, avoids
modulo in delay-ring reads and specializes the unchanged one-tap stereo fallback.
Tests compare cached geometry with scalar calculations, delay indexing with the
modulo reference and HRTF coefficients with the fused reference (error < 1e-6).

All four native rounds pass the fixed 51-case gate. Their cumulative geometric
mean ratio is **24.767×**. Round 4's biggest gain is the 1,024-triangle miss query
(16.24 µs to 32.09 ns, 506×). Its largest regression is the small 3D reflection
iterator (61.35 ns to 88.04 ns, 43.5% more latency). Every case, including
regressions, remains in the aggregate and raw comparison.

The final real-browser run completed all 21 unchanged workloads with finite
checksums and no hidden-tab intervals. Round 4 improves their geometric mean by
25.9%; the cumulative browser ratio is **112.963×**. This remains a shared-kernel
suite result dominated by unchanged-trace reuse, not an application frame-rate
claim. Native rounds use the 1.5× acceptance gate; no claim is made that every
round independently improves browser performance by 50%.

## Profiling evidence

Userspace pprof sampling produced actual flame graphs without changing host
security settings (`perf_event_paranoid=4` prevents perf events here):

- [3D listener trace](flamegraphs/listener3d.svg): 15,289 samples; BVH ray-entry
  tests account for 49.4% of leaf samples in the 1,024-emitter workload.
- [64 changing DSP voices](flamegraphs/dsp.svg): repeated software-dispatched
  FMA routines appear in the hot per-sample path.
- [Moving HRTF](flamegraphs/hrtf.svg): convolution dominates, with additional
  time in response decoding/interpolation.

The graphs are sampling evidence, not instruction counts. Profiling runs are
separate from Criterion measurement. [Leaf-sample summaries](flamegraphs/baseline-summary.json)
retain the counts used above.

Additional [Callgrind and DHAT evidence](profiles/summary.json) uses the original
and final profiling binaries on `churn/3d/128`, five iterations, including setup.
Instruction counts change from 361,315,650 to 359,181,320 (about 0.6% fewer).
Peak heap usage increases from 81,464 to 86,712 bytes; total allocated bytes
increase from 395,396 to 441,841. Caches trade some memory for query reuse.
This changing-scene result is much smaller than the aggregate warm-cache gain.
The raw profiles are retained beside the summary.

The 64-changing-voice DSP driver allocates 796 blocks for both one and ten
iterations, with the same 5,492,848-byte peak. Total bytes differ by one byte
in whole-process output formatting. No per-iteration allocation growth was
observed in this test; that is not a general leak or real-time-deadline guarantee.

## Verification status

A fresh strict workspace coverage run passed **296 tests** across Bevy 0.17–0.20,
with all four `debug_draw` features enabled. Production coverage is **9,309/9,309
lines (100%)** and **934/934 functions (100%)**. LLVM regions are 12,992/13,094
(99.22%); generic instantiations are 1,546/1,710 (90.41%). Branch coverage was not
instrumented. These metrics are distinct; this is a 100% line/function result,
not a claim of complete branch or instantiation coverage. The [per-file summary](coverage-summary.json)
retains all of these metrics. Only tests, benchmarks and examples are excluded;
production adapter, audio-processing and debug-drawing modules are included.

The strict script cleans both old profiles and workspace binaries for the custom
coverage profile before measuring. It selects every workspace package explicitly,
uses an isolated ALSA null device on Linux for older rodio output controls, and
fails below 100% lines. `nix run .#coverage` and the Nix coverage derivation use
the same gate. This supersedes the earlier partial coverage reports.

The expanded tests exercise 4,096 simultaneously changing emitters, decoding and
internal looping on all four Bevy versions, finite gizmo output without a GPU,
cache invalidation, scalar geometry/FIR references and extreme floating-point
inputs. Strict core, performance-harness and Bevy 0.19 adapter Clippy passed.
The full WebGL2 showcase also builds for `wasm32-unknown-unknown`.

Additional environment timings, final browser execution and final CI results
are recorded below as verification completes.
