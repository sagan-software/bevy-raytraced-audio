# Core invariant scratchpad

## Legal modes

- Propagation dimension is exactly `TwoDimensional` or `ThreeDimensional`; each adapter owns one dimension.
- Scene execution is CPU-only by default. A later GPU adapter may report `Ready` or a typed fallback reason and must preserve CPU output semantics.
- Each trace returns a direct path and bounded aggregate reflection data. It does not own source audio samples.

## Boundary values

- Public coordinates must be finite `f32` values and use meters. Reject NaN and both infinities during construction.
- Band gains and absorption coefficients must be finite values in `[0, 1]`.
- Scene primitives must have finite coordinates and nonzero length or area. Reject degenerate geometry at insertion.
- Bevy transforms are converted to core coordinates only at adapter ingress; audio remains in Bevy's normal asset and playback path unless a caller opts into processed playback.

## Derived values

- Path distance is computed from source and listener positions; callers do not set it independently.
- Surface transmission is computed from material absorption; callers do not set both values.
- Occlusion and reflection aggregates are computed from scene geometry and are never stored as mutable source-of-truth fields.
- GPU selection and fallback status are derived from runtime initialization and dispatch outcomes.

## Failure categories

- Malformed input covers non-finite coordinates, out-of-range coefficients, and degenerate geometry.
- Policy failures cover a requested acceleration mode unavailable in a selected adapter profile.
- Runtime fallback covers GPU adapter, device, shader, or dispatch failure and continues through the CPU path when fallback is enabled.
