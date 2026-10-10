# Measured optimization rounds

The fixed native suite contains 51 cases. The shared real-browser suite contains
21. Native measurements use Rust 1.99.0, the generic x86-64 target, optimized
Criterion executables and the Intel i7-8750H Linux host. Browser measurements use
the collaborative Chromium 152 / Electron 44 browser, which reports 32 logical
processors; it runs on a different host. Cross-platform absolute times are not
compared. Each before/after ratio uses the same environment and workload.

| Candidate                                        | Native geometric mean vs starting code | Conservative lower bound | Browser geometric mean | Accepted round |
| ------------------------------------------------ | -------------------------------------: | -----------------------: | ---------------------: | -------------- |
| Math and DSP arithmetic                          |                                 1.463× |                   1.430× |                 1.324× | No             |
| Above, plus FIR lanes and visibility termination |                                 1.720× |                   1.684× |                 1.721× | Round 1        |
| Reuse unchanged Bevy traces (vs Round 1) | 1.539× | 1.506× | Shared kernel unchanged | Round 2 |

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

## Verification status

The accepted core change passed 92 core/shared workload tests, strict core/harness
Clippy, the scalar FIR and extreme-range math regressions, and real-browser
execution of all 21 cases with finite output and no hidden-tab intervals.
The initial core coverage report was 97.50% lines, 97.76% regions and 95.66%
functions. That is **not 100%**. The next workspace run passed its tests, but initially reported only default
members. Selecting all packages explicitly reports 92.85% lines (952 uncovered),
including shared adapter sources instantiated by each Bevy compatibility crate.
The strict coverage command fails unless all selected lines execute. This is
not a 100% coverage result.

Four successful 50% rounds are required by the request; only measured rounds
that pass the complete-suite gate count. This report does not claim four rounds
or 100% coverage until those checks pass.
