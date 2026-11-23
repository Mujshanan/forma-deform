# FDM-02.6 — Global Cage Embedding Validation

## Status

Implementation added on `feat/fdm-02-1-identity-topology-contracts`. Verification commands have not been run, following the standing instruction to skip tests and proceed. Treat this as implemented but unverified.

## Purpose

FDM-02.3 and FDM-02.2 validate local face/cell geometry and cage boundaries. Those checks do not, by themselves, prove that the interiors of all distinct faces form a non-overlapping planar embedding.

FDM-02.6 adds a fail-closed global validator and a revision-bound certificate. Binding construction must require this certificate before performing source-feature point location.

## Public API

Module: `form_deform_core::embedding`

- `validate_cage_embedding(&DeformationTopology, &DeformationControlState) -> Result<CageEmbeddingCertificate, EmbeddingError>`
- `CageEmbeddingCertificate::matches(...)` checks whether a certificate still describes the exact topology and control state.
- Certificate fields are private. Only the validator can create one.
- Read-only accessors expose topology ID/revision, control-state revision, geometry fingerprint, and face count.

The certificate is bound to:
- topology identity and topology revision;
- control-state revision;
- SHA-256 fingerprint of stable control-point IDs, canonicalized f64 positions, topology face kinds, and authored face vertex order.

Any position edit or control-state revision change invalidates the certificate. Pin-state-only edits also advance the control-state revision and conservatively invalidate it.

## Validation performed

1. Verify topology identity/revision matches the supplied control state.
2. Run the topology variant's structural validation and exact control-state membership validation.
3. Enforce resource limits and reject missing, duplicate, non-finite, or near-coincident control points.
4. Resolve all face vertices by stable control-point ID.
5. Reject degenerate faces, inconsistent winding, non-convex quads, and zero-length edges.
6. Validate shared-edge incidence, manifoldness, opposite directed traversal on shared edges, and connected face adjacency.
7. Require every topology control point to be referenced by a face.
8. Require exactly one directed boundary loop.
9. Broad-phase candidate face pairs using an X-axis AABB sweep.
10. For candidate pairs, reject edge intersections except exact shared edges or legal contact at declared shared vertices; reject face-interior containment/overlap.
11. Fail closed if the candidate-pair work budget is exceeded.

The pairwise narrow phase uses f64 arithmetic and a scale-aware tolerance derived from the cage bounds. No geometry is reordered, repaired, clamped, or triangulated.

## Complexity and safety limits

- Maximum points: 100,000.
- Maximum faces: 100,000.
- Maximum AABB candidate face pairs: 8,000,000.
- The sweep-and-prune broad phase is efficient for spatially localized meshes, but worst-case candidate generation can still be quadratic. The explicit budget prevents unbounded work and returns `CandidateLimitExceeded` instead of issuing a certificate.

The candidate budget is a safety bound, not a proof that every large valid cage can be certified. If real workloads exceed it, a later optimization can replace the broad phase without changing certificate semantics.

## Important semantics

- Shared-edge contact is valid only when both faces declare the exact same edge and traverse it in opposite directions.
- Shared-vertex contact is valid only at the shared stable ID and only when the two segments do not overlap along the same ray.
- Geometric contact without matching topology identity is not treated as adjacency.
- Overlapping non-adjacent face interiors are rejected; face order is never used to choose a winner.
- A certificate does not replace FDM-03 binding freshness stamps. It is a prerequisite for safe point location and is specific to the cage geometry used during that operation.

## Acceptance criteria

- [x] Public validation entry point accepts the typed topology enum and matching control state.
- [x] Certificate has private construction and records topology/control-state revisions plus a geometry fingerprint.
- [x] Stable IDs, not storage-vector assumptions, resolve face coordinates.
- [x] Local face, shared-edge, connectivity, point-use, and boundary-loop invariants are checked.
- [x] Global face-pair intersections and containment are checked with a bounded broad phase.
- [x] Shared edges and shared vertices have explicit legal-contact rules.
- [x] Tests are authored for a valid adjacent-triangle cage, invalid overlapping geometry, and certificate invalidation.
- [ ] Tests/build/format verification (not run by instruction).

## Non-goals

- No cage geometry repair or auto-rewinding.
- No point-location search or source binding.
- No change to topology identity/revision semantics.
- No change to AniCraft Path2D or native Asset Warp.
- No persistence format or host adapter.
- No merge or PR creation.
