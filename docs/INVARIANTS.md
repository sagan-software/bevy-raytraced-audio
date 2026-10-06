# Core invariant scratchpad

## Implemented modes

- Propagation dimension is exactly `TwoDimensional` or `ThreeDimensional`; each adapter owns one dimension.
- Scene execution is CPU-only. The core has no renderer, audio-device, or GPU dependency.
- Each trace returns a direct path and aggregate first-order reflection energy. It does not own source audio samples.

## Material energy

- Absorption `A` is an incident-energy fraction in `[0, 1]`; transmission `T` is an amplitude fraction in `[0, 1]`.
- Each frequency band satisfies `A + T² <= 1`; the remaining reflected-energy fraction is `R = 1 - A - T²`.
- `AcousticMaterial::default()` has `A = 0` and `T = 0`; it blocks direct transmission and preserves all incident energy for reflection.
- `AcousticMaterial::new(absorption)` sets transmission to zero; `try_with_transmission` validates all three bands before returning a material.
- Sequential surface transmission multiplies per band. The adapter reduces the three accumulated amplitude gains to their arithmetic mean for Bevy's scalar sink volume.
- `with_occluded_gain` applies only when the accumulated direct gain is zero in all three bands.

## Boundary values

- Public coordinates must be finite `f32` values and use meters. Reject NaN and both infinities during construction.
- Band gains and absorption coefficients must be finite values in `[0, 1]`.
- Scene primitives must have finite coordinates and nonzero length or area. Reject degenerate geometry at insertion.
- Bevy transforms are converted to core coordinates only at adapter ingress; `AudioPlayer` assets and playback remain Bevy-owned, while the adapter adjusts sink volume.

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
