# FDM-03.3 — Outside-Cage Policies

## Status

Policy-resolution groundwork is implemented. This is **not a complete FDM-03.3 binding implementation**: policy identity is not yet carried through `BindingStamp`, and bounded extrapolation is represented as an explicit boundary point plus source-space offset rather than as an extrapolated face-coordinate record. Do not mark this step complete until the integration contract below is implemented and verified.

No tests, formatting, compilation, or CI checks have been run.

## Compatibility

FDM-03.2 remains unchanged. Callers first run `point_location::locate_point`; only its `OutsideCage` outcome is eligible for this resolver. Located points continue to use their existing inside-face coordinates.

The existing `binding::OutsideCagePolicy` and FDM-03.1 stamp validation remain fail-closed: only `UnboundZeroInfluence` is accepted by a binding stamp. This module does not bypass that check or make older records silently adopt a new policy.

## New API

Module: `outside_policy`

- `OutsideCageResolutionPolicy::UnboundZeroInfluence`
- `OutsideCageResolutionPolicy::ClampToBoundary`
- `OutsideCageResolutionPolicy::BoundedExtrapolation { max_distance }`
- `resolve_outside_point(point, geometry, policy)`

The geometry must be a `ValidatedCageGeometry` built from a current FDM-02.6 embedding certificate.

### Boundary projection

Boundary segments are derived from face-edge incidence: an edge is on the cage boundary only when exactly one face references its undirected stable-control-ID pair. Projection considers all such edges.

For each segment, the resolver computes the clamped Euclidean segment projection and its distance. The winner is the smallest distance; near-equal distances use deterministic face-index then authored-edge-index ordering. It returns the owning face, edge index, ordered endpoint IDs, face coordinates at the projected point, projected source-space point, and distance.

Triangle edge coordinates are interpolated barycentric vertex weights. Quad edge coordinates are interpolated in the authored `P00, P10, P11, P01` parameterization. No quad diagonal is introduced.

### Bounded extrapolation result

The bounded policy requires a finite positive `max_distance`. If the nearest-boundary distance exceeds it, resolution fails with `DistanceLimitExceeded`. Otherwise the result retains both the boundary projection and the exact source-space offset from that projection.

**Important:** the offset is not yet converted into out-of-domain triangle/quad coordinates and is not accepted by FDM-03.1's inside-only `MappingRecord` validator. It is an explicit geometric result, not yet a persistent binding representation. This avoids inventing a face-coordinate extrapolation rule or silently weakening v1 validation.

## Required follow-up before completion

1. Define a versioned, serializable outside-policy identity that includes the bounded-extrapolation distance and units/coordinate-space interpretation.
2. Carry that identity through `BindingStamp`, freshness checks, schema validation, and policy-specific record validation.
3. Define a mathematically consistent extrapolated mapping for both triangle and bilinear quad faces, including conditioning limits and reconstruction checks. Reject ambiguous or ill-conditioned solutions.
4. Ensure clamped records validate against the projected point while preserving the original source feature point and policy result for auditability.
5. Add tests for boundary edge extraction, nearest-edge selection, deterministic ties, triangle and quad edge coordinates, zero/invalid limits, exceeded limits, large finite values, and mixed-face cages.
6. Test compatibility: v1 `UnboundZeroInfluence` records remain unchanged; unsupported or stale policy identities fail closed.

No policy may auto-rebind stale source features or modify Path2D commands. Smart Controller evaluation and native Asset Warp remain outside this subsystem.
