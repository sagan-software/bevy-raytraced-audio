# Project plan

## Outcome

Build a modular Rust acoustic propagation system for Bevy with separate 2D and 3D integrations, an always-available CPU backend, an optional GPU compute backend, and a low-friction opt-in API for existing projects.

The first review covers research and design. It does not authorize implementation to begin until the plan, solver-reuse decision, and template import are reviewed.

## Acceptance criteria

- A Bevy app can add the plugin with a small configuration value and retain its existing Bevy audio plugin, playback settings, asset loading, and unrelated entities.
- An app can opt selected listeners, emitters, and surfaces into acoustic processing. Unmarked audio and geometry retain their existing behavior.
- There are separate public 2D and 3D integration crates over shared acoustic code.
- CPU simulation works without a GPU, renderer, or GPU driver. Automatic backend selection uses CPU when GPU setup or execution is unavailable.
- GPU acceleration is optional and reports its selected backend and fallback reason through diagnostics.
- Existing `AudioPlayer`/`AudioSource` projects have a documented migration path. Full filtering and reverb do not silently claim to modify Bevy playback where Bevy exposes no such effect API.
- Supported Bevy releases are tested individually: 0.17, 0.18, 0.19, and 0.20. Before 0.20 is stable, test its current release candidate and label that support as provisional.
- The package declares Rust 1.96 as its minimum and CI tests stable Rust 1.96, 1.97, 1.98, and 1.99, matching the four stable minor releases available on 2026-10-04.
- Production-code line coverage is at least 90%, with branch gaps reported separately. Tests cover each reachable changed branch and each backend fallback.
- Criterion benchmarks cover acoustic build, direct-path queries, reflection queries, scene updates, and CPU/GPU parity on fixed inputs. Performance claims include the input size, hardware, compiler, build profile, and measured output.
- The copied Bevy template supplies the project's initial scaffold, Nix setup, profiles, and preferred tooling. Its origin, commit, retained settings, and temporary demo baseline are recorded in [template import research](research/template.md).
- `nix build`, `nix run`, and `nix flake check` have documented and tested behavior. The default run target launches a small demo; named 2D and 3D demos are available.
- Every downloaded asset has a recorded source, creator, exact license, retrieval date, checksum, and any modifications.
- The project uses the Sagan Dylints runner and a pinned Dylint library configuration after verifying that the configured checks work with the project's Bevy and Rust matrix.
- The solver and code reuse policy is explicit. The current recommendation is an independent implementation, using `omg-audio` as a research reference and copying no source unless a separate audit and approval changes that decision.

## Work sequence

### 0. Import and inspect the template

The exact template source is now copied from commit `da85b65ffbad26f78d23c953d379a5b0d624e1ed`. Keep its networked 2D app only as a temporary scaffold baseline while the owner reviews this plan. After approval, retain relevant Nix, toolchain, lint, profile, test, benchmark, and AI tooling settings; replace or remove template-specific gameplay and dependencies.

Exit evidence: template origin and commit recorded; imported files are committed; Nix flake and package skeleton intact; Rust and Clippy MSRV fields located; no template secrets or external game assets copied. The donor gameplay remains clearly labeled as temporary baseline code.

### 1. Settle the audio integration boundary

Prototype the Bevy `Decodable` path in the Bevy 0.17/0.18 trait shape and the 0.19/0.20 trait shape. Verify per-playback decoder state, `AudioPlayer` lifecycle, asset loading, spatial transforms, volume controls, and format-feature behavior. Compare this path with an optional `bevy_seedling`/Firewheel node path.

Exit evidence: one minimal brownfield app per Bevy trait family plays an unprocessed sound and an acoustically processed sound; the example preserves `PlaybackSettings` behavior; the API clearly states whether a user must change the source asset/player type or audio backend.

### 2. Settle the solver and reuse boundary

Compare an independent solver with an adapter or code reuse proposal based on `omg-audio`. Review the exact package licenses, algorithm and data contracts, test evidence, current GPU implementation state, Bevy integration effort, and the different wgpu versions across Bevy releases. Keep proprietary Vercidium source and SDKs out of the project. The default recommendation is an independent solver and public API, with prior art informing requirements and tests.

Exit evidence: an approved solver ownership decision with license notes and a documented reason for adopting or rejecting code reuse.

### 3. Define the acoustic model and public contracts

Choose material coefficients, direct-path transmission, early-reflection budget, late-reverb model, coordinate rules, scene invalidation, update cadence, deterministic seeding, and error categories. Document each value's unit and range. Separate malformed configuration from unavailable runtime capability.

Exit evidence: glossary, API sketch, compatibility table, core data contracts, and boundary-test list are approved before implementation.

### 4. Implement the shared CPU core

Build geometry representations, material validation, acceleration structures, deterministic ray sampling, and immutable simulation outputs without a Bevy or GPU dependency. Keep DSP and audio-device code outside the acoustic core.

Exit evidence: deterministic unit tests cover geometric boundaries, numerical input failures, material energy limits, scene rebuild/update behavior, and stable output for fixed seeds. Core line coverage reaches 90% or more.

### 5. Implement separate Bevy 2D and 3D adapters

Add ECS components/resources, transform and mesh extraction, scene update tracking, listener/emitter collection, diagnostics, and CPU execution. A 2D surface remains planar; render-layer z ordering has no acoustic meaning unless a later layer model is explicitly designed.

Exit evidence: each adapter has a minimal app test and a demo. Existing unmarked playback and entities remain unchanged.

### 6. Integrate full audio effects

Apply direct-path attenuation and spectral filtering, early reflections, and late reverb through the approved output path. Make updates safe for real-time audio processing: no waits, locks, unbounded queues, or allocation in the audio callback. Keep Bevy's built-in playback route usable for effects it can express.

Exit evidence: captured audio fixtures assert timing, channel layout, attenuation/filter response, reflection arrival, reverb decay, and deterministic output within explicit numeric tolerances.

### 7. Add optional GPU compute

Implement a GPU backend behind an opt-in crate or feature. Reuse Bevy's render device when the app has one; avoid requiring `bevy_render` for CPU-only users. Define upload, synchronization, capacity, device-loss, shader-failure, and fallback behavior before enabling GPU by default.

Exit evidence: feature builds on all supported Bevy minors; a CPU-only build has no GPU dependency; a software or hardware adapter test compares GPU output with CPU output; missing adapter, device loss, and shader failure select CPU without blocking playback.

### 8. Complete examples and asset provenance

Add tutorial-style, commented examples. Use generated tones and procedural geometry first. Add CC0 assets only when they improve a specific demonstration.

Exit evidence: every example builds through Nix and Cargo; each interactive demo describes expected controls and audible/visible behavior; asset metadata is complete.

### 9. Complete compatibility and release gates

Run the Bevy 0.17/0.18/0.19/0.20 and Rust 1.96/1.97/1.98/1.99 matrix, Nix checks, project lints, Dylints, coverage, and benchmarks. Replace the 0.20 release-candidate pin when a stable 0.20 release is available.

Exit evidence: all required matrix cells pass; `nix build`, `nix run`, and `nix flake check` pass from a clean checkout; 90% production line coverage is met; benchmark baselines are recorded; known platform gaps are listed.

## Decisions to review

1. Approve the Bevy audio integration boundary in [the proposed architecture](DESIGN.md). The built-in sink exposes playback controls, while full reverb and filtering require a processed decoder or graph node.
2. Confirm the proposed planar XY definition for 2D mode and whether layered 2D acoustics belong in the first release.
3. Confirm whether the initial support promise targets Windows, Linux, and macOS, with browser/WebGPU support treated as later validation.
4. Confirm which template-only development tools should remain after the demo is replaced. The import currently retains AI tooling, editor tooling, networking, physics, and particles from the source template.
5. Confirm the solver policy. The recommendation is an independent implementation with no copied `omg-audio` code; details are in [prior-art research](research/prior-art.md).

## Risks tracked

- Bevy 0.17/0.18 and 0.19/0.20 expose incompatible `Decodable` trait shapes.
- Bevy's built-in audio player does not expose a general live effect insert, so a strict no-code-change audio drop-in promise is not supported by the current public API.
- GPU compute may be slower than CPU for small scenes or small ray budgets. Automatic selection needs measured thresholds, not GPU presence alone.
- Dynamic mesh extraction and BVH rebuilding may cost more than steady-state ray tracing.
- A Dylint library may use a nightly compiler newer or older than a supported stable compiler and may not check every dependency combination.
- Bevy 0.20 is pre-release at the research snapshot date.
