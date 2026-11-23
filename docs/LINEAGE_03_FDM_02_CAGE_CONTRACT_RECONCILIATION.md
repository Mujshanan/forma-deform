# LINEAGE-03 — FDM-02 Cage Contract Reconciliation

**Status:** Architecture assessment; implementation of the FDM-02.6/02.7 port is not yet complete.  
**Canonical integration line:** `test/fdm-03-v1-integration-regressions`  
**Related drafts:** PR #37 (FDM-02.6) and PR #40 (FDM-02.7/02.8).

## 1. Decision boundary

FDM-01.3 and FDM-01.4 have been integrated into the FDM-02.1/FDM-03 line. The operator engine uses the existing canonical `GeometrySnapshot`, `EvaluationRequest`, `DeformerOperator`, and `EvaluatedGeometryResult` contracts. No duplicate geometry or result abstraction was introduced.

The remaining parallel line is FDM-02.6/02.7. It must be ported at the domain boundary, not merged as a second topology and binding stack.

## 2. Canonical owners on the integration line

| Concern | Canonical contract |
|---|---|
| Stable cage/control-point identity | `topology.rs`: `ControlPointId`, `DeformationTopologyId`, `DeformationTopologyRevision` |
| Triangle and quad topology | `DeformationTopology` / `TopologyView` |
| Mutable control-point state | `DeformationControlState` with its independent revision |
| Global planar embedding certificate | `embedding.rs`: `validate_cage_embedding` / `CageEmbeddingCertificate` |
| Certified geometry view | `binding.rs`: `ValidatedCageGeometry` |
| Path2D source-feature bindings | `BindingRecord` and `BindingStamp` |
| Path2D binding persistence boundary | `binding_persistence.rs`: `PersistedBinding` and `BindingHostAdapter` |
| Path2D binding evaluation | `binding_evaluation.rs` and `curve_sample_evaluation.rs` |

These types already connect topology, certification, source mapping, persistence validation, and derived curve-sample evaluation. They are the integration target.

## 3. Why the two embedding validators are not interchangeable yet

### FDM-02.6 — `CageEmbedding`

The PR #37 implementation uses an indexed, triangle-only `CageEmbedding` with raw `u64` vertex IDs and `Point2` positions. It has useful validation guarantees:

- explicit vertex-link checks for bow-tie/disconnected fans;
- shared-vertex collinear-overlap rejection;
- triangle winding and edge-incidence checks;
- crossing/touching and contained-face rejection;
- support for multiple disconnected face components.

Its numerical tolerance is based on a relative factor of `1e-12`. It does not provide the typed topology identity/control-state revision contract used by FDM-02.1.

### FDM-03 — `DeformationTopology` and `CageEmbeddingCertificate`

The canonical integration line already has a global validator over stable typed topology and control state. It supports triangles and convex quads, bounds point/face counts and pairwise candidate work, and uses a sweep/AABB broad phase.

The two implementations still differ in representation and numerical policy:

- FDM-02.6 accepts triangles only; FDM-03 supports triangles and strictly convex quads.
- FDM-02.6 uses an indexed cage with raw `u64` IDs; FDM-03 uses typed stable IDs, topology revisions, and separately revisioned control state.
- FDM-02.6 uses a `1e-12` relative tolerance; FDM-03 retains its `1e-9` relative tolerance and bounded AABB candidate budget.

The canonical FDM-03 validator has now been extended to preserve the important FDM-02.6 acceptance guarantees: explicit vertex-link validation, disconnected manifold components, and multiple independent boundary loops. The FDM-02.6 source model itself is not imported, so there remains one authoritative validator.

## 4. Binding models are related but serve different inputs

### FDM-02.7 — generic point-array attachment

`CageBinding` binds caller-supplied points to triangle supports. It stores stable point IDs, sorted stable cage vertex IDs, and normalized barycentric weights. It is useful for point arrays and other non-Path2D feature collections.

### FDM-03 — Path2D source-feature binding

`BindingRecord` binds enumerated Path2D anchors and Bézier control points. It supports triangle barycentric and quad bilinear coordinates, explicit outside-cage policies, source-feature locators, and validation that mappings reconstruct the authored source points.

These are not duplicate products. The generic point-array use case should remain available, but it should reuse the canonical topology, certificate, validated geometry, face-coordinate rules, and evaluation primitives. It should not retain a private triangle mesh or run its own global validator.

## 5. Persistence boundary mismatch

PR #40 adds `PersistedCageBinding` to the optional `cage_bindings` collection in `FdmDocument`. The FDM-03 Path2D path instead uses a separate `PersistedBinding` envelope and host adapter contract; the host chooses where the payload is stored and owns document transactions.

The first port should preserve the validated payload semantics while avoiding a second persistence owner. Recommended direction:

- keep authored deformer definitions and composition graphs in `FdmDocument`;
- keep each binding family as a validated FDM-owned payload with an explicit host adapter boundary;
- use the same host transaction/dirty-state/undo-redo responsibilities for both;
- do not silently add or migrate `cage_bindings` into the document schema as part of the code port.

A later document-schema decision can choose a unified container, but that must be explicit and versioned.

## 6. Required reconciliation work

### LINEAGE-03.A — operator engine integration

**Complete on the integration branch.** FDM-01.3/01.4 source, tests, documentation, and CI gates are integrated. The unified branch passes rustfmt, Clippy, workspace tests, benchmark smoke, and all three release WASM builds.

### LINEAGE-03.B — validator parity

**Implemented on the canonical integration branch.**

- Added generic vertex-link validation for triangle and quad faces; bow-tie/disconnected face fans fail closed.
- Disconnected, non-overlapping manifold components are accepted and counted in the certificate.
- Multiple independent boundary loops are accepted and counted; each loop must be closed, consistently directed, and contain at least three vertices.
- Added regression coverage for separate components, a connected quad ring with an inner boundary, bow-tie vertices, and collinear same-ray shared-vertex overlap.
- Preserved convex-quad validation and the bounded AABB pair-candidate budget.
- Kept the FDM-03 `1e-9` relative tolerance; the FDM-02.6 `1e-12` policy was not silently substituted.
- The certificate now reports connected-component and boundary-loop counts in addition to face count, and remains tied to topology identity/revision, control-state revision, and the exact geometry fingerprint.

The canonical validator remains a single implementation in `embedding.rs`. FDM-02.6's raw indexed cage and duplicate validator were not copied.

### LINEAGE-03.C — generic point-binding adapter port

**Implemented on the canonical integration branch.**

- Replaced the indexed `CageEmbedding` runtime input with `DeformationTopology` and `ValidatedCageGeometry`.
- Preserved triangle-barycentric point attachments and stable point IDs.
- Added an explicit point-location tie-break policy. Path2D binding retains lowest-face-index behavior; generic point bindings select the lexicographically lowest sorted stable vertex-ID support.
- Kept generic point bindings triangle-only and fail-closed outside the cage. No implicit quad triangulation, clamping, or extrapolation was introduced.
- Added versioned `PersistedCageBinding` JSON with typed topology identity/revision, stable cage vertex/triangle identities, source snapshot stamp, coordinate-space descriptor, and point supports.
- Persistence validation checks source target/generation, geometry revisions/fingerprint, topology identity/revision, cage vertex/triangle identity, exact point-ID coverage, and source-point reconstruction against a freshly certified rest cage.
- Added `CageBindingHostAdapter` with a separate point-binding payload slot. Writes use compare-and-swap on the expected binding revision and the source snapshot stamp, preventing stale or racing writes. The host owns atomic document transactions, dirty state, save/load, and undo/redo; the FDM core never mutates the host document.
- Generic point bindings use `GeometrySnapshotStamp` plus `CoordinateSpaceDescriptor`, not a Path2D-only snapshot, so they can bind host-supplied point arrays from other geometry payloads.
- Persisted triangle identities are canonicalized and validated; a payload cannot silently reorder its support topology.
- Enabled serde_json's `float_roundtrip` feature so persisted barycentric `f64` values survive JSON round-trips without changing their binary values.

The PR #40 `FdmDocument.cage_bindings` field was deliberately not copied. Both Path2D and generic point-binding families use separate FDM-owned payload envelopes and host adapter slots, leaving the host free to choose document storage without creating two competing persistence owners.

### LINEAGE-03.D — integration verification

- Runtime point-binding tests cover stable-ID shared-edge selection, identity at rest, absolute evaluation on a changed cage pose, outside-point rejection, duplicate point IDs, and explicit rejection of quad cages.
- Persistence tests cover exact JSON round-trip, current-source validation, source revision staleness, point reconstruction mismatch, invalid supports/weights, cage topology changes, and host adapter persist/load.
- FDM-03 embedding tests cover triangles/quads, disconnected manifold components, multiple boundary loops, bow-tie vertex rejection, and shared-vertex collinear overlap.
- Latest integrated CI run [#45159333217](https://github.com/Mujshanan/forma-deform/actions/runs/45159333217) passed rustfmt, Clippy, workspace tests, benchmark smoke, and all three release WASM builds.

## 7. Explicit non-goals

- No AniCraft host/UI integration in this phase.
- No changes to CCC-01.5: Smart Controller and pin-deformation evaluation remain independent.
- No graph scheduler, cache, automatic rebind, host document transaction, apply, undo/redo, or bake implementation.
- No new deformation algorithms or authoring UI until the branch line is reconciled.

## 8. Current branch and PR status

- PR #47 remains a draft and is not merged.
- The integration branch is 107 commits ahead and 0 behind its declared FDM-02.1 base.
- Current integrated head: `0cbbefc758122f08224b60055a80314577deca0e`.
- GitHub Actions run [#45160021180](https://github.com/Mujshanan/forma-deform/actions/runs/45160021180) passed rustfmt, Clippy, workspace tests, benchmark smoke, and all three release WASM builds.
- PR #47 remains a draft and is not merged.
- PR #37 and PR #40 remain open drafts on their original lineage; neither has been merged into the canonical line. Their semantic work is ported into the integration branch, but the original PRs have not been closed or modified.
