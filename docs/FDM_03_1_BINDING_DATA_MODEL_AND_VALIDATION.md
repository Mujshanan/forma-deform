# FDM-03.1 — Binding Data Model and Validation

## Implementation status

Implementation files and unit tests have been authored. **Tests, formatting, and compilation have not been run** in accordance with the instruction to skip verification. This step is not declared verified or complete until those checks are requested and pass.

## Added API

- `binding::enumerate_source_features` emits Path2D v1 coordinate-bearing features in command/role order. `Close` remains structural and creates no feature. Locators are snapshot-local.
- `MappingPolicy::TriangleBarycentricQuadBilinearInsideV1` names the versioned mixed-face policy. Triangles use barycentric weights; quads use bilinear `(u, v)` coordinates in authored vertex order. There is no diagonal split.
- `BindingStamp` captures target identity/generation, source revisions and fingerprint, coordinate-space descriptor, payload schema, FDM topology identity/revision, schema and numerical-policy versions, mapping policy, and outside policy.
- `ValidatedCageGeometry` is an immutable ID-to-position view. Its constructor requires a current `CageEmbeddingCertificate`; callers cannot set a validation boolean or construct the certificate themselves.
- `BindingRecord` stores one ordered mapped/unbound status per source feature. `validate_against` checks the exact source snapshot, topology, certified geometry, face kind/order, coordinate domain, and reconstructed source position.

## Validation behavior

- FDM-03.1 enables only `UnboundZeroInfluence`. Clamp and extrapolation return an unsupported-policy error.
- Control-state revision is excluded from binding freshness. It remains part of embedding-certificate validity and must be included separately in evaluation/cache keys.
- Triangle weights must be finite, within [0, 1], and sum to one within the dimensionless policy tolerance. Values are not normalized or clamped.
- Quad coordinates must be finite and within [0, 1]. Reconstruction uses all four authored vertices.
- Face references are topology-revision-bound indices accompanied by stable control-point IDs in exact authored order. The v1 subface locator is explicitly zero; no hidden triangulation is permitted.
- Incomplete, duplicated, reordered, stale, or geometrically inconsistent source-feature records fail closed.

## Scope boundary

This step does not implement point-location search, inverse-bilinear solving, boundary projection, extrapolation, curve sampling, persistence, deformation evaluation, host scheduling, or AniCraft UI integration. Those remain in FDM-03.2 through FDM-03.5 or later host integration work.

## Files

- `crates/form-deform-core/src/binding.rs`
- `crates/form-deform-core/src/lib.rs`

Unit tests cover deterministic feature enumeration, all-four-vertex bilinear reconstruction, certificate gating, unsupported outside policies, freshness, complete/incomplete feature coverage, non-finite/out-of-domain triangle coordinates, and stable face-ID ordering. Additional edge-case and integration coverage remains appropriate before declaring this step complete.
