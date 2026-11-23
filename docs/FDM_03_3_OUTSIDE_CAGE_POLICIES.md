# FDM-03.3 — Outside-Cage Policies

## Status

The binding-stamp integration and mathematical outside-coordinate contract are implemented in the current branch. The work is committed but **not verified**: tests, compilation, formatting, and CI have not been run.

## Policy identity and compatibility

`binding::OutsideCagePolicy` is part of `BindingStamp` and has three explicit values:

- `UnboundZeroInfluence`
- `ClampToBoundary`
- `BoundedExtrapolation { max_distance }`

`OUTSIDE_CAGE_POLICY_VERSION` is stamped independently. The bounded distance must be finite and strictly positive. It is expressed in the source snapshot's coordinate-space linear units; the complete `CoordinateSpaceDescriptor` is already part of the stamp, so a unit/scale/origin change makes the binding stale.

New stamps use binding schema v2. The validator accepts legacy schema v1 only with `UnboundZeroInfluence`; a legacy stamp cannot opt into clamping or extrapolation. Policy version mismatches and invalid parameters fail closed. Policy parameters participate in the stamp's value identity and therefore in any cache key derived from the stamp.

FDM-03.2 remains the first-stage classifier. Only an explicit `OutsideCage` result may be passed to outside-policy resolution. Locator errors are not treated as outside points.

## Binding record semantics

`SourceFeatureBinding` now distinguishes:

- `Mapped(MappingRecord)`: ordinary inside-cage mapping.
- `Clamped(MappingRecord)`: boundary-projected mapping; accepted only with `ClampToBoundary`.
- `Extrapolated(MappingRecord)`: out-of-domain face coordinates; accepted only with `BoundedExtrapolation`.
- `Unbound`: accepted only with `UnboundZeroInfluence`.

The record validator enforces policy/status compatibility. Clamped entries recompute the deterministic nearest boundary projection and require the record's face and coordinates to match it exactly; reconstruction is checked against the projected point, not the original outside point. The original Path2D feature remains unchanged and is re-enumerated from its stamped source snapshot.

Extrapolated entries recompute the nearest boundary and its distance, enforce the stamped maximum, require the same owning face, require genuinely out-of-domain coordinates, and reconstruct the original source feature point. Inside-only `Mapped` records continue to reject out-of-domain coordinates.

## Boundary projection

Boundary segments are derived from face-edge incidence: an undirected stable-control-ID edge is on the boundary only when exactly one face references it. Projection considers every such edge. The nearest segment wins; distance ties within the relative tie tolerance use face index and then authored edge index.

Triangle coordinates on a boundary edge are interpolated barycentric vertex weights. Quad coordinates are interpolated in authored `P00, P10, P11, P01` bilinear parameter space. No diagonal split is introduced.

## Mathematical extrapolation

For a triangle with authored vertices `A,B,C`, solve the affine system

`P = A + u(B-A) + v(C-A)`

and store barycentric weights `(1-u-v, u, v)`. The weights sum to one but may be negative or exceed one outside the triangle. The inverse is rejected when the determinant is non-finite or numerically singular relative to the face scale.

For a quad in authored order `P00,P10,P11,P01`, use the full bilinear patch

`P(u,v) = a + b u + c v + d u v`

with `a=P00`, `b=P10-P00`, `c=P01-P00`, and `d=P00-P10+P11-P01`. Eliminate `v` using

`cross(q-bu, c+du)=0`, where `q=P-P00`.

Solve the resulting quadratic in `u` with a cancellation-resistant root formula, recover `v` by projection onto `c+du`, and reconstruct the point. Exactly one finite, sufficiently conditioned solution must reconstruct within the scale-relative spatial tolerance. Singular, non-finite, non-reconstructing, or multiple-solution inverses fail closed. Coordinates may lie outside `[0,1]^2`; no triangulation or nearest-edge extrapolation approximation is used.

The bounded distance is Euclidean source-space distance from the source point to the nearest cage-boundary segment. It limits the amount of extrapolation independently of the parameter magnitudes. A valid coordinate inverse is not accepted if the boundary distance exceeds the stamped limit.

## Validation and remaining verification

Tests were added for policy-stamp parameter validation, triangle barycentric extrapolation, quad bilinear extrapolation, and distance-limit rejection. Existing inside-only mapping checks remain strict.

Before marking FDM-03.3 complete, run the Rust test suite, formatting, compilation, and required CI; address any failures and inspect the final branch head. No test/build/format/CI run has been performed as part of this implementation turn.

No policy auto-rebinds stale source features or modifies Path2D commands. Smart Controller evaluation and native Asset Warp remain independent.
