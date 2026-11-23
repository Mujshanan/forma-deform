# FDM-02.8 — Persistent Cage Bindings and Stale-Binding Protection

## Purpose

Persist FDM cage attachments as authored document data and fail closed whenever the captured source geometry or cage identity no longer matches the current source. This milestone extends the FDM-02.7 barycentric binding representation and the existing FDM document persistence envelope.

## Durable record

Each `PersistedCageBinding` stores:

- stable binding ID and binding revision;
- host target identity, including target generation;
- captured `GeometrySnapshotStamp` (geometry revision, topology revision, and canonical fingerprint);
- coordinate-space descriptor;
- sorted stable cage vertex IDs;
- canonical triangle identity triples (sorted vertex IDs, with triangles sorted as a collection);
- attached point IDs, stable support vertex IDs, and barycentric weights.

No evaluated positions, transient caches, or host object pointers are serialized. The host remains responsible for embedding the FDM payload in the native document and for dirty state, save/load transactions, and undo/redo.

## Document compatibility

`FdmDocument` now contains an optional `cage_bindings` collection with a serde default. Existing v1 documents that do not contain this field load with an empty binding collection. The document schema version remains v1 because this is an additive optional field and old documents remain readable; all new writes include the collection.

The existing `FdmDocument::new` constructor remains source-compatible and creates an empty binding collection. `new_with_cage_bindings` is available for callers that author or restore binding records.

## Validation and limits

- Binding IDs are unique within an FDM document.
- Host target and captured stamp identities must match.
- Revisions and identifiers must be valid.
- Vertex IDs and triangle identities must be unique and canonical.
- Every point support must refer to one stored cage triangle.
- Barycentric weights must be finite, non-negative within tolerance, and sum to one within `1e-9`.
- Resource limits bound record count, cage vertices, triangles, and bound points.
- Unknown record fields fail closed through serde's `deny_unknown_fields`.

## Stale-binding decision

Call `PersistedCageBinding::freshness_against_cage` with the current host snapshot stamp and current cage vertex/triangle identities before evaluation.

The returned `BindingFreshness` distinguishes:

- `Current`: the binding may be evaluated.
- `GeometryChanged`, `TopologyChanged`, `FingerprintChanged`: source changed; explicit rebind required.
- `GeometryRevisionRegressed`, `TopologyRevisionRegressed`: host revision integrity/regression; do not evaluate.
- `FingerprintIntegrityError`: unchanged revisions with changed fingerprint; treat as a host snapshot integrity failure.
- `TargetMismatch`: target identity or generation changed; binding belongs to another target.
- `CageVertexSetChanged`, `CageTopologyChanged`: actual cage identity changed even if the host failed to advance a revision.

Only `BindingFreshness::Current` is safe to evaluate. All other states require rejection and user-visible stale status; none trigger automatic rebinding.

## Conversion

`CageBinding::to_persisted` converts a runtime binding into the validated durable record. The host adapter supplies the authoritative target, reference snapshot stamp, and coordinate-space descriptor.

## Tests

Coverage includes record serialization round-trip, full FDM document round-trip, legacy documents without the optional collection, duplicate binding IDs, malformed support/weights, current-vs-stale source revisions, revision regression, fingerprint integrity failure, and cage vertex/topology mismatch.

## Non-goals

- Host document storage implementation.
- Automatic rebinding or topology repair.
- Silent use of stale records.
- Replacing or duplicating FDM-02.6 global cage validation.
- Persisting evaluated geometry or runtime caches.
