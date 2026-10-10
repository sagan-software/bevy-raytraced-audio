# Core invariant scratchpad

## Implemented modes

- Propagation dimension is exactly `TwoDimensional` or `ThreeDimensional`; each adapter owns one dimension.
- Scene execution is CPU-only. The core has no renderer, audio-device, or GPU dependency.
- Direct/image-source queries return geometric response data. Listener traces separately estimate muffling, reverb, and outdoor ambience. The DSP processor consumes samples without owning an audio device.

## Material energy

- Absorption `A` is an incident-energy fraction in `[0, 1]`; transmission `T` is an amplitude fraction in `[0, 1]`.
- Each frequency band satisfies `A + T² <= 1`; the remaining reflected-energy fraction is `R = 1 - A - T²`.
- `AcousticMaterial::default()` has `A = 0` and `T = 0`; it blocks direct transmission and preserves all incident energy for reflection.
- `AcousticMaterial::new(absorption)` sets transmission to zero; `try_with_transmission` validates all three bands before returning a material.
- Sequential surface transmission multiplies per band. Direct-only scalar playback uses the mean of three gains; traced scalar playback uses the mean of low/high muffle gains. Processed playback applies the low/high filter to samples.
- `with_occluded_gain` applies to fully silent scalar paths; processed audio uses the traced filter.

## Boundary values

- Public coordinates must be finite `f32` values and use meters. Reject NaN and both infinities during construction.
- Band gains and absorption coefficients must be finite values in `[0, 1]`.
- Scene primitives must have finite coordinates and nonzero length or area. Reject degenerate geometry at insertion.
- Bevy transforms are converted to core coordinates only at adapter ingress; `AudioPlayer` assets and playback remain Bevy-owned, while the adapter updates sink volume or atomic DSP parameters.

## Derived values

- Path distance is computed from source and listener positions; callers do not set it independently.
- Direct transmission, occlusion, and reflection aggregates are computed from scene geometry and material values; callers do not set them independently.
- A direct path is marked occluded when any accumulated band gain differs from one.

## Failure categories

- Malformed input covers non-finite coordinates, out-of-range coefficients, and degenerate geometry.
- Combined material values that exceed the incident-energy budget are rejected by `try_with_transmission`.

## Reflection path queries

- A query yields only finite, visible, first-order specular reflections; coincident source and listener positions yield none.
- Paths follow surface insertion order. A surface index identifies one entry in the current scene and changes when that scene is rebuilt.
- Reflection point, mirrored image-source position, path distance, and per-band relative energy derive from scene geometry and material reflection energy. Solver output points retain double precision.
- Any other registered surface crossing either open reflection leg omits the candidate, regardless of its direct transmission value.
- Each registered segment or triangle hit contributes one material transmission factor to the direct path. A ray crossing a shared edge can intersect both registered 3D triangles and apply both factors when at least one accumulated band remains nonzero.

## Adapter path output

- Per-emitter path storage exists only when the caller attaches the matching 2D or 3D reflection-path component.
- `paths()` returns a borrowed slice in surface insertion order. The adapter reuses the component's vector between updates.
- The adapter replaces the slice contents on a valid update and clears it when the listener is ambiguous, either transform is invalid, or source and listener coincide.
- The aggregate response remains available without the optional path component. Path output does not modify audio samples.
- The iterator borrows the scene and keeps constant query state. It tests each candidate reflector against the BVH for both legs; the worst case remains quadratic when bounds overlap densely.
- The existing `AcousticResponse` remains copyable and continues to report aggregate reflected energy.

## Listener reverb

- Reverb send follows surviving echo energy, not geometric visibility alone. Fully absorbing surfaces cannot create a wet signal.

## Processed playback

- Invalid or disabled listener tracing clears stale muffling and reverb parameters.
- Configuration changes retrace immediately, independent of the periodic trace interval.
- The DSP sample processor uses preallocated storage and atomic parameters. Codec decoding and loop restart allocation are separate from that guarantee.
- Looping playback continues reading live parameters after the first loop.
- One-shot tails survive quiet pre-delay intervals, finish on a channel-frame boundary, and end within six seconds after the source.
- Debug animation speed changes only the visual replay. It does not stretch sound or change the physical speed of sound.
