# Bevy Raytraced Audio

CPU acoustic path queries for Bevy, with separate 2D and 3D adapters. The
adapters are opt-in: existing `AudioPlayer`, `PlaybackSettings`, and Bevy audio
plugins keep controlling playback. Mark only the listeners, emitters, and
surfaces that should take part in acoustic queries.

**Current scope:** direct-path occlusion changes the existing Bevy sink volume.
The core also reports first-order reflection paths and band energy. Those
reflection values do not yet modify audio samples. There is no GPU backend,
mesh extraction, filter, or reverb DSP in this version.

## Quick start

Choose one adapter crate. This example uses Bevy 0.19:

```toml
[dependencies]
bevy-raytraced-audio-2d = { git = "https://github.com/sagan-software/bevy-raytraced-audio", default-features = false, features = ["bevy_0_19"] }
```

Add its plugin next to your existing Bevy plugins, then opt in the relevant
entities:

```rust
use bevy::prelude::*;
use bevy_raytraced_audio_2d::{
    RaytracedAudio2dPlugin, RaytracedAudioEmitter2d, RaytracedAudioListener2d,
};

fn configure_audio(app: &mut App) {
    app.add_plugins(RaytracedAudio2dPlugin::default());
}

fn spawn_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("audio.ogg")),
        PlaybackSettings::LOOP.with_spatial(true),
        RaytracedAudioEmitter2d,
        Transform::from_xyz(4.0, 0.0, 0.0),
    ));
    commands.spawn((
        RaytracedAudioListener2d,
        Transform::from_xyz(-4.0, 0.0, 0.0),
    ));
}
```

The default plugin silences an emitter when an explicit surface blocks its
direct path. Configure another linear gain with
`RaytracedAudio2dPlugin::default().with_occluded_gain(0.15)?`. Surface setup,
error handling, and the corresponding 3D version are shown in the
[tutorial examples](docs/EXAMPLES.md).

For a Bevy 0.17, 0.18, or 0.20 project, set the adapter feature to `bevy_0_17`,
`bevy_0_18`, or `bevy_0_20`. Disable default features when selecting a version.
Use one Bevy adapter version per application. The current 0.20 adapter targets
`0.20.0-rc.2`; the stable 0.20 release is not available as of 2026-10-05.

## Examples

The [2D GIF](assets/gifs/examples-2d.gif) and [3D GIF](assets/gifs/examples-3d.gif)
show the four browser demo routes.

![2D tutorial and stress examples](assets/gifs/examples-2d.gif)

![3D tutorial and stress examples](assets/gifs/examples-3d.gif)

Open the [live browser gallery](https://sagan-software.github.io/bevy-raytraced-audio/)
to run each example in its own WebAssembly page and enable browser audio with a
click. See the [example guide](docs/EXAMPLES.md) for controls and workload sizes.

Run the native examples with:

```sh
nix run .#demo-2d
nix run .#demo-3d
nix run .#stress-2d
nix run .#stress-3d
```

## Supported versions

| Bevy adapter | Bevy version | Status |
| --- | --- | --- |
| `bevy_0_17` | 0.17.3 | Feature and integration tested |
| `bevy_0_18` | 0.18.1 | Feature and integration tested |
| `bevy_0_19` | 0.19.1 | Default feature; examples use this version |
| `bevy_0_20` | 0.20.0-rc.2 | Release-candidate compatibility only |

The workspace MSRV is Rust 1.96.1. CI tests Rust 1.96.1, 1.97.0, 1.98.1, and
1.99.0. The local verification host uses Rust 1.99.0. See the
[compatibility notes](docs/COMPATIBILITY.md) for the tested matrix and release
sources.

## Build and verification

Nix supplies the native Bevy libraries, Rust tools, benchmark runner, browser
toolchain, and Markdown book builder:

```sh
nix develop
nix build
nix run .#check
nix flake check
```

Build the browser gallery and book locally, then serve them on
`http://127.0.0.1:8000`:

```sh
nix run .#web-build
nix run .#web-serve
```

Run the test suite and Criterion workload set with:

```sh
nix run .#test
nix run .#bench -- --quick --noplot
```

The latest local coverage run reports 98.18% executable-line coverage,
100.00% region coverage, and 98.98% function coverage. The quick Criterion run
executes all 22 named workload scenarios. These figures describe code and
workload execution coverage; they do not certify real-time audio quality or
rendered frame rate. See [testing and benchmark results](docs/TESTING-AND-BENCHMARKS.md).

## Performance status

The 128-emitter and 256-surface adapter schedule benchmark measured 1.52 ms in
2D and 4.51 ms in 3D on an Intel Core i7-8565U laptop CPU. This benchmark
measures the Bevy schedule without rendering or browser presentation.

The 90 FPS target is not verified. On this host, the visible browser check uses
SwiftShader through Xvfb and reports roughly 1–5 FPS. Native display validation
is unavailable here. See the [measured limits and remaining coverage gaps](docs/TESTING-AND-BENCHMARKS.md).

## Documentation

- [Markdown book](docs/README.md)
- [Architecture and invariants](docs/DESIGN.md)
- [Example catalogue](docs/EXAMPLES.md)
- [Bevy and Rust compatibility](docs/COMPATIBILITY.md)
- [Tests and benchmark results](docs/TESTING-AND-BENCHMARKS.md)
- [Nix commands](docs/NIX.md)
- [Sagan Dylints setup](docs/DYLINTS.md)
- [Research and sources](docs/research/README.md)
- [Delivery plan and remaining work](docs/PLAN.md)

## License

The code is available under the MIT license. See [LICENSE](LICENSE).
