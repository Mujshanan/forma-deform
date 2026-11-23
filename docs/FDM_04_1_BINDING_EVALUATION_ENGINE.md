# FDM-04.1 — Binding Evaluation Engine

## Purpose

Evaluate an FDM-03 validated source-to-cage binding against a current, certified cage pose and produce a new immutable Path2D value.

This stage is the pure geometry-evaluation boundary between persistent binding data and later result integration. It does not own host scheduling, caching, document mutation, rendering, or apply/bake behavior.

## Inputs and trust requirements

The public entry point is `evaluate_binding`:

- `ValidatedBindingRecord`: a capability value produced only after FDM-03 persistence/binding validation against the exact rest-space source and cage.
- `GeometrySnapshot`: the immutable source Path2D snapshot originally bound.
- `DeformationTopology`: the stable cage topology.
- `ValidatedCageGeometry`: the current cage pose, constructed from a fresh global embedding certificate.

The evaluator rechecks source snapshot validity, source/target revisions and fingerprint, coordinate-space and payload stamps, and cage topology identity/revision. The current cage may have a newer control-state revision; it must not have a different topology identity or revision.

A raw or deserialized BindingRecord is not accepted. The evaluator does not manufacture or trust embedding certificates.

## Evaluation semantics

For each source feature in canonical Path2D command order:

| Binding status | Evaluated position |
|---|---|
| Mapped | Reconstruct from current cage face coordinates |
| Clamped | Reconstruct from the stamped boundary face coordinates |
| Extrapolated | Reconstruct from the stamped extrapolated face coordinates |
| Unbound | Preserve the authored source position (zero deformation influence) |

Triangle faces use the stored barycentric weights. Quad faces use the stored bilinear `(u, v)` coordinates in authored face order `P00, P10, P11, P01`. No triangulation or coordinate-space conversion is introduced.

The evaluator rebuilds the same command sequence and segment kinds. MoveTo/LineTo endpoints, quadratic controls/endpoints, and cubic controls/endpoints are replaced by their evaluated feature positions. Close commands remain unchanged. Path2D remains the authoritative authored representation; no flattening or destructive tessellation occurs.

## Determinism and non-accumulation

Every call evaluates from the original immutable source snapshot and the supplied current cage pose. It never takes a previous evaluated result as its input. Repeated calls with identical inputs therefore return identical geometry and fingerprints.

The result reports:
- Evaluated Path2D.
- Original source snapshot stamp.
- Current cage geometry fingerprint.
- Canonical evaluated Path2D fingerprint.
- Counts of mapped, clamped, extrapolated, and unbound features.

These are evaluation outputs, not persisted authored state. Hosts may use the source stamp and cage fingerprint as inputs to their own cache key, but cache ownership and invalidation remain outside this module.

## Failure behavior

Evaluation fails closed for:
- Invalid source snapshot or stale source/target/cage-topology dependencies.
- Incomplete, duplicate, or out-of-order source feature coverage.
- Missing face/control-point references, changed control-point order, or unsupported subfaces.
- Face/mapping-kind mismatch, invalid barycentric sums, non-finite coordinates/results.
- Invalid reconstructed Path2D or fingerprint failure.

The evaluator does not silently rebind, switch outside-cage policies, repair malformed records, or fall back to a different face.

## Boundaries

- No AniCraft source changes or host adapter implementation.
- No Smart Controller or bone-evaluation dependency or composition.
- No host scheduling, frame evaluation, cache, mutation, undo/redo, persistence of evaluated output, apply, or bake.
- No changes to native Asset Warp.
- Adaptive curve samples remain derived FDM-03 data. A later FDM-04 slice may expose their evaluation as transient preview/tessellation output without replacing Path2D.

## Initial verification matrix

- Triangle barycentric mapping follows a translated cage pose.
- Quad bilinear evaluation uses all four current control points.
- Repeated evaluation is identical and does not accumulate.
- Unbound features remain at their source positions.
- Source revision and cage topology changes are rejected.
- Adaptive curve-sample evaluation remains a follow-up FDM-04 slice before the engine is declared complete.
