# Testing and benchmarks

## Coverage target

Target at least 90% executable-line coverage across testable production crates, measured with `cargo llvm-cov`. Report production crates separately and list excluded generated files. Do not combine coverage across unrelated examples to hide gaps. Treat line coverage as execution evidence; focused assertions establish behavior. Record branch coverage separately when the tooling reports it.

The 90% target applies to production behavior, not to coverage of Criterion harness code. Compile all benchmark targets and run each named benchmark scenario. Cover benchmark input classes through the benchmark catalogue and retain stable inputs, not a percentage from the benchmark source itself.

## Test layers

### Core tests

- Validate acoustic material fields independently, including finite bounds, invalid coefficients, and boundary values.
- Trace empty geometry, one surface, a grazing intersection, parallel rays, reversed winding, degenerate primitives, and transformed primitives.
- Check direct visibility, repeated blockers, transmission, and the exact order of accumulated material interactions.
- Verify reflection path length and arrival order for a room with an analytical answer.
- Check late-reverb decay against fixed parameterized fixtures.
- Verify deterministic seeded output and stable collection ordering.
- Exercise scene build, no-op update, transform-only change, material-only change, and geometry replacement.
- Test non-finite and extreme coordinates before they reach a BVH or shader.

### Bevy integration tests

- Test plugin registration with and without `bevy_audio` and `bevy_render` where supported.
- Verify an app with existing `AudioPlayer<AudioSource>` remains unchanged when the plugin is installed and the entity has no acoustic marker.
- Verify opted-in source setup, `PlaybackSettings` startup behavior, looping, pause, mute, volume, speed, start position, duration, and despawn/remove modes.
- Verify missing and late-loaded assets, multiple players sharing one source asset, entity despawn, source reuse, and listener changes.
- Test Bevy 0.17/0.18 and 0.19/0.20 `Decodable` adapter implementations independently.
- Verify surface opt-in, invalid mesh attributes, missing material, transform changes, and unsupported geometry diagnostics.

### Audio output tests

Use generated tones and impulses with fixed sample rates and block sizes. Assert channel count, onset times, direct gain, filter response, early-reflection delays, reverb tail decay, clipping bounds, and continuity across block boundaries. Store full-output snapshots only where deterministic sample output is promised; keep approximate acoustic comparisons numeric and tolerance-bound.

For concurrency tests, stall simulation completion while processing continues. Assert that the audio callback consumes the latest completed snapshot without blocking, taking a lock, allocating, or growing an unbounded queue.

### GPU tests

- Compare CPU and GPU results for the same seeded scene and state explicit tolerances.
- Test zero geometry, one primitive, capacity limits, dispatch boundaries, and maximum supported scene size.
- Inject adapter absence, device creation failure, shader compilation failure, dispatch failure, and device loss.
- Verify each recoverable GPU failure selects CPU and emits a diagnostic reason.
- Keep a GPU test suite optional when CI has no suitable adapter; still run a mock capability/fallback suite on every CI host.

## Benchmarks

Use Criterion with release-like compiler settings and a documented environment. Bench these independent workloads:

| Benchmark | Parameters to sweep | Metric |
| --- | --- | --- |
| 2D scene build | segment count, material count | build time, allocations and allocated bytes |
| 3D BVH build/refit | triangle count, changed fraction | build/refit time, allocations and peak retained geometry |
| Direct-path simulation | emitter/listener count, ray count, occluder count | time per update, rays per second |
| Early reflections | ray count, bounce limit, reflection cap | time per update, result count |
| Dynamic update | moving surfaces, static surfaces, update cadence | update time, geometry copied |
| CPU/GPU crossover | scene size, rays, update rate, adapter | end-to-end time including upload and synchronization |
| Audio processing | channel count, sample rate, block size, active voices | time per block and deadline margin |

Record CPU/GPU model, operating system, driver, Rust compiler, Bevy version, feature set, input seed, geometry and ray counts, and Criterion baseline. Do not set a performance threshold until representative baselines exist. Compare CPU and GPU end-to-end time, not only shader execution.

## Imported scaffold gates

The current Nix template says to run development commands through Nix, not a host-installed Rust toolchain. Its full `nix run .#check` workflow covers formatting, Clippy, Bevy feature checks, Bevy-specific lints, tests, doctests, private docs, benchmark compilation, and coverage. `nix flake check` exposes those checks as isolated derivations. These gates currently validate the inherited Bevy 0.19.1 network demo.

The target library gates must cover the shared core and each Bevy adapter. Use the imported Nix workflows while the template policy applies:

```sh
nix run .#check
nix flake check
nix build
nix develop --command nix run github:sagan-software/dylints/483b64d83e38352994d509eacf4a56db1892f1a3 -- --repo . --fast
```

The Bevy adapters may require separate feature builds because one Cargo invocation cannot contain multiple incompatible Bevy minor versions. Record the exact matrix commands and test results after selecting the crate and adapter layout. The imported flake currently sets its coverage floor to 50%; raise the project production-code threshold to at least 90% after the audio crate structure exists. Do not report the inherited baseline's coverage as audio-library coverage.
