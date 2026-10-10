# Elevation and room acoustics

Research and implementation: 2026-10-10.

## Findings and options

The existing 3D tracer already launches a rotated Fibonacci sphere and intersects
XYZ triangles, including floors and ceilings. The missing link was playback:
left/right panning cannot distinguish equal-distance sources above and below the
listener. Elevation relies on direction-dependent outer-ear spectral filtering;
head movement supplies additional cues. [Resonance Audio concepts](https://resonance-audio.github.io/resonance-audio/discover/concepts.html)
explains these mechanisms and distinguishes direct sound, early reflections and
late reverberation. Competitive games also use HRTF headphone processing;
[VALORANT's audio options](https://playvalorant.com/en-gb/news/game-updates/valorant-patch-notes-8-0/)
include both built-in HRTF and external spatializers.

| Option                                     | Tradeoff for this project                                                                                                          |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Stereo level panning                       | Cheap and useful on speakers, but no elevation spectral cues. Retained as an A/B mode.                                             |
| Authored elevation EQ                      | Small and easy to exaggerate for gameplay, but uses invented coloration rather than measured ears.                                 |
| Measured HRTF convolution                  | Supplies pinna coloration, timing and head shadow together. Selected for point sources.                                            |
| Full spatial-audio middleware / Ambisonics | Broader capabilities, but adds a separate renderer and integration burden across native, browser and four Bevy versions. Deferred. |

[Steam Audio's binaural API](https://valvesoftware.github.io/steam-audio/doc/capi/binaural-effect.html)
provides a useful integration precedent: listener-relative 3D directions, stereo
output, interpolation, and explicit tail draining. Our implementation is local
Rust, not an integration with Steam Audio.

[Steam Audio materials](https://valvesoftware.github.io/steam-audio/doc/unreal/material.html)
separate frequency-dependent absorption and transmission from reflection
scattering. We use that distinction: carpet and upholstery absorb more high
frequencies, shelves scatter reflections, and furniture geometry intercepts
rays. A clear source is not arbitrarily low-pass filtered merely because the
room has carpet. Its reflected energy and tail change instead.

An open scene should not receive a generic loudness boost or an enclosed-room
tail. As a design inference from reflection geometry, outdoor impact character
comes from the recording, distance, nearby returns and more distant obstacles.
The renderer now retains an early return near an isolated outdoor surface even
when the traced diffuse tail is zero.

## Implementation

- `BinauralProcessor` interpolates the diffuse-field-equalized KEMAR responses
  measured by Bill Gardner and Keith Martin, MIT Media Laboratory. The
  [measurement archive](https://sound.media.mit.edu/resources/KEMAR.html) and
  [bundled attribution](../../src/hrtf/README.md) document provenance and use terms.
- Responses are resampled with a windowed-sinc filter for the decoded audio rate.
  FIR coefficients and distance gain are smoothed. Audio processing does not
  allocate or lock after construction. The embedded bank is about 185 KiB.
- `RaytracedAudioPlayer::with_binaural(distance_scale)` opts into stereo headphone
  output. Stereo inputs become mono point sources. Preparation disables Bevy's
  spatial sink so it cannot downmix and repan the output. Distance gain is applied
  once. Other players, including 2D sources, retain their original layout.
- The 3D adapter updates head-relative direction every frame, independent of ray
  cadence. The showcase listener follows body yaw and first-person head pitch.
  Missing or ambiguous listeners smoothly mute headphone voices.
- Traces retain low/mid/high surface-energy losses. Low/high decay times drive
  the comb feedback spectrum instead of fixed damping. Early returns have a
  separate send from late reverb; absorbing surfaces do not create return energy.
- Furniture uses six-face proxies with upholstery, wood, books and carpet
  presets. Floor treatment replaces the existing floor material rather than
  adding a second acoustic barrier. Treatment changes remove/restore furniture
  surfaces as well as changing their visibility.

## Repeatable listening

Use ordinary stereo headphones without a second spatializer. In the showcase:

1. Select **2** (above), then **3** (below). **B** compares HRTF with stereo panning.
   Move/turn, or use **F** for first-person head rotation.
2. Enter a room, close openings with **C**, then use **J** for a clap or **K** for
   the same gunshot recording. **T** cycles empty tile, empty carpet, furnished
   carpet. The HUD reports bass/treble decay and escaped-ray fraction.
3. Walk outside and repeat the impulse. Nearby house surfaces can return an
   echo, while an open space loses the sustained enclosed-room tail.
4. **Tab** independently bypasses room processing; **B** leaves room processing
   active while changing only the headphone cues.

## Limits and validation

KEMAR is a generic mannequin, not the user's own ears. Measurements cover
-40° to +90°; steeper downward angles use -40°, so nadir localization is an
approximation. 8–192 kHz decoded audio is supported. Strong floor muffling can
remove the high frequencies needed for elevation perception. These changes do
not simulate vibration through joists or frequency-dependent diffraction.

Reverb and early timing remain listener-centered statistical estimates, not
per-source, directionally resolved impulse responses. The early return is a
single averaged tap and the diffuse return is centered stereo. Material values
are illustrative presets, not measured assemblies. HRTF cost scales per voice;
large scenes should reserve it for important positional sounds.

Regression tests cover spherical second moments (a flat circle fails),
elevation waveform differences, interaural timing/level, listener rotation,
sample rates, missing listeners, downmixing/tail draining, softer furnished-room
responses, shorter treble tails, and isolated outdoor early reflections. These
objective checks establish signal behavior; they do not certify an individual
listener's localization accuracy.
