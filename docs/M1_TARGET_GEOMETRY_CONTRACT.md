# M1 Proposal — Stable Target Identity and Revisioned Geometry Snapshots

**Status:** FDM-side M1 contract and executable primitives implemented; AniCraft host conformance and architecture acceptance remain pending

## Implementation progress — M1 FDM-side contract

The FDM-side M1 contract is implemented in `form-deform-core`:

- Opaque host namespace, document, target-kind, target, and coordinate-space identifiers.
- Mandatory non-zero target incarnation generation.
- Independent non-zero geometry and topology revision epochs.
- Validated coordinate-space descriptors with explicit units, scale, axes, origin, and rest-space kind.
- An owned immutable Path2D v1 payload with private command storage and validated command sequencing.
- Deterministic SHA-256 fingerprint generation using the versioned `fdm-path2d-sha256-v1` canonical byte encoding.
- Snapshot construction that rejects unsupported payload kind/schema and recomputes the fingerprint to verify that it matches the payload and coordinate descriptor.
- Snapshot stamps and freshness comparison that distinguish target identity mismatch, ordinary staleness, revision regression, and equal-revision fingerprint integrity failures.

The implementation does not create host IDs, read AniCraft documents, persist bindings, acquire host snapshots, update host revisions, or apply deformation results. Those remain AniCraft-side conformance work and are not included in this M1 FDM-side change. Native Asset Warp and pin tweening remain untouched.

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

This is the FDM-side M1 contract proposal and reference implementation. It does not authorize an AniCraft code change or imply that AniCraft currently exposes these APIs.

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
    generation: NonZeroU64,
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

## 10. Host conformance acceptance tests (future AniCraft adapter)

The FDM-side contract tests are implemented in `form-deform-core`. End-to-end host acceptance remains blocked until AniCraft separately supplies the capabilities listed in §11.9. The future host adapter must demonstrate:

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

## Draft history

The original unresolved decision list is superseded by the resolved proposal and host-side conformance blockers in the next section. Do not treat this historical draft list as a second active decision list.

## 11. Contract decisions — resolved proposal and host-side blockers

This section closes the FDM-side semantic choices for review. These are proposed contract rules, not claims that AniCraft already implements them. Host-side conformance remains a blocker before an adapter can be written.

### 11.1 Persistent document identity

- `document_id` is a host-issued, opaque UUID (or equivalent collision-resistant persistent identifier) stored in the document's serialized root.
- Save/reopen, rename, and moving the same document preserve the ID.
- Duplicate/clone and Save As create a new document identity. The source document remains unchanged.
- Importing content into an existing document does not import the source document identity. Imported targets receive identities allocated by the destination host.
- A reference is scoped to exactly one document identity. Matching a numeric target ID in another document is never sufficient.
- A missing or malformed document identity is an explicit resolution failure; the adapter must not synthesize a stable ID at load time and pretend it was persisted.

This requires a future AniCraft document-model change. No such change is included in this PR.

### 11.2 Persistent target identity and incarnation

The persistent identity tuple is:

`(host_namespace, document_id, target_kind, target_id, generation)`

- `target_id` is opaque to FDM. For the current AniCraft model it may encode a decimal document-scoped `ElementId`; FDM must not parse it or infer identity from its numeric value.
- `generation` is a host-issued, non-zero incarnation token. It changes when a target is deleted and a new target is created, even if the host reuses the same underlying numeric ID.
- Rename, reorder, reparent, visibility changes, and selection changes do not change target identity.
- A target kind change is represented as deletion of the old typed target and creation of a new typed target; it must not silently preserve an incompatible binding.
- The host must guarantee uniqueness of the full tuple for the lifetime of all persisted references. If the host can formally guarantee never-reused IDs for all supported mutation/import/migration paths, a constant generation may be used only after that guarantee is documented and tested. Until then, generation remains mandatory.
- FDM-owned IDs, runtime handles, array indices, display names, and pin indices are never substitutes for this tuple.

### 11.3 Initial target and payload scope

- The first host geometry target is a shape/element that exposes canonical authored 2D path geometry.
- The first payload kind is `Path2D` only. Mesh and point-set payloads are deferred; no implicit tessellation or conversion is permitted.
- For an unwarped shape, the canonical source is its authored `Shape.path`.
- For a shape with native Asset Warp, the canonical source is `Warp.rest`, not the bent/shown `Shape.path`.
- Native Asset Warp pin IDs and pin positions are not part of the source path payload. They are a separate host-owned state/dependency and must not be smuggled into the path fingerprint or geometry revision.
- The containing shape is the M1 geometry target. A native warp pin is not itself the source-geometry target identity. Any future pin-level binding needs its own explicit stable identity contract.

### 11.4 Revision semantics, including undo/redo

Geometry and topology revisions are independent, target-scoped, non-zero unsigned 64-bit change epochs.

- Both start at 1 when a target's revision history is created.
- `geometry_revision` advances whenever any canonical Path2D payload field or its coordinate interpretation changes.
- `topology_revision` advances only when path structure changes: segment count/order, segment kind, endpoint/control-point structure, or closure structure.
- A topology edit advances both geometry and topology revisions.
- A coordinate-only edit advances geometry revision only.
- A transform, hierarchy, display-name, selection, or native Asset Warp pin-only edit does not advance either source-path revision.
- Undo and redo are new state transitions. If undo restores byte-identical old path content, the fingerprint may return to its earlier value, but geometry revision still advances. If topology is restored/changed, topology revision also advances. Revisions never move backwards or get restored from an undo snapshot.
- Revisions never wrap. Exhaustion is a hard host error; the host must not reset a counter while references may survive.
- A host migration that cannot preserve revision history must explicitly invalidate/reissue the document identity or provide a documented migration mapping. It must not silently reset revisions under an unchanged target reference.

A revision is a change epoch, not a content hash and not an undo-history index. Equality means the host asserts no relevant source change since that epoch.

### 11.5 Snapshot atomicity and immutability

- A snapshot is a complete value captured at one logical host-document state.
- The host must capture target identity, source payload, coordinate descriptor, revisions, and fingerprint atomically under one document read transaction/lock or equivalent consistent snapshot mechanism.
- FDM receives ownership of an immutable snapshot value (or read-only transport buffer). It can retain it for evaluation, but cannot mutate it or write through to the host.
- In Rust, snapshot fields are private and consumers access payload only through shared references. Transport implementations must provide equivalent read-only semantics.
- A snapshot remains unchanged after subsequent host edits. A newer snapshot is a new value with new dependency stamps.
- A snapshot is not proof that its source remains current. Freshness must be checked against current host state immediately before any future result-application operation.

### 11.6 Fingerprint canonicalization

The implemented algorithm identifier is `fdm-path2d-sha256-v1`. It hashes the FDM-owned canonical Path2D payload, not AniCraft's native object layout. An AniCraft adapter must map the host's authored path into this schema before constructing a snapshot.

The exact canonical byte stream is:

1. ASCII domain prefix `FORMA-DEFORM\\0PATH2D\\0`.
2. Schema version `1` as little-endian `u32`. The payload kind is fixed as `Path2D` by the domain and schema.
3. Coordinate descriptor, in this exact order:
   - Coordinate-space ID: UTF-8 bytes preceded by byte length as little-endian `u32`.
   - Space-kind tag: `0x01` = `TargetLocalRest`.
   - Dimensionality: one `u8` (must equal 2).
   - Source linear unit and canonical linear unit: each UTF-8 with little-endian `u32` byte length.
   - Unit scale: little-endian IEEE-754 binary64.
   - Axis tag: `0x01` = X-right/Y-up; `0x02` = X-right/Y-down.
   - Origin tag: `0x01` = top-left; `0x02` = bottom-left; `0x03` = center.
4. Command count as little-endian `u64`.
5. Commands in authored order, each with a one-byte tag and its coordinate values:
   - `0x01` MoveTo: x, y.
   - `0x02` LineTo: x, y.
   - `0x03` QuadraticTo: control x, control y, endpoint x, endpoint y.
   - `0x04` CubicTo: control1 x/y, control2 x/y, endpoint x/y.
   - `0x05` Close: no coordinate values.
6. SHA-256 over the complete byte stream. The resulting digest is exactly 32 bytes.

Every coordinate and scale is finite. NaN and infinities are rejected before hashing. Negative zero is normalized to positive zero. All finite floating-point values are encoded as little-endian IEEE-754 binary64; no rounding, quantization, or stringification is performed. Strings are UTF-8 and length-prefixed, command order and command tags are significant, and no object memory, map iteration order, runtime IDs, pointers, display names, target identity, or revision numbers enter the digest.

Path2D v1 requires a non-empty command list beginning with MoveTo. LineTo and curve commands require an active subpath; Close requires an open subpath. A new MoveTo starts another subpath. Point construction rejects non-finite coordinates. The FDM-owned Path2D stores its command vector privately and exposes only shared access.

Snapshot construction validates the Path2D payload kind and schema version, calculates the fingerprint from the payload plus coordinate descriptor, and rejects a supplied fingerprint that does not match. This prevents a host adapter from accidentally pairing a revision stamp with unrelated geometry bytes. Revisions remain change epochs; the fingerprint remains a content-verification token. Any future change to this canonical byte stream requires a new algorithm identifier.

### 11.7 Coordinate-space semantics

- The first snapshot space is `TargetLocalRest`: coordinates are in the target's authored rest/source path space, not stage space and not post-warp displayed space.
- The host must explicitly declare source linear unit, canonical linear unit, conversion scale (canonical units per source unit), axis orientation, origin, and stable coordinate-space ID.
- FDM must not assume that AniCraft's stage-space `warp.info` coordinates and artwork-space rest coordinates are interchangeable.
- A coordinate descriptor change changes the interpretation of the payload, so it advances geometry revision and changes the fingerprint.
- Unknown or unsupported coordinate metadata fails closed. No implicit stage/local conversion is allowed in M1.

The actual AniCraft axis/origin/unit values must be confirmed against source-level tests before adapter implementation; this proposal deliberately does not guess them.

### 11.8 Freshness and failure semantics

- A current snapshot has the exact same full target identity, geometry revision, topology revision, and content fingerprint as the host's current source state.
- A different target identity is a resolution/identity error, not a merely stale geometry snapshot.
- Any geometry revision, topology revision, or fingerprint mismatch marks the snapshot stale. A lower current revision is also a mismatch; revision rollback is never treated as current.
- A topology revision mismatch invalidates topology-dependent mappings even if coordinates happen to compare equal.
- A fingerprint mismatch with equal revisions is a host consistency/integrity error and must be reported distinctly from an ordinary edit.
- No automatic rebase, silent retry against different geometry, or acceptance based only on matching fingerprints is permitted.

### 11.9 Remaining host conformance blockers

The FDM-side semantics and canonical Path2D v1 encoding above are specified and implemented as the M1 proposal. Before implementation against AniCraft, the host must provide evidence for:

1. Persistent document UUID storage and duplicate/Save As/import semantics.
2. Target incarnation guarantees across deletion, reuse, import, undo/redo, and migration.
3. A stable shape target-kind/capability contract.
4. A deterministic adapter mapping from AniCraft's authored path model into the FDM-owned Path2D v1 schema.
5. Per-target monotonic revision updates on every mutation path, including undo/redo.
6. Atomic snapshot acquisition from `Warp.rest` when native Asset Warp exists, otherwise `Shape.path`.
7. Verified source units, axis orientation, origin, and coordinate-space ID.
8. A conformance test proving fingerprint stability and mismatch detection.

These are AniCraft-side requirements, not permission to modify AniCraft. They must be separately reviewed and authorized.

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
