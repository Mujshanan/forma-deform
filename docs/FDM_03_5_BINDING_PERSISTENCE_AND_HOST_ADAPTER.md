# FDM-03.5 — Binding Persistence, Stale-Binding Diagnostics, and Host Adapter Contract

## Status

The first implementation is committed but **unverified**. Rust tests, formatting, compilation, and CI have not been run. Do not treat this step as complete until the requested verification is performed and passes.

## Responsibility boundary

FDM owns:
- The versioned binding payload schema and serialization contract.
- Structural validation of deserialized binding data.
- Revalidation against the current immutable source snapshot and current certified cage.
- Typed stale-binding and invalid-data diagnostics.
- A host-neutral adapter interface for reading/writing opaque binding payloads.

AniCraft remains responsible for:
- Embedding the payload in its authoritative document model.
- Document transactions, dirty-state tracking, save/load, undo/redo, and recovery.
- Host target lookup and immutable snapshot creation.
- Supplying the current cage topology and authored control state.
- Deciding when to discard, retain, or explicitly rebind stale data.

FDM does not write files, access host documents, or own the host's transaction lifecycle.

## Persisted schema

The binding payload is separate from the existing `FdmDocument` schema:

- Schema name: `forma-deform-binding`
- Schema version: `1`
- Maximum encoded payload: 16 MiB
- Maximum feature entries: 1,000,000
- Unknown fields and unsupported schema versions fail closed.

The envelope contains one `BindingRecord`, including its complete `BindingStamp` and ordered feature entries. Serialization covers the source target/generation, source geometry and topology revisions, canonical fingerprint algorithm/digest, coordinate-space descriptor, payload schema, FDM topology identity/revision, binding schema and mapping-policy versions, outside-cage policy, and mapped/clamped/extrapolated/unbound feature records.

No evaluated output, runtime cache, cage embedding certificate, or adaptive curve-sample buffer is persisted. The certificate is recomputed from current topology and control state. Curve samples remain derived data; any future persisted sample cache must be independently keyed by the exact source stamp and sampling-policy version/values.

## Trust and load sequence

Deserialization is not validation. The safe load path is:

1. Read the opaque payload from host-owned storage.
2. Enforce byte limits, decode the known envelope, reject unknown fields/schema versions, and validate the binding stamp.
3. Ask the host adapter for the current immutable geometry snapshot, topology, and control state.
4. Validate the snapshot envelope and recompute the global cage-embedding certificate.
5. Construct `ValidatedCageGeometry` from that certificate.
6. Run `BindingRecord::validate_against` for complete source-feature coverage, exact locator ordering, mapping-policy compatibility, face/control-ID order, coordinate domains, outside-policy consistency, and source-point reconstruction.
7. Only then return `ValidatedBindingRecord`, the capability wrapper indicating that this exact record passed current validation.

The host must not pass deserialized records directly into evaluation. It must use the validated wrapper/load path.

## Stale-binding diagnostics

The loader distinguishes stale dependencies from malformed or invalid data:

| Diagnostic | Meaning |
|---|---|
| `StaleTarget` | Host target identity or generation changed |
| `StaleSourceGeometryRevision` | Source geometry revision changed |
| `StaleSourceTopologyRevision` | Source Path2D topology revision changed |
| `StaleSourceFingerprint` | Canonical source content changed |
| `StaleCoordinateSpace` | Coordinate interpretation changed |
| `StalePayloadSchema` | Source payload kind/schema changed |
| `StaleCageTopology` | FDM topology identity or revision changed |
| `InvalidCurrentSnapshot` | Host supplied an invalid snapshot |
| `InvalidCageEmbedding` | Current cage cannot be certified |
| `InvalidBinding` | Persisted data fails structural or geometric validation |

A stale binding is never automatically rebound, approximated, clamped under a different policy, or silently treated as valid. The host may keep the stale payload for undo/history or ask the user to explicitly rebind it; that decision is outside FDM.

Control-state revision is intentionally not a binding-staleness reason. Moving or pinning cage controls does not change the rest-space association, but it invalidates the embedding certificate and evaluation cache. The certificate is recomputed against the current control state before the binding is trusted again.

## Host adapter contract

`BindingHostAdapter` exposes only three operations:
- `current_binding_context(target)`: return an immutable source snapshot, topology, and control state.
- `read_binding_payload(target)`: retrieve the opaque FDM JSON payload, if present.
- `write_binding_payload(target, payload)`: store the payload in the host document.

The adapter must ensure that target lookup respects the full `HostTargetRef`, including generation. Writes must participate atomically in the host's document transaction. The adapter must not persist runtime caches or treat the FDM payload as a replacement for canonical Path2D or native Asset Warp data.

The generic `load_validated_binding` helper implements read, decode, and revalidation. Host-specific error types remain owned by the adapter.

## Compatibility and migration

This is the first persisted binding-envelope version. Unknown versions are rejected; no guessed migration is performed. The inner binding stamp continues to enforce its own schema and policy versions. Legacy binding-stamp acceptance remains limited to the explicitly supported v1/unbound policy path already defined by `binding.rs`; it is not a promise that arbitrary historical serialized payloads are readable.

A future migration must be named, deterministic, tested against real fixtures, and must never invent source point IDs, control IDs, face mappings, or policy parameters.

## Tests authored

- Oversized payload rejection before deserialization.
- Malformed/unknown envelope rejection.
- Existing binding validation continues to be the final gate after deserialization.

Additional integration tests should cover a full binding JSON round trip, each stale diagnostic, invalid cage certificates, and adapter load/write behavior once the host adapter is integrated.

## Files

- `crates/form-deform-core/src/binding.rs`
- `crates/form-deform-core/src/binding_persistence.rs`
- `crates/form-deform-core/src/lib.rs`
- `docs/FDM_03_5_BINDING_PERSISTENCE_AND_HOST_ADAPTER.md`
