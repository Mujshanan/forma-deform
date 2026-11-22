# FD-M7 — Multi-axis five-pose controller field

**Status:** first host-independent implementation slice. No AniCraft host API or persistence change is included.

## Scope

M7 begins by enabling the existing `JoystickFivePose` mapping variant in the canonical Smart Controller mapper. The controller uses two ordered scalar inputs:

- input 0: X, normalized/clamped to [-1, 1]
- input 1: Y, normalized/clamped to [-1, 1]

The five referenced runtime poses are ordered by the mapping fields, not by map iteration:

| Anchor | Coordinate |
|---|---:|
| center | (0, 0) |
| up | (0, 1) |
| right | (1, 0) |
| down | (0, -1) |
| left | (-1, 0) |

Each pose must contain exactly the controller's declared binding IDs. All five poses must have matching binding IDs, typed value kinds, and angle wrapping semantics.

## Interpolation semantics

The implementation uses inverse squared Euclidean distance weighting (Shepard interpolation) over the five authored anchors.

For a normalized input point p and anchor a_i:

- d_i² = ||p - a_i||²
- w_i = (1 / d_i²) / sum_j(1 / d_j²)

An exact anchor hit returns that authored pose directly, avoiding division by zero and preserving exact authored values. Otherwise, each output binding is blended using the normalized weights.

- Scalars and Vector2 values use weighted arithmetic interpolation.
- Unwrapped angles use weighted arithmetic interpolation in radians.
- Wrapped angles use a weighted circular mean. A near-zero resultant vector is rejected as ambiguous rather than choosing an arbitrary angle.
- Non-finite input coordinates are rejected.
- Coordinates outside the authored joystick square are clamped per axis.

This is a deterministic five-anchor field, not a full authored 2D pose grid. It does not create diagonal corner poses or infer extra authoring points.

## Boundaries

- Pose data remains supplied by the runtime `PoseLibrary`; M7 does not claim durable pose persistence.
- The mapper remains pure and host-independent.
- AniCraft continues to own bone samples, coordinate transforms, scheduling, timeline evaluation, document storage, and target mutation.
- This first slice does not add additive multi-controller composition or implicit source blending.
- A future grid-based pose field should use an explicit schema and triangulation/interpolation contract rather than silently changing this five-pose mapping.

## Tests and acceptance

- Exact center and cardinal anchor preservation.
- Per-axis clamping.
- Deterministic finite diagonal interpolation.
- Vector2 output blending.
- Wrapped-angle circular interpolation.
- Existing Rust workspace and WASM build suites.
