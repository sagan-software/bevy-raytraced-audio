# Performance laboratory

The starting application is checkpoint `211967f`. Benchmarks and stress fixtures
are committed separately before optimization. Do not compare with the historical
numbers in TESTING-AND-BENCHMARKS.md: the machine and application have changed.

## Acceptance protocol

“50% faster” means 1.5 times throughput, or at most two thirds of the previous
latency. Four successful rounds would compound to 5.0625 times throughput.
Use the equal-weight geometric mean of before/after mean latency ratios across
**every** native Criterion case. Retain individual timings and regressions.
`compare-benchmarks.py` rejects empty suites, missing/added cases and invalid
measurements. Its conservative bound combines per-case confidence endpoints;
it is not a joint statistical confidence interval. A round passes only when
that conservative lower bound reaches 1.5. An unsuccessful round does not become
a claimed 50% baseline. Do not change workloads, sample counts, quality settings,
compiler flags, CPU affinity or thread count between the paired measurements.

```sh
nix develop .#performance
scripts/benchmark-native.sh baseline
# Make and verify a production optimization.
scripts/benchmark-native.sh candidate1
python3 scripts/compare-benchmarks.py baseline candidate1 \
  --output target/performance/comparison1.json
```

Run benchmarks without concurrent compilation, coverage, browser measurement or
profiling. The runner builds everything first, uses 30 samples with 1 second
warmup and 3 seconds measurement, saves named Criterion estimates, records the
revision and machine, and refuses to overwrite a completed baseline. Keep
`target/criterion` with the environment logs for future comparisons. For
repeatability, repeat complete runs; short smoke runs are not acceptance evidence.

## Workloads

- Existing open/blocked/reflected paths, lazy/refill reflection output and
  32/256/1,024-surface queries in both dimensions.
- Real Bevy adapter updates at 1 through 256 emitters and up to 1,024 surfaces.
- Complete listener tracing with 1/64/1,024 emitters, 256 interior surfaces,
  enclosing walls, scattering, permeation, 64 primary rays and four bounces.
- Recorded rays in both dimensions, including secondary segments.
- Simultaneous replacement of 128/1,024 surfaces and movement of every source,
  with BVH rebuild included. Real ECS stress also mutates transforms and voices.
- 44.1/48 kHz mono/stereo filtering and reverb, including changes to 64 voices.
- 44.1/48/96 kHz measured HRTF convolution, moving sources and stereo fallback.

The shared `acoustic-performance` crate supplies exactly the same core workload
code to Criterion and WebAssembly. Setup and initial warmup happen outside the
timed loop; dynamic changes and rebuilding happen inside. Checksums are consumed
and outputs are checked for finiteness. The old serial/task-pool pairs were
removed because Bevy's pool is global: omitting its plugin after initialization
did not restore serial execution. Use fresh processes with `BEVY_TASK_THREADS=1`
and `BEVY_TASK_THREADS=4` instead; compare each configuration with itself.

## Browser

```sh
scripts/benchmark-web-build.sh target/performance/web-baseline
python3 -m http.server 8765 --directory target/performance
# Open http://localhost:8765/web-baseline/ and run the suite.
```

Build another directory for each candidate, preserving the baseline WASM.
The builder matches wasm-bindgen CLI to Cargo.lock. The page records its browser,
concurrency, build label, per-sample timings, medians, p95 and checksums. A run that
was hidden is flagged. Run both builds in the same visible browser and repeat.
These measure actual browser WASM execution, not Node, animation callbacks,
rendered frames, audio scheduling deadlines or physical device performance.

## Profiling and monitoring

The performance Nix shell includes cargo-flamegraph/perf, Inferno, Hyperfine,
Valgrind/Callgrind/DHAT, heaptrack, GDB, strace, sysstat, htop, cargo-bloat and
cargo-llvm-cov. Linux-only tools are conditional. Debug symbols are retained in
the bench/profiling profiles. Profiling runs are separate from timing baselines.

```sh
cargo flamegraph -p acoustic-performance --profile profiling -- \
  listener/3d/1024 2000
# Linux userspace sampling fallback when perf_event permissions are unavailable:
cargo run -p acoustic-performance --profile profiling --features sampling -- \
  listener/3d/1024 2000
# Writes flamegraph.svg and profile-stacks.txt; no sysctl changes needed.

cargo build -p acoustic-performance --profile profiling
valgrind --tool=callgrind --callgrind-out-file=target/performance/callgrind.out \
  target/profiling/acoustic-performance listener/3d/1024 10
callgrind_annotate target/performance/callgrind.out
valgrind --tool=dhat --dhat-out-file=target/performance/dhat.json \
  target/profiling/acoustic-performance churn/3d/1024 10
heaptrack target/profiling/acoustic-performance dsp/48000/changing64 1000
hyperfine --warmup 3 'target/profiling/acoustic-performance hrtf/48000/moving 10000'
```

The bounded driver includes initial construction in whole-process profiling;
Criterion excludes it. DHAT therefore reports both setup and repeated work.
Inspect call stacks before attributing allocations to the audio callback.
[Criterion's profiling guide](https://bheisler.github.io/criterion.rs/book/user_guide/profiling.html)
describes `--profile-time` for longer isolated loops. The optional
[pprof sampler](https://github.com/tikv/pprof-rs) uses userspace signals.

## Coverage

Coverage is a separate correctness metric, not the fraction of benchmarks that
ran. Report lines, regions and functions, with the scope and exclusions explicit.
Do not hide production files or unreachable code just to reach a percentage.
The strict runner selects every workspace package explicitly, cleans stale custom-profile
binaries, enables debug drawing for all four Bevy versions and requires 100% production
line coverage. Tests, benches and examples are excluded; library production files are not.
On Linux, old rodio spatial controls use a temporary, process-local ALSA null device.
Other platforms do not enable that virtual device fixture and may report uncovered lines.
Region coverage is reported separately; 100% line coverage does not imply every branch
or every possible input was tested.

```sh
nix develop .#performance
scripts/coverage-strict.sh target/performance/coverage
# The Nix coverage app/check uses the same runner and threshold.
```

Portable generic native code, a separately paired CPU-native build, single/multi
worker configurations and real browser WASM provide different execution
conditions. They do not establish Windows, macOS, ARM or physical audio-device
performance unless those systems were actually measured.

## Result-cache policy and invalidation measurements

`RayTraceSettings::with_result_reuse(false)` disables completed listener-result reuse.
Use the same setting on `RaytracedAudioTracing2d.settings` or
`RaytracedAudioTracing3d.settings` to force every _eligible_ adapter update to trace.
The adapter's `interval_s` still applies; use `0.0` when comparing every-frame work.
Surface changes and newly encountered emitters continue to request immediate updates.
The default is `true` for compatibility.

A cached listener result is reusable only when the scene revision, listener position,
trace settings, and ordered source positions match exactly. The cache contains one
previous result, not a history of scenes or approximate positions. Moving either
endpoint invalidates that result; inserting/removing a source also invalidates it.
Listener orientation independently updates binaural processing each frame. Movement
therefore defeats completed-result reuse, but unchanged geometry can still reuse its
spatial acceleration structure. Both policies retain that structure and scratch buffers.
Disabling result reuse skips comparisons and does not allocate or retain result revision
keys. It still collects the same diagnostic counters as the enabled path.

`ListenerTrace2d::cache_statistics()` and its 3D equivalent expose hits, computations,
forced computations, cold starts, and invalidation reasons. Reasons identify the **first**
difference, in scene/listener/settings/source order. They do not purport to count every
simultaneous mutation. Adapter `scheduling_statistics()` separately records scheduled,
unchanged, and throttled updates, plus geometry refreshes. A geometry refresh is not an
exact count of lazy BVH construction. Diagnostic changes do not mark acoustic result
resources as changed.

The `cache_behavior` Criterion suite pairs both policies on sixteen fixed workloads:
2D/3D static controls; listener movement; joint listener/source movement; movement of
the last of 1,024 sources; spawning/removal; geometry changes; simultaneous movement,
spawning/removal and geometry changes; and nine unchanged frames between movements.
Every other case has 64 sources (up to 67 during churn) and 128 interior surfaces plus
an enclosing room. Input sequences, quality, and mutation costs match between policies.
The tests compare every frame's acoustic checksum between policies; core regression
tests additionally compare complete response vectors, reverb, ambience and recorded rays.

```sh
nix develop .#performance
cargo bench --locked -p acoustic-performance --bench cache_behavior -- \
  --save-baseline cache-baseline --sample-size 30 --warm-up-time 1 --measurement-time 3
# Repeat in the opposite policy order to detect order/thermal bias.
CACHE_BENCH_REVERSE=1 cargo bench --locked -p acoustic-performance --bench cache_behavior -- \
  --save-baseline cache-reverse --sample-size 30 --warm-up-time 1 --measurement-time 3
cargo run --locked --release -p acoustic-performance --bin cache-statistics -- 1000 > cache-counts.json
python3 scripts/compare-cache.py --criterion target/criterion/cache_behavior \
  --baseline cache-baseline --output cache-comparison.json
scripts/benchmark-web-build.sh target/performance/cache-web
```

The browser laboratory's **Compare cache on/off** button alternates the policies within
each sample. Download its raw JSON and run `scripts/compare-cache.py --browser FILE
--output REPORT`. Positive reported overhead means caching was slower. Static controls
are reported individually, rather than averaged into dynamic-scene results. Counter
runs have a fixed frame count independent of Criterion's adaptive iteration counts.

## Actual application frame measurements

Showcase, `stress_2d`, and `stress_3d` accept opt-in benchmark configuration through
`ACOUSTIC_FRAME_BENCH` JSON natively or a `frame_bench` JSON query parameter in browsers.
Ordinary interactive launches retain their existing behavior. The fixed `active-v1`
workload moves both endpoints, replaces four processed sound voices each second, and
changes geometry. Stress runs add 240 sources to the existing sixteen, plus four
transient voices, for 260 traced sources. Showcase moves the player, keeps its moving
NPCs, toggles the real doors, and replaces transient voices. Audio stays enabled.

The harness preserves each application's ray count, bounce count, rendering assets,
and resolution, uses a trace interval of zero for both policies, and requests
`AutoNoVsync`. This intentionally measures every-frame acoustic updates rather than
the slower default application cadence. Animation/churn follow elapsed wall time.
The default warm-up is 15 seconds followed by 20 measured seconds. Raw frame intervals,
main-update durations, completed render schedules, cache/scheduling deltas, source
counts, the minimum/maximum live processed-sink counts, quality settings, resolution and selected GPU are retained. FPS is frames
per measured wall-clock duration, not the rolling HUD value. It includes rendering
and presentation backpressure; it does not prove that a display presents more frames
than its refresh rate. Browser presentation may remain vsync limited.

```sh
cargo build --locked -p bevy-raytraced-audio-examples --profile application-bench \
  --features frame-profile --example showcase --example stress_2d --example stress_3d
python3 scripts/benchmark-frames.py target/performance/frames-baseline --label baseline
scripts/benchmark-frame-web-build.sh target/performance/frame-web-baseline
# Set this as the browser query's frame_bench value, URL-encoded:
# {"cache":false,"warmup_s":15,"duration_s":20,"interval_s":0,"label":"baseline"}
```

Run each native suite serially with no compiler/profiler running concurrently. The
runner alternates cache-policy order across three repetitions and archives startup
logs, including audio/driver errors. On Linux the performance shell includes Mesa
Vulkan drivers and PipeWire ALSA plugins built against the Nix runtime; it does not
change the host's driver configuration. `VK_DRIVER_FILES` can select a specific ICD.
Measured-window native audio underruns invalidate the timing, while startup errors remain in the log.
In browsers, activate audio before the measurement window and retain
`window.acousticFrameReport`; suspended audio or hidden tabs invalidate a timing run.

`compare-frames.py BEFORE AFTER --output REPORT` requires all three applications,
at least three repetitions each, matching quality/environment, active audio, completed
render schedules, and **disabled result caching**. It gives applications equal weight
using the geometric mean of their median-FPS ratios. The acceptance gate requires at
least 1.5× even using the slowest new/best old whole-run ratios. That conservative
range is not a joint statistical confidence interval. Profiling runs are rejected.
Each accepted round must use the previous accepted result as its baseline; an
unaccepted candidate does not count toward the four requested rounds.

For a separate native flamegraph run, enable `frame-profile` at build time and add
`"flamegraph":"OUTPUT.svg"` to the JSON configuration. Sampling starts after warm-up.
Never use a profiled run as FPS acceptance evidence. Earlier kernel benchmark gains,
particularly warm result-cache hits, do not establish application-FPS gains.
