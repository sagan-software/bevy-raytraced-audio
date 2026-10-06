# Bevy audio API research

Research date: 2026-10-06. The official release list still names Bevy 0.19.1 as the latest stable release and 0.20.0-rc.2 as the newest 0.20 release candidate. Tagged Bevy source files below were checked for each supported minor. The Bevy 0.20 milestone remains open, so the project continues to label 0.20 support as release-candidate compatibility.

## Built-in playback model

Bevy's [audio module](https://docs.rs/bevy/0.19.1/bevy/audio/index.html) exposes `AudioPlayer`, `AudioSource`, `PlaybackSettings`, `AudioSink`, `SpatialAudioSink`, `SpatialListener`, `SpatialScale`, `GlobalVolume`, `Decodable`, `AddAudioSource`, `Volume`, and related types. The official `AudioSource` asset exposes its encoded bytes in Bevy 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2. This permits a custom source asset to wrap the same encoded file without changing Bevy's loader for `.wav`, `.ogg`, `.flac`, or `.mp3` files.

An `AudioPlayer<Source>` stores an asset handle, where `Source: Asset + Decodable`. `PlaybackSettings` controls initial mode, volume, speed, pause/mute state, start position, duration, and the spatial flag. Bevy adds `AudioSink` or `SpatialAudioSink` after playback starts. Later playback changes go through the sink; changes to `PlaybackSettings` do not change an already-playing sound. See [Bevy's 0.19.1 playback implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio.rs).

`AudioSinkPlayback` exposes operations such as volume, speed, seek, pause, play, mute, stop, and completion status. The underlying rodio sink fields are private. `SpatialAudioSink` adds ear/listener/emitter positioning. These APIs do not expose a general insertable low-pass, convolution, reflection, or reverb node. The official [0.19.1 sink source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/sinks.rs) is the boundary to recheck during the API spike.

Bevy documents its built-in spatial mode as simple stereo panning. It does not provide HRTF or other high-quality 3D rendering through this path. `SpatialListener` expects one listener entity and uses `Transform`/`GlobalTransform` plus ear offsets.

## `Decodable` extension point

Bevy's [Decodable example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/decodable.rs) implements a custom asset, decoder iterator, `rodio::Source`, and `Decodable`, then registers it with `AddAudioSource::add_audio_source`. A custom decoder is created for playback. This is the public seam to prototype because it retains Bevy's `AudioPlayer`, asset, and playback lifecycle for a custom source asset.

There is a version boundary. Bevy 0.17 and 0.18 define the associated `DecoderItem` and constrain it through `rodio::Sample`; Bevy 0.19 and 0.20 use `Iterator<Item = rodio::Sample>` directly. Each version requires the custom source asset to implement `Asset + Decodable`. Compare the tagged [0.17.3 source](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_audio/src/audio_source.rs), [0.18.1 source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs), [0.19.1 source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs), and [0.20.0-rc.2 source](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_audio/src/audio_source.rs).

The custom source creates a decoder per playback, but `decoder()` receives neither the Bevy entity nor its acoustic result. An asset shared by multiple players therefore cannot hold independent live effect state for each player. Any live source integration must establish per-emitter ownership or use a backend API that supplies per-voice state. This ownership boundary remains a design gate for audible reflections.

## Official spatial examples

- [Spatial audio 2D](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/spatial_audio_2d.rs) uses `AudioPlayer`, `PlaybackSettings::LOOP.with_spatial(true)`, `SpatialListener`, transforms, and a 2D visual scene. Positions remain `Vec3` with z set for the planar scene.
- [Spatial audio 3D](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/spatial_audio_3d.rs) uses the same playback components in a 3D scene and queries `SpatialAudioSink` for runtime control.
- [Decodable](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/audio/decodable.rs) shows custom generated audio and asset registration.
- The [Bevy examples index](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/README.md) also lists audio control, sound effects, soundtrack, and pitch examples that should inform the tutorial set.

## Integration conclusion

Inference from the public API: a plugin can keep the built-in `AudioPlugin` and modify sink controls Bevy exposes, including volume. The custom `Decodable` seam requires a custom source asset, so it does not add processing to an existing `AudioPlayer<AudioSource>`. Bevy does not expose a public sink insertion API for arbitrary live filtering, convolution, early reflections, or reverb.

For full effects, prototype a custom `Decodable` asset and decoder or an optional audio graph adapter. A Seedling/Firewheel adapter gives a graph for custom DSP, but Seedling asks projects to disable Bevy's `bevy_audio` feature and use its own `SamplePlayer`. That is a larger migration and should remain optional.

## API design notes

- Keep Bevy's `AudioPlayer`, `PlaybackSettings`, and `SpatialListener` when their behavior is preserved.
- Require an explicit acoustic-emitter opt-in so an installed plugin does not alter every sound.
- Require explicit acoustic surfaces and materials; render materials do not supply acoustic coefficients by default.
- State exactly which processed playback path adds or wraps a source type.
- Test startup playback settings, looping, paused start, volume/speed control, despawn/remove modes, missing assets, source reuse, and concurrent playback under each trait version.
