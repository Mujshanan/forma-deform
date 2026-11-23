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

Its acceptance semantics differ:

- it currently requires one connected face component;
- it currently requires one closed boundary cycle;
- its relative embedding tolerance is `1e-9`;
- it has no separately named vertex-link validation pass, although shared-vertex intersections and boundary checks reject some malformed fans.

Therefore, simply deleting FDM-02.6 would silently lose accepted multi-component cases and some explicit validation guarantees. Simply importing FDM-02.6 would create a second authoritative geometry model and validator.

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

### LINEAGE-03.B — validator parity and policy decision

Before porting the generic point binding:

1. Add parity tests on the canonical validator for bow-tie vertex fans and shared-vertex collinear overlap.
2. Decide whether multiple disconnected cage components and multiple boundary loops are valid. Recommended default: permit disjoint components/loops when each component is independently manifold and non-overlapping, because the FDM-02.6 contract explicitly accepts separate components and artwork may contain disconnected islands.
3. Preserve convex-quad validation and the existing bounded pair-candidate work limit.
4. Keep the canonical FDM-03 tolerance policy unless a deliberate numerical-policy version change is approved. Do not silently replace `1e-9` with the FDM-02.6 `1e-12` threshold.
5. Return one certificate tied to topology identity/revision, control-state revision, and the exact geometry fingerprint.

The validator must remain one implementation. If the acceptance policy is broadened, implement it in `embedding.rs` and version/document the policy.

### LINEAGE-03.C — generic point-binding adapter port

1. Replace `CageEmbedding` inputs with `DeformationTopology`, `DeformationControlState`, and the canonical certified geometry.
2. Keep stable point IDs and deterministic shared-edge selection by sorted stable vertex-ID support.
3. Preserve the current FDM-02.7 triangle-barycentric behavior; do not silently introduce quad support or outside extrapolation in this port.
4. Reuse shared face-support evaluation rather than copying barycentric math into another engine.
5. Port persistent point-binding validation onto typed topology identity/revision and the canonical source snapshot contract.
6. Keep stale records fail-closed; no automatic rebind, geometry repair, or host document mutation.

### LINEAGE-03.D — integration verification

- Test generic point binding and Path2D feature binding against the same certified topology.
- Verify triangle and quad Path2D mappings, stable-ID shared-edge tie-breaking, disconnected components if accepted, and bow-tie rejection.
- Verify persisted point bindings reject source target/generation, source fingerprint, vertex-set, and topology changes.
- Verify current cage poses are validated before evaluation and that outputs are absolute and non-mutating.
- Run rustfmt, Clippy, workspace tests, benchmark smoke, and all three release WASM builds on the final integration head.

## 7. Explicit non-goals

- No AniCraft host/UI integration in this phase.
- No changes to CCC-01.5: Smart Controller and pin-deformation evaluation remain independent.
- No graph scheduler, cache, automatic rebind, host document transaction, apply, undo/redo, or bake implementation.
- No new deformation algorithms or authoring UI until the branch line is reconciled.

## 8. Current branch and PR status

- PR #47 remains a draft and is not merged.
- The integration branch is 82 commits ahead and 0 behind its declared FDM-02.1 base.
- PR #37 and PR #40 remain open drafts on their original lineage; neither has been merged into the canonical line.
- FDM-02.6/02.7 source has not been copied into the canonical line by this assessment.
