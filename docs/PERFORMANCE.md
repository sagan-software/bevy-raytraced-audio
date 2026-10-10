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
The existing Nix check's 91% threshold is historical, not proof of 100% coverage.

```sh
cargo llvm-cov --workspace --locked --html --output-dir target/coverage
cargo llvm-cov report --show-missing-lines
# Strict requested line-coverage acceptance (fails if any production line is missed):
cargo llvm-cov report --ignore-filename-regex '(^|/)(tests|benches|examples)/' \
  --fail-under-lines 100
```

Portable generic native code, a separately paired CPU-native build, single/multi
worker configurations and real browser WASM provide different execution
conditions. They do not establish Windows, macOS, ARM or physical audio-device
performance unless those systems were actually measured.
