# Acoustic village audit

The sandbox now uses the **3D acoustic adapter** with real Y-up coordinates.
A 3D rendering over the old 2D solver could not test upstairs/downstairs sound:
height was discarded. Both listener and emitters now retain their height, and
floors, ceilings, stairs, walls and moving leaves have acoustic geometry.

## Repeatable listening scenarios

Click the scene, start at a low system volume, then use these presets:

| Key | Position and isolated sound                       | Comparison                                                     |
| --- | ------------------------------------------------- | -------------------------------------------------------------- |
| 0   | Front garden; explore freely                      | Quiet mixed scene; N toggles rain, G your footsteps            |
| 1   | Speech cottage, outside its internal double doors | E opens both leaves; C closes all openings                     |
| 2   | Ground floor, upstairs runner only                | Hear timber-floor transmission and paths through the stairwell |
| 3   | Ground floor, basement runner only                | Compare a source below your feet with the upstairs source      |
| 4   | On the bridge, stream only                        | Deck blocks the direct path to the water                       |
| 5   | Beside the water, stream only                     | Direct path, less enclosure                                    |
| 6   | Outside a speech-cottage window                   | E opens/closes the pane while its frame remains solid          |
| 7   | Inside the basement                               | Same-floor footsteps; walk up the left staircase               |
| 8   | Upstairs                                          | Same-floor footsteps; walk down the right staircase            |

F enters first person. In third person the character faces the cursor, while
the overhead camera stays independent. WASD movement follows the camera;
Q/R orbits overhead or turns in first person. PgUp/PgDn supplies keyboard tilt
when an embedded browser cannot capture the pointer. H hides the guide.
Blue ear cup = left; coral = right. The bottom readout uses the character's ears,
not the overhead camera's screen axes.

E operates the nearest opening. O/C opens/closes all doors and windows for
comparisons; an opening pauses if its moving leaf would hit the player. Yellow
outlines mark two removable wall sections. X removes/restores the nearby module,
including its acoustic triangles and collider; step clear of its footprint before
rebuilding. Upper-story cutaways change
visibility only. They do not remove acoustic floors or ceilings.

Tab bypasses filtering/reverb while preserving the selected mix. M mutes all
categories; −/+ adjusts master gain. P freezes and silences the NPC runners.
The isolated presets exclude rain and the listener's own footsteps so those
sounds cannot obscure the comparison. V displays traced paths.

## Confirmed defects and fixes

- **Triangle/segment seams:** direct transmission multiplied both adjacent
  primitives when a path hit their shared boundary. A two-face wall with 0.5
  bass transmission could fall from 0.25 to 0.0625 at the seam. Coincident,
  coplanar crossings with identical transmission now count once; separate wall
  faces still multiply. Regression cases cover 2D joins and 3D diagonals.
- **Stereo downmix level:** rodio 0.22.2 discarded the result of its channel
  averaging expression, summing channels instead. The vendored correction and
  actual sample tests cover mono, stereo and six-channel inputs.
- **Reversed panning:** the earlier upstream rodio correction remains in the
  vendored backend. Samples are tested on both sides at multiple distances and
  listener headings. The overhead camera no longer turns with the character,
  which had hidden their rotation from view.
- **Redundant frame work:** image-source queries ran every render frame even
  when the listener tracer was throttled. Both computations now share the
  configured interval, while mixer and DSP controls continue updating every
  frame. Geometry changes, new emitters, and processing-mode changes still
  invalidate immediately. Direct-only mode still updates every frame.
- **Footstep voice churn and harsh levels:** NPCs now keep one persistent
  footstep track each, rather than spawning overlapping processed one-shots.
  Own footsteps are centered, quieter, and excluded from emitter tracing.
  Master starts at 0.55; footsteps and rain have much lower category gains;
  rain starts off. Gains fade over 120 ms when muted or isolated.
- **Loop preparation:** rain now uses a one-second equal-power wrap crossfade
  and −24 LUFS normalization. This reduces its decoded end-to-start sample jump
  from about 0.0585 to 0.0305; natural noise also has adjacent-sample jumps, so
  this measurement alone does not prove an audible click in the old recording.

## What this model does and does not measure

The kit's visual apertures were measured from the source meshes. Structural
modules generate lightweight acoustic planes and matching colliders. Furniture
now has six-face acoustic proxies. Tile, carpet, upholstery and shelves use
separate absorption/scattering presets. These are illustrative materials, not
measured transmission-loss curves for a particular home.

The renderer now offers measured KEMAR HRTF headphone elevation cues, material
band-dependent reverb decay, and an averaged early return that remains audible
near outdoor surfaces. See the [elevation/material update](research/vertical-audio-and-materials.md)
for controls, provenance and validation. It does not provide frequency-dependent
diffraction, per-source directional reflection taps, or structure-borne impact
transmission through joists. Upstairs footsteps remain airborne sources at the
NPC's feet. Generic HRTF elevation quality varies by listener, and directions
below -40° use the lowest measured response.

Earlier browser PCM probes checked actual scheduled output, clipping, and channel energy.
A warmed first-pass village produced 172 consecutive buffers over eight seconds
with no late scheduling and a peak of 0.3455 (below full scale). Scene/view
transitions did produce occasional main-thread scheduling delays of roughly
0.2 seconds in the embedded preview. Its inactive animation frames were also
throttled to about 2 FPS. Those are distinct from the core geometry defects;
this is not a claim of glitch-free playback on all devices. Final browser
checks and their scope are recorded in the testing guide.

All building, furniture and nature models are Kenney CC0 assets. Speech and
water are CC0 recordings. See [asset provenance](ASSETS.md).
