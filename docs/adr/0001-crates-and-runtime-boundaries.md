---
status: proposed
date: 2026-10-04
---

# ADR 0001: Separate acoustic simulation from Bevy and audio backends

## Context

The project must support Bevy 0.17 through 0.20, separate 2D and 3D use, CPU fallback, optional GPU acceleration, and brownfield installation. Bevy's built-in audio path uses `AudioPlayer<Source>` with `Source: Asset + Decodable`; the public sink exposes playback controls but not a general DSP graph. Bevy 0.17/0.18 and 0.19/0.20 have different `Decodable` trait shapes.

The research found both a Bevy integration around Steam Audio and a direct Firewheel/Seedling integration. It also found `omg-audio`, a closely related Rust propagation engine with an Apache-2.0 core. The current recommendation is an independent solver and API, with no source copied from either Vercidium or OMG. The owner can request a separate reuse audit before implementation. The template expected by the owner is not available on the current host, so the final crate and Nix layout must wait for its import.

## Proposed decision

Keep the acoustic core independent of Bevy and audio devices. Publish distinct 2D and 3D Bevy integration crates. Treat CPU as the required reference backend and GPU compute as optional. Keep audio output integration as a separate adapter so users do not have to replace Bevy audio merely to add the plugin. Implement the solver independently unless an approved follow-up audit selects reuse of `omg-audio` code.

Before committing to the API, prototype the custom `Decodable` route and compare it with a Seedling/Firewheel graph node. Provide a built-in sink path only for effects Bevy can express. The full-effects API must name any source or audio-backend migration it requires.

## Consequences

- Core simulation tests can run without Bevy, a render device, or an audio device.
- Separate Bevy 2D and 3D crates keep dimensional contracts explicit.
- Bevy-minor adapter code may be needed for `Decodable` and other API changes.
- A single plugin cannot promise to add filter and reverb inserts to already-playing built-in audio through the current public sink API.
- GPU failure cannot stop CPU-capable applications from producing the core simulation output.

## Evidence

- Bevy [0.19 audio source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs)
- Bevy [0.18 audio source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs)
- Bevy [0.19 playback and sink source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/sinks.rs)
- [AudioNimbus Bevy example](https://github.com/MaxenceMaire/audionimbus/tree/master/audionimbus/examples/bevy)
- [Bevy Seedling](https://github.com/CorvusPrudens/bevy_seedling)

## Review gate

Keep this ADR proposed until the owner approves the audio migration and solver-reuse boundaries, the template is imported, and the compatibility spike passes.
