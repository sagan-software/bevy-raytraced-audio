# Audio credits

Every recording in this directory except the chime is a third-party field or
foley recording. None of them is synthesized. All of them are released under
[CC0 1.0 Universal (public domain dedication)](https://creativecommons.org/publicdomain/zero/1.0/),
so attribution is not legally required. We credit the authors anyway.

Freesound sources were taken from the public high-quality preview of each sound
(`https://cdn.freesound.org/previews/...-hq.ogg`). The preview has the same
license as the sound page.

All files were changed in the same ways. Each one was downmixed to mono,
resampled to 44.1 kHz, and encoded as Ogg Vorbis. One-shots use quality 4 and
loops use quality 3. One-shots have their leading silence trimmed so the
transient starts within about 5 ms. They have a fade-out on the tail and are
peak-normalized to about -3 dBFS. Loops are gain-normalized to about -20 LUFS
integrated, with a -1 dBFS peak limiter.

| File                                            | Title                                                                                                                                        | Author                              | Source                                                        | License                                                       | Modifications                                                                                                                                          |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bevy-raytraced-audio-chime.wav`                | Chime                                                                                                                                        | This project                        | Generated locally                                             | Project license                                               | None                                                                                                                                                   |
| `music_loop.ogg`                                | GuitarDandCWithLONGERFade.wav                                                                                                                | kvgarlic (Kevin Boucher)            | <https://freesound.org/people/kvgarlic/sounds/209334/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Took 51.33 s starting at 29.5 s and made it loop with a 1 s equal-power crossfade. Mono, normalized.                                                   |
| `footstep_wood_1.ogg` … `footstep_wood_4.ogg`   | Hiking Boot Footsteps on Wooden Planks                                                                                                       | Fission9                            | <https://freesound.org/people/Fission9/sounds/521589/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Split the first four steps into 0.35 s one-shots. Trimmed, faded, mono, normalized.                                                                    |
| `footstep_grass_1.ogg` … `footstep_grass_4.ogg` | Hiking Boot Footsteps on Grass                                                                                                               | Fission9                            | <https://freesound.org/people/Fission9/sounds/521587/>        | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Split the first four steps into 0.45 s one-shots. Trimmed, faded, mono, normalized.                                                                    |
| `gunshot_1.ogg`                                 | gun lee enfield 303 rifle clean shot.wav                                                                                                     | kyles                               | <https://freesound.org/people/kyles/sounds/450853/>           | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to 1.4 s with a 0.5 s fade-out. Mono, normalized.                                                                                              |
| `gunshot_2.ogg`                                 | Rifle gunshot, one shot                                                                                                                      | felix.blume                         | <https://freesound.org/people/felix.blume/sounds/710084/>     | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Removed 0.83 s of leading silence. Trimmed to 1.2 s with a 0.4 s fade-out. Mono (mid channel of the M/S recording), normalized.                        |
| `clap.ogg`                                      | Single Hand Clapping - Klatschen einer Person                                                                                                | florianreichelt                     | <https://freesound.org/people/florianreichelt/sounds/405624/> | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Extracted one clap at about 10.3 s. Trimmed to 0.6 s, faded, mono, normalized.                                                                         |
| `knock.ogg`                                     | Toc toc (knocking on wooden door)                                                                                                            | aoimotion                           | <https://freesound.org/people/aoimotion/sounds/728289/>       | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to a 0.75 s double knock. Faded, mono, normalized (+19.7 dB).                                                                                  |
| `door_close.ogg`                                | doorClose_1.ogg, from the RPG Audio pack                                                                                                     | Kenney (www.kenney.nl)              | <https://kenney.nl/assets/rpg-audio>                          | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Trimmed to 0.65 s, faded, mono, resampled from 48 kHz, normalized.                                                                                     |
| `rain_loop.ogg`                                 | Soft Rain Loop                                                                                                                               | _lynks                              | <https://freesound.org/people/_lynks/sounds/595717/>          | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | 21.5 s loop, 1 s equal-power wrap crossfade. Mono, −24 LUFS / −6 dBTP, Vorbis quality 6.                                                               |
| `forest_ambience.ogg`                           | 260623_055825_Taiwan Chiayi Alishan Ogasawara Lookout - June Early Morning - Forest Birds & Dawn Ambience_台灣嘉義阿里山小笠原觀景台6月_清晨 | BayTsai                             | <https://freesound.org/people/BayTsai/sounds/860231/>         | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | First 39.5 s made into a 37 s loop with a 2.5 s equal-power crossfade. Mono, resampled from 48 kHz, normalized.                                        |
| `fire_crackle.ogg`                              | fire-crackling.wav                                                                                                                           | jmehlferber                         | <https://freesound.org/people/jmehlferber/sounds/370938/>     | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | First 24 s made into a 22 s loop with a 2 s equal-power crossfade. Mono, normalized. The crackle peaks are left intact, so it measures about -25 LUFS. |
| `voice.ogg`                                     | Intermission Announcement                                                                                                                    | bevibeldesign (voice of Matt Downs) | <https://freesound.org/people/bevibeldesign/sounds/350442/>   | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) | Leading and trailing silence trimmed, giving 17.9 s. Faded, mono, normalized to -20 LUFS.                                                              |

## Acoustic village additions (2026-10-10)

- `stream_loop.ogg`: [Babbling Brook](https://freesound.org/people/mycompasstv/sounds/474288/)
  by **mycompasstv**, CC0. Source: the author's 256 kbps high-quality MP3 preview,
  `https://cdn.freesound.org/previews/474/474288_3902754-hq.mp3`.
  Converted to mono 44.1 kHz; first 30 seconds reordered at the two-second mark
  with a two-second linear wrap crossfade (28-second final loop); normalized to
  −24 LUFS with a −6 dB true-peak ceiling; encoded as Vorbis quality 6.
- `footsteps_walk_loop.ogg`: derived from the four existing **Fission9** CC0
  wooden-footstep recordings credited above. Each is padded to half a second,
  then concatenated into a two-second mono 44.1 kHz loop, Vorbis quality 6.
  This lets each NPC keep one audio voice and one reverb processor while walking.
- The speech cottage reuses `voice.ogg`, **bevibeldesign / Matt Downs**'s CC0
  intermission announcement credited above. These are intelligible recorded
  words, not generated speech.

The sandbox uses conservative playback gains in addition to file normalization:
master 0.55, own footsteps 0.08, NPC footsteps 0.13, rain 0.055 (initially off),
stream 0.4, speech 0.7. Isolated presets mute unrelated categories, and master
mute remains effective with acoustic processing bypassed.

Rain follow-up: `rain_loop.ogg` now uses a one-second equal-power wrap crossfade
(21.5 seconds total), normalized to −24 LUFS / −6 dBTP and encoded at Vorbis
quality 6. The source and CC0 license are unchanged. This replaces the earlier
five-millisecond edge fades.
