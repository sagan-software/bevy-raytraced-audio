# Examples

The nine Bevy examples use Bevy 0.19.1 on native platforms and in the browser.
Each browser page puts the running scene first, followed by its description,
controls, and full highlighted Rust source. The gallery launches the sandbox
from a static preview; click **Enable sound** and then the canvas to play and
use keyboard controls.

The demos use CC0 guitar, rain, forest, footsteps, and one-shot recordings.
[Audio credits and processing details](ASSETS.md) identify every source. The
example package enables Bevy's `vorbis` decoder, and the `audio_fixture` tests
check the Ogg recordings and their processed output. The old chime remains only
as a WAV decoder regression fixture.

| Example         | Scene and controls                                                                                         | What it demonstrates                                                                                                     |
| --------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `showcase`      | F first person, mouse facing, WASD walk, E openings, X wall sections, 0–8 scenarios, H guide               | CC0 modular houses with speech, basement/upstairs runners, operable doors/windows and a stream/bridge; full 3D acoustics |
| `minimal_2d`    | Two rooms and a music speaker; Space toggles the door, WASD/arrows move, V toggles rays                    | Bounced paths around a doorway and frequency-dependent muffling through a closed wall                                    |
| `reverb_2d`     | 1/2/3 select bathroom/hall/cathedral, C toggles clutter, Space claps                                       | Room-dependent reverb, pre-delay, and echo rays                                                                          |
| `ambience_2d`   | WASD/arrows move, 1/2/3 toggle door/west/north window, B shows bounces                                     | Escaped rays measure outdoor exposure; yellow rays steer the rain toward openings                                        |
| `permeation_2d` | The speaker switches between wood and stone rooms automatically; Tab switches immediately                  | Per-band transmission through different materials and multiple wall faces                                                |
| `minimal_3d`    | Two rooms built from triangles; WASD/arrows move, Space toggles the door, V toggles rays                   | Full 3D tracing including acoustic floors and ceilings; the ceiling is hidden visually                                   |
| `forest_3d`     | WASD/arrows walk, Shift runs, hold left mouse to orbit, wheel zooms, B toggles paths, F1 shows diagnostics | Forest ambience, explicit stone/brush surfaces, and first-order path data                                                |
| `stress_2d`     | 16 moving emitters and 32 wall segments                                                                    | 2D update workload and rendered frame-rate readout                                                                       |
| `stress_3d`     | 16 moving emitters and 32 wall triangles                                                                   | Matching 3D workload and frame-rate readout                                                                              |

## Watch and hear the sandbox

The sandbox is now a full 3D acoustic village with two furnished modular houses,
working single/double doors and windows, basement and upstairs stairs, NPC
footsteps, recorded speech, and a stream with a bridge. Kenney's CC0 kits provide
the structural models. See the [audit and scenario guide](SANDBOX-AUDIT.md) for
repeatable tests and the limits of the acoustic model.

The third-person humanoid faces the mouse; its camera stays independent. F
enters first person, Esc returns overhead, WASD walks, and Q/R orbits or turns.
Blue and coral ear cups identify left/right relative to the character. E opens
or closes the nearest opening; O/C opens/closes all. X removes or rebuilds a
yellow-outlined wall. Upper floors hide visually in overhead view but remain
acoustically active. H hides the guide, and V displays acoustic rays.

Keys 1–8 isolate speech, footsteps above/below, bridge/water, window, basement,
and upstairs scenarios; 0 returns to exploration. Tab bypasses processing,
M mutes everything, −/+ adjusts master volume, N toggles quiet rain, G toggles
own footsteps, and P pauses the runners. Rain starts off. B switches measured
HRTF headphone cues independently of room processing. T cycles empty tile,
empty carpet, and furnished carpet; J plays a clap and K plays a gunshot for
repeatable indoor/outdoor comparisons.

`RaytracedAudioPlayer` applies a two-band filter and Schroeder reverb to decoded
audio. Plain `AudioPlayer` emitters receive scalar volume changes. This is an
energy-based acoustic approximation, not a wave or structure-borne simulation;
the showcase opts into measured HRTF stereo headphone rendering. The generic
KEMAR response is not personalized and clamps directions below -40° to the
lowest measurement. See [research and limitations](research/vertical-audio-and-materials.md).

## Run native examples

```sh
nix run .#showcase
nix run .#demo-2d
nix run .#demo-3d
nix run .#reverb-2d
nix run .#ambience-2d
nix run .#permeation-2d
nix run .#forest-3d
nix run .#stress-2d
nix run .#stress-3d
```

Inside `nix develop`, use `cargo run --locked --package
bevy-raytraced-audio-examples --example showcase` (or any name in the table).
Native examples need a display, graphics adapter, and audio output to play.
The core-only `cargo run --example trace_2d` prints direct transmission and
first-order reflection geometry without those devices.

## Build browser examples

```sh
nix run .#web-build
nix run .#web-serve
```

The local server listens on `http://127.0.0.1:8000`. The builder compiles all
nine WebGL2 examples, generates 19 HTML pages from
`website/example-catalog.mjs`, embeds each Rust source, copies the audio files,
and builds the Markdown book into a unique directory under `target/`.
`website/` is source input, not a ready-to-serve site.

The loader checks WebGL2 and waits for canvas sizing and successive draw calls
before reporting readiness. It resumes browser-suspended audio from a trusted
click or key gesture. A running Web Audio graph proves that playback is queued;
physical speaker output and subjective sound quality still need a listening
check.

## Performance evidence

The measurements and GIFs from 2026-10-06 predate the listener-ray and processed
audio update. They cover the original five scenes and direct/image-source
queries. They must not be used as measurements of the new sandbox, DSP, or
nine-route gallery. See [testing and benchmarks](TESTING-AND-BENCHMARKS.md) for
the historical numbers and their limits. Fresh physical-display and audio-output
measurements remain outstanding.

## Optional image-source paths

`RaytracedAudioReflectionPaths2d` and `RaytracedAudioReflectionPaths3d` remain
available for geometric first-order paths. Attach one to an emitter and call
`paths()` for its surface index, reflection point, image source, total distance,
and relative energy by band. The forest example still visualizes these paths.
These components are separate from the listener-ray debug plugins and do not
create individual audible reflection taps.
