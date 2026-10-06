# Compatibility

Research snapshot: 2026-10-06.

## Bevy versions

| Bevy | Release state | Audio API family | Project support path |
| --- | --- | --- | --- |
| 0.17.3 | Stable | `Decodable` with `DecoderItem` | `bevy-raytraced-audio-compat-0-17` |
| 0.18.1 | Stable | `Decodable` with `DecoderItem` | `bevy-raytraced-audio-compat-0-18` |
| 0.19.1 | Stable | `Decodable` uses rodio `Sample` | `bevy-raytraced-audio-compat-0-19` |
| 0.20.0-rc.2 | Release candidate | `Decodable` uses rodio `Sample` | `bevy-raytraced-audio-compat-0-20` |

The Bevy 0.20 row is provisional because the pinned release is not stable. The
browser gallery and tutorial examples use Bevy 0.19.1. Adapter compatibility
crates isolate the changes to Bevy's audio sink types.

The workspace rejects selecting multiple Bevy-minor features for one adapter
crate. A consuming application should select exactly one feature, for example
`bevy_0_19`.

Sources: [Bevy releases](https://github.com/bevyengine/bevy/releases), tagged
Bevy [0.17.3](https://github.com/bevyengine/bevy/blob/v0.17.3/Cargo.toml),
[0.18.1](https://github.com/bevyengine/bevy/blob/v0.18.1/Cargo.toml),
[0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml), and
[0.20.0-rc.2](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/Cargo.toml)
manifests. See the versioned Bevy audio sources in
[0.17.3](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_audio/src/audio_source.rs),
[0.18.1](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs),
[0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs), and
[0.20.0-rc.2](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_audio/src/audio_source.rs).

## Rust versions

The manifest `rust-version` and Clippy `msrv` are `1.96.1`. The compatibility
workflow tests Rust `1.96.1`, `1.97.0`, `1.98.1`, and `1.99.0`. The 1.96 minimum
matches the Bevy 0.20 release candidate's declared minimum.

## Test matrix

The compatibility workflow runs the core and adapter workspace tests with each Rust version, then checks all four Bevy feature selections. This defines 16 Rust and Bevy combinations. The matrix excludes the Bevy 0.19.1 example package. That package and the four browser builds are tested separately on the locked workspace toolchain.

Run the same checks locally with:

```sh
nix run .#features
cargo test --workspace --exclude bevy-raytraced-audio-examples --locked
```

## Platform scope

The core is `std` Rust and has no graphics or audio-device dependency. Bevy
adapters use Bevy's existing audio plugin. Native examples require Bevy's
platform libraries and an available render device. Browser examples use Bevy
0.19.1, WebAssembly, WebGL2, and Web Audio. The browser examples require a
browser that supports those APIs and a user gesture to start playback.
