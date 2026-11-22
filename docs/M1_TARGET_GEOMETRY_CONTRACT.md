# M1 Proposal — Stable Target Identity and Revisioned Geometry Snapshots

**Status:** Proposed contract for review; not yet an accepted architecture decision  
**Scope:** Forma Deform ↔ AniCraft host boundary  
**Depends on:** ADM-001, ADM-002, ADM-007, CCC-01.5  
**Out of scope:** AniCraft implementation, persistence API design, evaluated-geometry injection, scheduling/cancellation, cache policy, animation sampling, bake transactions, multi-source composition

## 1. Purpose

Define the smallest host-neutral contract that lets Forma Deform identify a host-owned geometry target and evaluate against a known immutable version of that target's source geometry.

The contract must prevent four classes of error:

1. A target reference resolving to a different document or target after reload, duplication, deletion, or ID reuse.
2. FDM treating an array position, runtime pointer, or session-local handle as persistent identity.
3. FDM evaluating geometry in an assumed or ambiguous coordinate space.
4. A result calculated from an obsolete geometry snapshot being accepted after the target changes.

This is a design proposal. It does not authorize an AniCraft code change or imply that AniCraft currently exposes these APIs.

## 2. Ownership rules

### AniCraft owns

- Persistent document identity and host target IDs.
- Target existence, kind, and capabilities.
- Canonical authored source geometry and source topology.
- Per-target geometry/topology revisions and source fingerprints.
- Construction and lifetime of immutable geometry snapshots.
- Coordinate-space definitions and conversion metadata.
- Validation that a target reference and snapshot still identify the current host state.

### Forma Deform owns

- FDM-internal deformation-target identities, where needed.
- Deformation topology, control-point identities, mappings, and prepared geometry.
- Validation of whether a supplied snapshot is usable by a particular FDM algorithm.
- Deformation evaluation and typed result/dependency descriptions.

FDM must not allocate or persist AniCraft target IDs, mutate the supplied snapshot, or keep an authoritative copy of source artwork.

## 3. Identity model

Three identities must remain distinct.

### 3.1 Persistent host target reference

A durable binding refers to a host target using a document-scoped opaque identity:

| Field | Meaning |
|---|---|
| `host_namespace` | Stable identifier for the host implementation or host family |
| `document_id` | Host-issued persistent identity for the document |
| `target_kind` | Typed host target category |
| `target_id` | Opaque host-issued stable identifier within the document |

Illustrative Rust-like shape:

```rust
struct HostTargetRef {
    host_namespace: HostNamespace,
    document_id: HostDocumentId,
    target_kind: HostTargetKind,
    target_id: HostTargetId,
}
```

Requirements:

- The reference is independent of array order, tree position, display name, and runtime memory address.
- The host owns allocation, persistence, lookup, and deletion semantics for `document_id` and `target_id`.
- The host must guarantee that a reference cannot silently resolve to a different target after deletion and ID reuse. If IDs may be reused, the reference must include a host-issued generation/incarnation component.
- A document copy/duplicate must have a distinct document identity. The host must define whether duplicated target IDs are remapped and how authored FDM references are remapped with them.
- A save/reopen of the same document preserves its persistent document and target identities.
- A missing document, missing target, kind mismatch, or ambiguous resolution is an explicit failure, never a best-effort match by name or position.

The initial target-kind vocabulary should be small and host-defined. The adapter must not infer that a layer, symbol, group, bitmap, or shape is deformable merely because it has an ID. Target capability is queried separately.

### 3.2 FDM-owned identity

FDM may assign stable IDs to its own authored objects (deformation source, topology, control point, binding, composition node). These IDs are not host target IDs.

A persistent FDM object stores a `HostTargetRef` as its external target reference. It must never copy a host numeric ID into an FDM ID type or use a vector index as identity.

### 3.3 Runtime resolution identity

Resolving a persistent reference produces a runtime resolution record. This record may include:

- The exact `HostTargetRef` resolved.
- A session/document-instance token, if the host uses one.
- Current target kind and capability descriptor.
- Current geometry and topology revision tokens.
- A host-issued snapshot handle/token, if required by transport.

Runtime tokens are ephemeral. They are not serialized into authored FDM data and cannot replace `HostTargetRef`.

## 4. Revision model

Document-wide state IDs are too coarse to express target-specific geometry dependencies. The host contract should expose target-scoped revisions.

At minimum, geometry identity is the tuple:

```text
(document_id, target_kind, target_id,
 geometry_revision, topology_revision,
 geometry_fingerprint)
```

### 4.1 Geometry revision

A host-issued monotonic revision token scoped to one target identity. It changes whenever the canonical source geometry's coordinates or geometric attributes relevant to deformation change.

### 4.2 Topology revision

A host-issued monotonic revision token scoped to one target identity. It changes whenever the source geometry's structural organization changes, such as path segment structure, mesh vertices/connectivity, or other topology-defining data.

For a change that affects both coordinates and structure, both revisions change. A topology revision change is always a compatibility event for FDM mappings and topology-dependent authored data; FDM must not silently reinterpret old mappings.

### 4.3 Fingerprint

A deterministic content fingerprint over the canonical source-geometry payload and its topology-defining data. It is a verification token, not a replacement for revisions.

The fingerprint contract must identify its algorithm/version and canonicalization rules. It must not depend on memory layout, hash-map iteration order, pointer values, or nondeterministic serialization.

Revisions answer “has the host recorded a relevant change?” The fingerprint answers “does this snapshot's canonical content match the identified source content?” Both are required: a revision alone cannot detect a broken host implementation or mismatched payload; a fingerprint alone does not provide an efficient monotonic change sequence.

### 4.4 Separate dependency revisions

Do not overload geometry revision with unrelated state. Transform, hierarchy, animation sample, native Asset Warp state, and FDM-authored data are separate dependencies and will be specified in their own M1 contracts.

In particular, a transform-only change must not pretend that local source geometry changed. It may invalidate an evaluation later, but it is not a source-geometry revision.

## 5. Geometry snapshot

The host supplies a value-like immutable snapshot. It is a read-only view of host-owned source geometry, not a second authoritative document model.

Illustrative shape:

```rust
struct GeometrySnapshot {
    target: HostTargetRef,
    geometry_revision: GeometryRevision,
    topology_revision: TopologyRevision,
    fingerprint: GeometryFingerprint,
    coordinate_space: CoordinateSpaceDescriptor,
    geometry: GeometryPayload,
}
```

Required properties:

- Snapshot data is immutable for the lifetime of the evaluation.
- The snapshot is self-identifying: target reference, revisions, fingerprint, and coordinate space travel with the payload.
- The payload is complete for the declared geometry kind; it must not contain unresolved references to mutable host objects.
- Snapshot acquisition is atomic with respect to the revision tuple. The host must not return geometry from one state paired with revisions from another.
- FDM may retain a copy for the duration of evaluation, but must not mutate or write through to host data.
- A snapshot is a point-in-time input. It is not a live view and does not guarantee that the host target remains current after acquisition.

### 5.1 Initial coordinate-space rule

The initial contract should provide canonical source geometry in the target's declared local rest/source coordinate space.

The coordinate descriptor must explicitly state at least:

- Space kind and stable space identifier.
- Dimensionality (initially 2D for the current FDM scope).
- Linear unit and scale.
- Axis orientation/handedness and origin convention.
- Whether coordinates are local source/rest coordinates or another explicitly named stage in the host geometry pipeline.

The first implementation should not silently convert stage-space coordinates to target-local coordinates or vice versa. Conversion requires an explicit host-supplied transform contract, designed separately.

The initial snapshot must identify whether the payload is canonical authored source geometry or a named evaluated pipeline stage. FDM must not infer whether native Asset Warp, parent transforms, or any other upstream deformation has already been applied.

**AniCraft source finding:** for a shape without Asset Warp, `Shape.path` is the authored path. For a warped shape, AniCraft documents that `Shape.path` is the bent/shown outline while `Warp.rest` retains the outline as drawn. Therefore, the canonical pre-warp source snapshot must use `Warp.rest` when a live warp exists, and `Shape.path` otherwise. It must not snapshot the already-bent `Shape.path` and label it rest geometry. This also means native pin positions are not folded into the source-geometry revision; a change to the rest outline/rebase changes source geometry, while pin edits remain in the native Asset Warp state.

Existing pin edits continue through AniCraft's native warp commands; this proposal does not define composition between native Asset Warp and new FDM deformation sources.

### 5.2 Geometry payload

Use a versioned, typed payload envelope. The contract should permit multiple geometry kinds without pretending they are interchangeable.

Initial candidates:

- `Path2D`: ordered path segments and their geometric parameters.
- `Mesh2D`: vertices and explicit connectivity, only when the host exposes such source topology.
- `PointSet2D`: explicitly identified points where a host feature genuinely has point geometry.

Each payload declares its schema version and kind. Unsupported kinds return a capability/unsupported-geometry diagnostic. No lossy conversion, flattening, tessellation, or approximation is implicit in this contract; any such representation must be requested and identified explicitly by a later algorithm-specific contract.

## 6. Snapshot acquisition and validation lifecycle

1. FDM submits a persistent `HostTargetRef` to the host adapter.
2. AniCraft resolves the document and target, checks kind and requested capabilities, and rejects missing/ambiguous references.
3. AniCraft atomically captures canonical source geometry and its geometry/topology revisions and fingerprint.
4. AniCraft returns an immutable `GeometrySnapshot`.
5. FDM validates the target kind, payload schema, coordinate descriptor, revisions, and algorithm compatibility.
6. FDM evaluates without mutating the snapshot or authored data.
7. FDM includes the exact target reference and source revision tuple in its result/dependency description.
8. Before any result is applied, AniCraft compares the result's source revision tuple with the current target state. A mismatch rejects the result as stale; it is never silently rebased.

The final application protocol, atomicity, and scheduling are intentionally deferred to the later evaluation-result/freshness contract.

## 7. Failure and diagnostic behavior

The boundary must distinguish at least:

- Unknown host namespace or unsupported contract version.
- Document not found.
- Target not found.
- Target identity generation mismatch / stale identity.
- Target kind mismatch.
- Target kind exists but lacks requested deformation capability.
- Geometry snapshot unavailable.
- Unsupported geometry payload kind or schema version.
- Invalid or unknown coordinate-space descriptor.
- Geometry/topology revision mismatch.
- Fingerprint mismatch or invalid fingerprint algorithm/version.
- Malformed, non-finite, or structurally invalid geometry payload.

All failures are explicit and fail closed. No fallback to display names, array indices, approximate geometry, stale snapshots, or implicit coordinate conversion.

## 8. Compatibility and versioning

- Version the host integration contract independently from the FDM persistent-data schema and individual geometry payload schemas.
- Version each payload kind independently.
- Unknown required fields or unsupported major versions fail explicitly.
- Additive optional fields may be introduced compatibly only under a documented policy.
- Persistent host references must remain interpretable across application restarts.
- Runtime snapshot tokens and session handles are never persisted.
- Fingerprint algorithm identifiers must be explicit so future algorithm changes do not create false compatibility.

## 9. Proposed Rust boundary (illustrative, not API commitment)

The API should expose operations equivalent to:

```rust
trait GeometryHost {
    fn resolve_target(
        &self,
        target: &HostTargetRef,
        required_capabilities: &[TargetCapability],
    ) -> Result<ResolvedTarget, HostDiagnostic>;

    fn snapshot_source_geometry(
        &self,
        target: &HostTargetRef,
        request: GeometrySnapshotRequest,
    ) -> Result<GeometrySnapshot, HostDiagnostic>;

    fn validate_snapshot_current(
        &self,
        snapshot: &GeometrySnapshot,
    ) -> Result<SnapshotFreshness, HostDiagnostic>;
}
```

This is conceptual. It does not prescribe synchronous calls, Rust trait-object transport, serialization, plugin ABI, or a specific AniCraft implementation. A plugin boundary may need request/response DTOs instead.

## 10. Acceptance tests for the contract

The design is not implementation-complete until host and adapter tests cover:

1. A target reference resolves to the same target after save/reopen.
2. A reference from another document never resolves by coincidentally matching a target ID.
3. Duplicating a document cannot cause a reference to bind to the wrong copy.
4. Deleting a target and reusing an underlying numeric ID cannot resurrect an old reference.
5. Reordering siblings or changing display names does not change target identity.
6. Geometry coordinate edits advance geometry revision.
7. Structural geometry edits advance topology revision and invalidate topology-dependent FDM data.
8. A transform-only edit does not advance source geometry revision.
9. Snapshot payload and revision tuple are captured atomically.
10. A snapshot remains unchanged after subsequent host document edits.
11. Equivalent canonical geometry produces the same fingerprint under the declared algorithm/version.
12. A fingerprint mismatch is detected even if a revision token is accidentally unchanged.
13. Unknown payload kind/schema and unsupported coordinate spaces fail explicitly.
14. Stage/local coordinate ambiguity cannot pass validation.
15. A result referencing old geometry/topology revisions is rejected before application.

## 11. Open decisions before acceptance

The following require explicit resolution before calling this contract accepted or implementing it:

1. **Host document identity:** the inspected AniCraft `Document` / `DocumentFile` model has no persistent document UUID field. Its `next_id` is serialized, but that is an allocator, not document identity. The host therefore needs a persistent document UUID (or equivalent stable host-issued identity) and explicit copy/duplicate/Save As semantics before a durable cross-document target reference can be guaranteed.
2. **Target ID generation:** AniCraft's `ElementId`, `LayerId`, and `SymbolId` are document-local `u64` wrappers. `Document::alloc_id` allocates from serialized `next_id`, so the inspected design is monotonic under normal operation, but no generation/incarnation is part of the ID type. Confirm deletion, migration, import, and overflow behavior; retain a generation token in the external contract unless the host can formally guarantee non-reuse.
3. **Initial target kinds:** choose the first supported target kinds and exact capability queries; do not equate every ElementId with deformable geometry.
4. **Geometry payload:** confirm whether the first host snapshot is path-only or needs an additional explicit geometry kind.
5. **Revision ownership:** define where per-target geometry/topology counters live and how every mutation path advances them, including commands, undo/redo, imports, and document migrations.
6. **Fingerprint algorithm:** select a stable canonicalization and digest algorithm, including float normalization and versioning.
7. **Coordinate descriptor:** align units, axis orientation, origin, and local-rest semantics with AniCraft's actual geometry conventions.
8. **Native Asset Warp stage:** source inspection establishes that `Warp.rest` is the authored rest outline and `Shape.path` is the bent outline for a warped shape. Confirm the snapshot adapter uses `Warp.rest` when present and that existing pin edits remain on the native host path.
9. **Plugin transport:** choose how immutable snapshot payloads cross the current plugin boundary without leaking host implementation types.

## 12. Non-goals and preserved constraints

- Does not change AniCraft or its current document schema.
- Does not introduce FDM-owned source artwork copies.
- Does not define lattice/cage topology schemas or source mappings.
- Does not define transforms, hierarchy propagation, animation sampling, scheduling, cache invalidation, result application, or bake transactions.
- Does not replace native Asset Warp or alter existing pin tweening.
- Does not combine Smart Controller and Form Deform pin outputs; CCC-01.5 remains in force.
- Does not mandate CPU/GPU buffers, allocation, or a concrete ABI.
- Does not authorize a PR merge.

## 13. Source evidence and related architecture

The M0 audit records the inspected AniCraft implementation and ownership findings:

- [FDM v2 architecture baseline](FDM_V2_ARCHITECTURE.md)
- [FDM v2 host capability audit and M0 ownership matrix](FDM_V2_HOST_CAPABILITY_AUDIT.md)
- [AniCraft document evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)
- [AniCraft Asset Warp model](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs)
- [AniCraft Asset Warp commands](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs)
- [AniCraft Asset Warp ADR 0079](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0079-asset-warp.md)

The M0 audit and follow-up source inspection establish the following: AniCraft has document-scoped numeric `ElementId`, `LayerId`, and `SymbolId` wrappers; `Document::next_id` is serialized and `Document::alloc_id` allocates IDs within the document; the inspected `Document`/`DocumentFile` schema has no persistent document UUID; and a warped shape's `Shape.path` is the bent outline while `Warp.rest` preserves the authored rest outline. The inspected public model still does not expose a cross-cutting persistent document identity, target-ID generation token, or per-target geometry/topology revisions. These guarantees must be addressed before implementation.
