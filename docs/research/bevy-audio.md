# Bevy audio API research

Research date: 2026-10-04. The API inventory uses Bevy 0.19.1 documentation and tagged sources; the implementation must repeat the check for every supported minor.

## Built-in playback model

Bevy's [audio module](https://docs.rs/bevy/0.19.1/bevy/audio/index.html) exposes `AudioPlayer`, `AudioSource`, `PlaybackSettings`, `AudioSink`, `SpatialAudioSink`, `SpatialListener`, `SpatialScale`, `GlobalVolume`, `Decodable`, `AddAudioSource`, `Volume`, and related types.

An `AudioPlayer<Source>` stores an asset handle, where `Source: Asset + Decodable`. `PlaybackSettings` controls initial mode, volume, speed, pause/mute state, start position, duration, and the spatial flag. Bevy adds `AudioSink` or `SpatialAudioSink` after playback starts. Later playback changes go through the sink; changes to `PlaybackSettings` do not change an already-playing sound. See [Bevy's 0.19.1 playback implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio.rs).

`AudioSinkPlayback` exposes operations such as volume, speed, seek, pause, play, mute, stop, and completion status. The underlying rodio sink fields are private. `SpatialAudioSink` adds ear/listener/emitter positioning. These APIs do not expose a general insertable low-pass, convolution, reflection, or reverb node. The official [0.19.1 sink source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/sinks.rs) is the boundary to recheck during the API spike.

Bevy documents its built-in spatial mode as simple stereo panning. It does not provide HRTF or other high-quality 3D rendering through this path. `SpatialListener` expects one listener entity and uses `Transform`/`GlobalTransform` plus ear offsets.

## `Decodable` extension point

Bevy's [Decodable example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/decodable.rs) implements a custom asset, decoder iterator, `rodio::Source`, and `Decodable`, then registers it with `AddAudioSource`. A custom decoder is created for playback. This is the best public seam to prototype because it retains Bevy's `AudioPlayer`, asset, and playback lifecycle.

There is a version boundary. Bevy 0.17 and 0.18 define `DecoderItem` and constrain the decoder item through `rodio::Sample`; Bevy 0.19 and 0.20 use `rodio::Sample` directly on the decoder type. Compare the tagged [0.17.3 source](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_audio/src/audio_source.rs), [0.18.1 source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs), [0.19.1 source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs), and [0.20.0-rc.2 source](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_audio/src/audio_source.rs).

Prototype per-playback parameter ownership before choosing this API. A custom source can create a decoder per playback, but it does not receive the Bevy entity or acoustic result as a `decoder()` argument. Shared asset state can accidentally make multiple players share effect state. The design must show how each voice reads its own bounded, lock-free acoustic parameters.

## Official spatial examples

- [Spatial audio 2D](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/spatial_audio_2d.rs) uses `AudioPlayer`, `PlaybackSettings::LOOP.with_spatial(true)`, `SpatialListener`, transforms, and a 2D visual scene. Positions remain `Vec3` with z set for the planar scene.
- [Spatial audio 3D](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/spatial_audio_3d.rs) uses the same playback components in a 3D scene and queries `SpatialAudioSink` for runtime control.
- [Decodable](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/decodable.rs) shows custom generated audio and asset registration.
- The [Bevy examples index](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md) also lists audio control, sound effects, soundtrack, and pitch examples that should inform the tutorial set.

## Integration conclusion

Inference from the public API: a plugin can keep the built-in `AudioPlugin` and modify sink controls Bevy exposes, including volume. A plugin cannot promise arbitrary live filtering, convolution, early reflections, or reverb on an existing `AudioPlayer<AudioSource>` through a public sink insertion API.

For full effects, prototype a custom `Decodable` asset and decoder or an optional audio graph adapter. A Seedling/Firewheel adapter gives a graph for custom DSP, but Seedling asks projects to disable Bevy's `bevy_audio` feature and use its own `SamplePlayer`. That is a larger migration and should remain optional.

## API design notes

- Keep Bevy's `AudioPlayer`, `PlaybackSettings`, and `SpatialListener` when their behavior is preserved.
- Require an explicit acoustic-emitter opt-in so an installed plugin does not alter every sound.
- Require explicit acoustic surfaces and materials; render materials do not supply acoustic coefficients by default.
- State exactly which processed playback path adds or wraps a source type.
- Test startup playback settings, looping, paused start, volume/speed control, despawn/remove modes, missing assets, source reuse, and concurrent playback under each trait version.
