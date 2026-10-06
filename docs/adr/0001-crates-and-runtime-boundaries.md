---
status: accepted
date: 2026-10-05
---

# ADR 0001: Separate acoustic simulation from Bevy and audio backends

## Context

The project must support Bevy 0.17 through 0.20, separate 2D and 3D use, optional GPU acceleration, and brownfield installation. Bevy's built-in audio path uses `AudioPlayer<Source>` with `Source: Asset + Decodable`; the public sink exposes playback controls but not a general DSP graph. Bevy 0.17/0.18 and 0.19/0.20 have different `Decodable` trait shapes.

The research found a Bevy integration around Steam Audio, Firewheel and Seedling integrations, and `omg-audio`, a related Rust propagation engine. This project uses an independent solver and API, with no source copied from Vercidium or OMG. The imported template supplied the initial Nix and Bevy scaffold; template gameplay was removed.

## Decision

Keep the acoustic core independent of Bevy and audio devices. Use distinct 2D and 3D Bevy integration crates and one compatibility crate per Bevy minor. Keep the current CPU solver as the executable reference. Keep audio output integration in the adapters so users can retain Bevy's built-in playback path.

The current adapters adjust existing sink volume for direct-path occlusion. They expose reflection results as data. A future processed-audio API must name any source or audio-backend migration it requires and must not promise DSP through the current sink interface.

## Consequences

- Core simulation tests can run without Bevy, a render device, or an audio device.
- Separate Bevy 2D and 3D crates keep dimensional contracts explicit.
- Bevy-minor adapter code may be needed for `Decodable` and other API changes.
- A single plugin cannot promise to add filter and reverb inserts to already-playing built-in audio through the current public sink API.
- GPU acceleration, device selection, and GPU-to-CPU fallback are not implemented by this decision.

## Evidence

- Bevy [0.19 audio source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs)
- Bevy [0.18 audio source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs)
- Bevy [0.19 playback and sink source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/sinks.rs)
- [AudioNimbus Bevy example](https://github.com/MaxenceMaire/audionimbus/tree/master/audionimbus/examples/bevy)
- [Bevy Seedling](https://github.com/CorvusPrudens/bevy_seedling)

## Implementation status

The workspace implements the independent core and adapter boundary, and the local Bevy compatibility checks pass. GPU compute and processed audio remain separate design and implementation gates.
