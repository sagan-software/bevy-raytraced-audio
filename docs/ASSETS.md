# Asset provenance

The examples use generated geometry, one locally generated audio file, and
third-party CC0 audio recordings. The village also bundles Kenney CC0 building,
furniture and nature models, credited below. The GIFs and forest image are
captures of this project's browser scenes.

- `assets/audio/bevy-raytraced-audio-chime.wav`: 2-second mono PCM chime at
  44.1 kHz and 16-bit. It was generated locally. SHA-256:
  `04d9e0bb1a906761e38f598084184830588221065a308fdc80ba3e91b11c0ec4`.
- `assets/gifs/examples-2d.gif`: Twelve sampled frames of the 16-emitter,
  32-segment 2D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.
- `assets/gifs/examples-3d.gif`: Twelve sampled frames of the 16-emitter,
  32-triangle 3D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.
- `website/assets/forest-demo.webp`: A 960 × 466 capture of the 1280 × 621 Bevy
  canvas from Chromium 154 with SwiftShader WebGL2. It shows the brush-attenuated
  direct path and one floor-reflection path.
- `website/assets/gifs/forest-walk.gif`: Thirty-six captured frames from
  Chromium 154 with SwiftShader WebGL2. The GIF duration is 4.51 seconds. The
  listener moves across the brush screen, changing the direct response from
  75%, 65%, and 52% to 100% in all bands while one floor-reflection path remains
  visible.
- `website/assets/forest-walk.mp4`: The same canvas sequence in H.264 format
  without an audio track. It is retained as a historical capture; the gallery now features the sandbox.

## Third-party audio recordings

`examples/raytraced-audio/assets/audio/` also holds real (non-synthesized)
recordings. They replace the synthetic chime in the demos. Each recording is
released under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
Attribution is not required, but we credit each author. Freesound sources come
from the public HQ preview of the sound page, which carries the same license.
`examples/raytraced-audio/assets/audio/CREDITS.md` holds the same information
next to the files.

Every file is mono, 44.1 kHz Ogg Vorbis. The village additions below use quality 6. One-shots use `-q:a 4`, have leading
silence trimmed and a faded tail, and peak at about -3 dBFS. Loops use
`-q:a 3`, are normalized to about -20 LUFS integrated, and have a -1 dBFS
limiter.

| File                      | Title                                                                                                                                        | Author                              | Source                                                        | License                                                       | Modifications                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `music_loop.ogg`          | GuitarDandCWithLONGERFade.wav                                                                                                                | kvgarlic (Kevin Boucher)            | <https://freesound.org/people/kvgarlic/sounds/209334/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | 51.33 s from 29.5 s, looped with a 1 s crossfade, mono, normalized                               |
| `footstep_wood_1..4.ogg`  | Hiking Boot Footsteps on Wooden Planks                                                                                                       | Fission9                            | <https://freesound.org/people/Fission9/sounds/521589/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | First four steps split into 0.35 s one-shots, trimmed, faded, mono, normalized                   |
| `footstep_grass_1..4.ogg` | Hiking Boot Footsteps on Grass                                                                                                               | Fission9                            | <https://freesound.org/people/Fission9/sounds/521587/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | First four steps split into 0.45 s one-shots, trimmed, faded, mono, normalized                   |
| `gunshot_1.ogg`           | gun lee enfield 303 rifle clean shot.wav                                                                                                     | kyles                               | <https://freesound.org/people/kyles/sounds/450853/>           | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to 1.4 s with a fade-out, mono, normalized                                               |
| `gunshot_2.ogg`           | Rifle gunshot, one shot                                                                                                                      | felix.blume                         | <https://freesound.org/people/felix.blume/sounds/710084/>     | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Leading silence removed, trimmed to 1.2 s with a fade-out, mono, normalized                      |
| `clap.ogg`                | Single Hand Clapping - Klatschen einer Person                                                                                                | florianreichelt                     | <https://freesound.org/people/florianreichelt/sounds/405624/> | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | One clap extracted, 0.6 s, faded, mono, normalized                                               |
| `knock.ogg`               | Toc toc (knocking on wooden door)                                                                                                            | aoimotion                           | <https://freesound.org/people/aoimotion/sounds/728289/>       | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to 0.75 s, faded, mono, normalized                                                       |
| `door_close.ogg`          | doorClose_1.ogg (RPG Audio pack)                                                                                                             | Kenney (www.kenney.nl)              | <https://kenney.nl/assets/rpg-audio>                          | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to 0.65 s, faded, mono, resampled, normalized                                            |
| `rain_loop.ogg`           | Soft Rain Loop                                                                                                                               | _lynks                              | <https://freesound.org/people/_lynks/sounds/595717/>          | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | 21.5 s loop, 1 s equal-power wrap crossfade, mono, −24 LUFS, Vorbis quality 6                    |
| `forest_ambience.ogg`     | 260623_055825_Taiwan Chiayi Alishan Ogasawara Lookout - June Early Morning - Forest Birds & Dawn Ambience_台灣嘉義阿里山小笠原觀景台6月_清晨 | BayTsai                             | <https://freesound.org/people/BayTsai/sounds/860231/>         | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | 37 s loop with a 2.5 s crossfade, mono, resampled, normalized                                    |
| `fire_crackle.ogg`        | fire-crackling.wav                                                                                                                           | jmehlferber                         | <https://freesound.org/people/jmehlferber/sounds/370938/>     | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | 22 s loop with a 2 s crossfade, mono, normalized (about -25 LUFS to keep the crackle transients) |
| `voice.ogg`               | Intermission Announcement                                                                                                                    | bevibeldesign (voice of Matt Downs) | <https://freesound.org/people/bevibeldesign/sounds/350442/>   | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Silence trimmed (17.9 s), faded, mono, normalized                                                |

The browser builder copies the example audio directory beside every route. It
loads these bundled files locally; it does not fetch third-party recordings at runtime.

The website uses the Fira Sans regular and medium web fonts hosted by the Bevy
website. The font files are licensed under the SIL Open Font License 1.1; the
license text is included at `website/assets/fonts/OFL.txt`. The catalogue
illustrations in `website/assets/` are original SVGs made for this project.
The forest WebP and GIF show the project's own procedural scene.

The example package enables Bevy's `vorbis` and `wav` decoders. The
`audio_fixture` integration tests decode the bundled recordings and retained WAV
fixture through Bevy's `Decodable` implementation, and measure the processed
music's high-frequency attenuation.

## Modular village models and new loops

The sandbox uses selected models from [Kenney's Building Kit](https://kenney.nl/assets/building-kit),
[Furniture Kit](https://kenney.nl/assets/furniture-kit), and
[Nature Kit](https://kenney.nl/assets/nature-kit), all **CC0 1.0**. Original license
files are retained under `examples/raytraced-audio/assets/models/`; its
`CREDITS.md` lists usage and the two window derivatives. Window glass was
separated from its frame so opening it removes the pane from the aperture while
retaining the wall. The door models retain their original animations; the demo
rotates the hinge and its acoustic/collision children together.

The stream uses **mycompasstv**'s [Babbling Brook](https://freesound.org/people/mycompasstv/sounds/474288/)
(CC0), converted from the author's HQ preview to a 28-second mono loop with a
two-second wrap crossfade, −24 LUFS normalization, and Vorbis quality 6. The two
NPCs use a continuous sequence of the already-credited Fission9 wooden steps.
`voice.ogg` supplies recorded spoken words in the speech cottage.

Rain was reprocessed with a one-second equal-power wrap crossfade and −24 LUFS
normalization (21.5-second final loop), Vorbis quality 6. Low default category
gains and a 0.55 master gain keep the new village substantially quieter than
the original sandbox. Rain starts off. The adjacent audio `CREDITS.md` records
all sources and processing details.

No third-party asset is fetched at runtime. The browser builder copies the
models, textures, licenses and recordings beside each example route.
