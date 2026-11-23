# FDM-03.0 — Topology-to-Geometry Binding Architecture and Mapping Contracts

## Status

Architecture contract for FDM-03. This step defines the data boundary and invalidation rules before implementing point-location or deformation mapping. It does not yet calculate or persist bindings.

## 1. Responsibility boundary

### Forma Deform owns

- Authored deformation topology (regular lattice or irregular cage).
- Control-point identities and control-state revisions.
- Binding records that map immutable source-geometry features into the cage domain.
- Mapping/evaluation mathematics and deterministic intermediate results.
- Binding validity checks against source and cage revision stamps.

### Host (AniCraft) owns

- Canonical authored Path2D and its source identity.
- Document/target identity and target generation.
- Immutable source snapshots and host geometry/topology revisions.
- Coordinate-space descriptors and source fingerprint generation.
- Scheduling, cache lifetime, persistence orchestration, applying/baking output, and undo/redo.
- Native Asset Warp and its pin model. FDM binding is not an alias or conversion of native Asset Warp.

Smart Controller evaluation and FDM pin deformation remain independent. This binding contract does not combine their outputs.

## 2. Binding target and immutable source

A binding is created against exactly one validated GeometrySnapshot whose payload kind/schema are the supported Path2D v1 contract.

The binding captures:
- HostTargetRef (including generation).
- Source GeometryRevision and source Path2D TopologyRevision.
- Source GeometryFingerprint (algorithm and digest).
- CoordinateSpaceDescriptor.
- Path2D payload kind and schema version.
- FDM DeformationTopologyId and DeformationTopologyRevision.
- Binding schema name/version and mapping-policy identifier.
- Outside-cage policy.

The binding does not own or mutate the source path. It references the immutable snapshot stamp and stores only the mapping data required to reproduce the association.

A source snapshot with any changed path content, coordinate interpretation, target generation, or source revision is not silently rebound. A source mismatch marks the binding stale and requires an explicit rebind operation.

## 3. Identity domains

The following identities are distinct and must not be substituted for one another:

- Host target identity: HostTargetRef.
- Source Path2D point identity: (command_index, point_role) within the exact fingerprinted snapshot.
- FDM topology identity: DeformationTopologyId.
- FDM control point identity: ControlPointId.
- FDM topology face reference: (face_index, subface_index) protected by the exact FDM topology ID/revision.

Path2D v1 has no persistent IDs for individual anchors or handles. Command index plus point role is therefore a snapshot-local locator only; it is invalid after any source snapshot fingerprint change. It must never be presented as a durable host point ID.

No generated mesh index, array offset, or sampled-polyline vertex is a persistent identity.

## 4. Source feature model

Binding operates on explicit Path2D command features, not on a destructive flattened replacement path.

Feature roles:
- MoveTo / LineTo: endpoint.
- QuadraticTo: control handle and endpoint.
- CubicTo: first handle, second handle, and endpoint.
- Close: structural command with no independent coordinate.

Each feature locator records command index and role. The subpath start/current-point relationship is derived from the original command stream and retained in the binding metadata.

The source Path2D remains authoritative. Any tessellation or adaptive samples used by a mapping algorithm are temporary derived data and must not replace or be persisted as the source geometry.

## 5. Cage-domain mapping contract

A binding maps each source feature point to a cage-domain location expressed by:
- FDM topology ID and topology revision.
- A face locator (face index plus subface index where triangulation is used).
- The stable control-point IDs of the containing face/subface.
- Barycentric weights for triangle mappings.
- An explicit mapping-policy/version identifier.

Weights must be finite, sum to one within the policy tolerance, and reference only control points in the stamped topology. Mapping data is immutable for a given binding revision.

### Triangle and quadrilateral policy

- Triangle faces use barycentric coordinates directly.
- Quadrilateral faces are not silently triangulated using storage order or an implementation-dependent diagonal.
- FDM-03.1 must select and document one deterministic quad mapping policy before writing binding records. Candidate policies are bilinear quad coordinates or a deterministic diagonal split. The chosen policy becomes part of the mapping-policy identifier and therefore part of binding identity.
- Mixed triangle/quad cages remain supported.

This defers a mathematically significant choice rather than baking an undocumented assumption into persisted bindings.

## 6. Outside-cage policy

Every binding declares exactly one policy:
- UnboundZeroInfluence (default): features outside the cage have no FDM deformation contribution.
- ClampToBoundary: map to the closest point on the cage boundary according to a documented distance/tie-break policy.
- Extrapolate: permit coordinates outside the cage domain using a documented bounded extrapolation policy.

The selected policy is explicit in the binding record. An implementation must not silently clamp or extrapolate. FDM-03.1 will define exact tie-breaking, tolerances, and extrapolation limits before enabling non-default policies.

## 7. Revision and freshness rules

Binding freshness requires exact agreement on:
- Host target identity and generation.
- Source geometry revision.
- Source topology revision.
- Source fingerprint and coordinate descriptor.
- FDM topology ID and topology revision.
- Mapping policy/version.

Control-state revision is deliberately excluded from binding freshness. Moving or pinning cage points changes evaluation output but does not invalidate the rest-space association. Evaluation/cache keys must include the current ControlStateRevision separately.

Consequences:
- Source geometry edit: binding stale.
- Source topology edit: binding stale.
- Coordinate-space change: binding stale.
- Host target deletion/recreation: binding stale.
- FDM topology edit (point/face/row/column insertion/removal): binding stale.
- Cage control-point movement or pin change: binding remains valid; evaluation output/cache is invalidated by control-state revision.

There is no implicit auto-rebind on stale data.

## 8. Coordinate-space contract

Cage points and Path2D source features must be expressed in the same validated CoordinateSpaceDescriptor and target-local rest coordinate space for v1.

No hidden stage transform, object transform, DPI conversion, axis flip, or unit conversion is inferred by FDM. If host and cage coordinates differ, the host adapter must provide an explicit, validated conversion before binding construction.

The coordinate descriptor is part of the source fingerprint and binding stamp.

## 9. Determinism and numerical requirements

- Binding construction is deterministic for identical snapshot, topology, outside policy, and mapping-policy version.
- Face traversal/tie-breaking must be deterministic and documented.
- No NaN or infinite coordinates/weights are accepted.
- Degenerate or invalid cage geometry is rejected before mapping.
- Tolerances are scale-aware and centralized per mapping policy.
- Any adaptive curve subdivision must preserve the original Path2D command stream and state its error bound, maximum depth, and resource budget.
- A budget exhaustion must return an explicit error or documented partial status; it must not silently claim a complete binding.

## 10. Versioning and persistence

Binding data requires its own versioned schema. It must not be added as unversioned fields to existing FDM topology records.

The binding schema stores:
- schema name/version;
- source snapshot stamp;
- topology identity/revision;
- mapping policy/version;
- outside policy;
- source feature locators and mapping coordinates;
- deterministic ordering and validation metadata.

Unknown fields and unsupported versions fail closed. No legacy data is assigned invented control-point IDs, face identities, or mappings.

Host owns persistence orchestration; FDM defines and validates the binding payload.

## 11. Implementation sequence

- FDM-03.0 — This architecture and mapping contract.
- FDM-03.1 — Binding data types, source feature enumeration, freshness stamps, policy enums, and validation. Resolve quadrilateral mapping policy.
- FDM-03.2 — Deterministic point location and source-feature-to-cage mapping for triangles and quads.
- FDM-03.3 — Outside-cage policies, boundary projection, bounded extrapolation, and deterministic tie-breaking.
- FDM-03.4 — Curve-feature sampling/adaptive mapping policy with explicit error and resource budgets.
- FDM-03.5 — Binding schema serialization, stale-binding diagnostics, and host adapter contract.

Each implementation step should remain independent of AniCraft's native Asset Warp internals.

## 12. Explicit non-goals

FDM-03 does not implement cage deformation, Bézier handle transformation, cage editing UI, Smart Controller composition, native Asset Warp conversion, host document persistence, or output baking. Those are separate contracts and implementation phases.
