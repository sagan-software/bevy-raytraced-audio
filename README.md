# Bevy Raytraced Audio

CPU acoustic path queries for Bevy, with separate 2D and 3D adapters. The
adapters are opt-in: existing `AudioPlayer`, `PlaybackSettings`, and Bevy audio
plugins keep controlling playback. Mark only the listeners, emitters, and
surfaces that should take part in acoustic queries.

**Current scope:** explicit surfaces control direct-path transmission through
Bevy's existing sink volume. Each material stores per-band absorption energy
`A` and amplitude transmission `T`, with `A + T² <= 1`; remaining energy
contributes to first-order reflections. The core reports reflection paths and
energy by band. Attach `RaytracedAudioReflectionPaths2d` or
`RaytracedAudioReflectionPaths3d` to an emitter to read individual paths. The
adapter does not filter samples or play reflection taps. GPU tracing, mesh
extraction, and reverb DSP are not implemented.

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

Default materials transmit zero amplitude, so the default plugin silences an
emitter behind an opaque surface. Set a fallback linear gain for paths whose
three accumulated band gains are zero with
`RaytracedAudio2dPlugin::default().with_occluded_gain(0.15)?`. Other paths scale
the sink by the arithmetic mean of their three accumulated amplitude gains.

Create a partially transmitting 2D surface like this:

```rust,ignore
let material = AcousticMaterial::default()
    .try_with_transmission(BandGain::try_new(0.2, 0.4, 0.6)?)?;
let wall = RaytracedAudioSurface2d::new(
    Vec2::new(0.0, -1.0),
    Vec2::new(0.0, 1.0),
    material,
)?;
```

Each amplitude transmission must be in `[0, 1]`. For each band, absorption
energy plus squared transmission amplitude must be at most `1`. The remaining
energy contributes to first-order reflections. Direct-path transmission
multiplies the gains of crossed surface primitives. The Bevy sink accepts one
linear volume value, so the adapter uses the arithmetic mean of the low, mid,
and high gains; it does not apply a frequency filter. A surface crossed by a
reflection leg still blocks that reflection, even when its direct transmission
is nonzero.

Surface setup, error handling, and the corresponding 3D version are shown in
the [tutorial examples](docs/EXAMPLES.md).

Both plugins accept `AudioBackendPreference`. `Cpu` is the default. `Auto`
currently logs that GPU compute is unavailable and continues on CPU; the GPU
backend is not implemented yet.

For a Bevy 0.17, 0.18, or 0.20 project, set the adapter feature to `bevy_0_17`,
`bevy_0_18`, or `bevy_0_20`. Disable default features when selecting a version.
Use one Bevy adapter version per application. The current 0.20 adapter targets
`0.20.0-rc.2`; the [official release list](https://github.com/bevyengine/bevy/releases)
has no stable 0.20 release as of 2026-10-06.

## Examples

The [2D GIF](assets/gifs/examples-2d.gif) and [3D GIF](assets/gifs/examples-3d.gif)
show the browser demo routes. The minimal 2D and 3D scenes draw valid
first-order reflection paths in cyan and direct paths in green or red.

![2D tutorial and stress examples](assets/gifs/examples-2d.gif)

![3D tutorial and stress examples](assets/gifs/examples-3d.gif)

Each GIF alternates between the tutorial route and its stress route. The
browser review loaded all four routes, enabled each Web Audio context after a
click, and confirmed that the gallery and stress pages fit a 390-pixel viewport
without horizontal overflow.

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

The workspace MSRV is Rust 1.96.1. CI tests Rust 1.96.1, 1.97.1, 1.98.1, and
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

The 2026-10-06 local check passed 107 workspace tests and reports 99.07% line
coverage, 98.43% region coverage, and 99.57% function coverage. Criterion
completed all 34 workloads declared by the benchmark suite. These figures
describe code coverage and named workload execution; they do not certify
real-time audio quality or rendered frame rate. See
[testing and benchmark results](docs/TESTING-AND-BENCHMARKS.md).

## Performance status

On the local Intel Core i7-8565U laptop CPU, the 2026-10-06 quick benchmark
measured the 16-emitter/32-surface stress schedule at 62.15 microseconds in 2D
and 62.78 microseconds in 3D with the task pool. The serial path measured
65.38 microseconds and 64.51 microseconds. At 64 emitters and 64 surfaces, the
task-pool estimates were 93.39 microseconds in 2D and 115.05 microseconds in
3D; serial estimates were 98.36 microseconds and 113.82 microseconds. These
are short quick-run estimates.

The 128/256 schedule measured 0.361 ms in 2D and 0.427 ms in 3D. The 256/1,024
schedule measured 3.657 ms and 4.719 ms. Each pair lists 2D, then 3D.

The adapter processes fewer than 16 emitters sequentially. At 16 or more, it
uses Bevy's parallel query when the compute task pool is initialized. Without
that pool, it falls back to sequential processing. A separate 30-sample
comparison measured the serial 256/1,024 schedule at 10.726 ms in 2D and
13.043 ms in 3D. The parallel path measured 5.508 ms and 6.006 ms under the
same Bevy multithreaded setup.

Each measured adapter update schedule fits within the 11.11 ms time available
to a 90 FPS frame on this CPU. These measurements exclude rendering, audio
output, and browser presentation, so they do not establish a complete game's
frame rate. A headless SwiftShader run sampled 334 FPS for 2D stress, 4 FPS for
3D stress, 4 FPS for minimal 2D, and 98 FPS for minimal 3D. Those samples do not
establish steady performance on physical graphics hardware. See the
[measured limits and remaining coverage gaps](docs/TESTING-AND-BENCHMARKS.md).

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
