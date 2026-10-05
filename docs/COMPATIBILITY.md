# Compatibility plan

Research snapshot: 2026-10-04.

The imported scaffold currently targets Bevy 0.19.1 only. It contains no ray-traced audio adapter, and it has not passed the 0.17–0.20 or Rust 1.96–1.99 matrix.

## Bevy releases

| Bevy target | State at snapshot | Declared Bevy `rust-version` | Audio source adapter |
| --- | --- | ---: | --- |
| 0.17.3 | Stable | 1.88.0 | `Decodable` has `DecoderItem` |
| 0.18.1 | Stable | 1.89.0 | `Decodable` has `DecoderItem` |
| 0.19.1 | Stable | 1.95.0 | `Decodable` uses rodio `Sample` directly |
| 0.20.0-rc.2 | Release candidate | 1.96.0 | `Decodable` uses rodio `Sample` directly |

Sources: the [Bevy release list](https://github.com/bevyengine/bevy/releases) and the tagged Bevy [0.17.3](https://github.com/bevyengine/bevy/blob/v0.17.3/Cargo.toml), [0.18.1](https://github.com/bevyengine/bevy/blob/v0.18.1/Cargo.toml), [0.19.1](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml), and [0.20.0-rc.2](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/Cargo.toml) manifests. See the versioned [0.17 audio source](https://github.com/bevyengine/bevy/blob/v0.17.3/crates/bevy_audio/src/audio_source.rs), [0.18 source](https://github.com/bevyengine/bevy/blob/v0.18.1/crates/bevy_audio/src/audio_source.rs), [0.19 source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_audio/src/audio_source.rs), and [0.20 release-candidate source](https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_audio/src/audio_source.rs).

The support promise must identify exact patch releases tested. Do not describe 0.20 as stable before its stable release. When 0.20 ships, replace the release-candidate row with the stable release and rerun its checks before publishing support.

## Rust releases

Rust 1.99.0 was released on 2026-10-01. The latest four stable minor versions on the research date are 1.96, 1.97, 1.98, and 1.99. The [Rust 1.99 release announcement](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/) is the current-version source.

The imported workspace and Clippy configuration set MSRV `1.96.1`. Keep that patch minimum unless the Bevy 0.20 support row can be validated against an earlier 1.96 patch. Test the four latest stable minor lines, selecting a current patch release for each line at matrix refresh time. Rust 1.95 cannot be the project minimum while supporting Bevy 0.20 RC2. Refresh patch selections when support policy is updated.

## CI matrix

Run the CPU workspace tests, example builds, and public API compile tests across all 16 combinations:

| Bevy | Rust versions |
| --- | --- |
| 0.17.3 | 1.96, 1.97, 1.98, 1.99 |
| 0.18.1 | 1.96, 1.97, 1.98, 1.99 |
| 0.19.1 | 1.96, 1.97, 1.98, 1.99 |
| 0.20.0-rc.2, then stable 0.20.0 | 1.96, 1.97, 1.98, 1.99 |

Each Bevy adapter selects exactly one Bevy minor through a compatibility feature or version-specific adapter boundary. Verify that the resulting dependency graph does not load multiple Bevy versions for one app. Build every public example against its matching Bevy release. Run CPU tests and integration tests on every matrix cell. Run hardware GPU tests only on supported CI devices, and run mock capability/fallback tests on every cell.

The Bevy versions have separate release cadences and `Decodable` trait shapes. A single source implementation is not assumed to compile against all four. Keep compatibility shims small and test public audio-source construction on each adapter.

The adapter packaging strategy remains open. Separate packages per Bevy minor give each crate one Bevy dependency but create several package names for the same 2D or 3D API. Feature-selected adapter modules keep one package name but need an explicit one-version selection rule and a tested `--all-features` path. Prototype both layouts against the required workspace Clippy command before selecting one.

## Platforms

The first platform proposal is native Windows, Linux, and macOS with CPU execution. GPU support uses Bevy's renderer device when available. Browser/WebGPU, Android, and iOS remain separate validation targets until the template and device/API constraints are confirmed.

## Evidence sources

- [Bevy Audio API, 0.19.1](https://docs.rs/bevy/0.19.1/bevy/audio/index.html)
- [Bevy 0.19 spatial audio examples](https://github.com/bevyengine/bevy/tree/v0.19.1/examples/audio)
- [Rust 1.99.0 release](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/)
