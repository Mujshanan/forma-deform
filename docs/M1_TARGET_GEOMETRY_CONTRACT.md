# M1 — Stable Target Identity and Revisioned Geometry Snapshots

**Status:** FDM-side source-geometry contract and implementation proposal. Host conformance remains blocked on separately reviewed AniCraft changes.

## 1. Purpose

Define the stable identity and immutable source-geometry snapshot boundary between AniCraft and Forma Deform. This contract is the first layer of FDM v2 M1; the evaluated-geometry and host lifecycle requirements are completed in [M1 — Evaluation and Host Lifecycle Contracts](M1_EVALUATION_LIFECYCLE_CONTRACT.md), with acceptance tests in [M1 — Host Conformance and Acceptance Test Matrix](M1_HOST_CONFORMANCE_TEST_MATRIX.md).

## 2. Contract decisions

- Persistent document identity is host-issued and survives save/reopen/rename/move; duplicate/clone/Save As allocate a new identity; imports receive destination-owned target identities.
- Persistent target identity is the tuple `(host_namespace, document_id, target_kind, target_id, generation)`. Generation is mandatory and non-zero.
- M1 targets are shape/element Path2D only: `Shape.path` when unwarped and `Warp.rest` when native Asset Warp exists. Pins remain separate host-owned state.
- Geometry and topology revisions are independent, non-zero monotonic change epochs. Topology changes advance both; coordinate/payload changes advance geometry; transform/display/selection/native pin-only changes do not advance source-path revisions. Undo/redo advance epochs.
- Host snapshot capture must atomically pair target identity, payload, coordinate descriptor, revisions, and fingerprint.
- Coordinate semantics are explicit `TargetLocalRest`; no assumed unit, axis, origin, or stage/local conversion.
- Freshness requires exact target identity, revisions, and fingerprint. Identity mismatch, stale source, revision regression, and equal-revision fingerprint integrity failure are distinct outcomes.

## 3. FDM-side implementation

- `host_contract`: opaque identity types, validated revision/fingerprint/coordinate metadata, Path2D-only immutable snapshot envelope, dependency stamps, and freshness comparison.
- `path2d`: owned immutable Path2D v1 schema, command validation, deterministic SHA-256 canonicalization (`fdm-path2d-sha256-v1`), negative-zero normalization, and coordinate-space-sensitive fingerprints.
- Snapshot construction validates payload kind/schema and recomputes the digest to reject a fingerprint that does not match the actual payload.
- Native tests cover identity/generation, invalid revisions and coordinates, command structure, fingerprint determinism and sensitivity, a golden SHA-256 vector, valid/invalid snapshot construction, payload/fingerprint mismatch, freshness, and revision regression.

## 4. Validation

GitHub Actions CI run #514 passed on head `86e584d4590033e4978499a8014c68aafca2c3db`: Bun typecheck/tests/build, `cargo test --workspace`, and all three release WebAssembly plugin builds. This validates the FDM-side code and test suite only; it does not establish host conformance.

## 5. AniCraft-side blockers (not included)

The inspected host model still requires a separately reviewed implementation for persistent document UUIDs, target incarnation, monotonic per-target revisions, atomic immutable snapshots, and verified coordinate metadata. The adapter must map AniCraft's authored path into the FDM-owned Path2D v1 schema. These are future host-conformance requirements, not authorization to change AniCraft.

## 6. Preserved boundaries

- No AniCraft source changes.
- No live playback or evaluation-result application protocol in this source-snapshot slice.
- Native Asset Warp and existing pin tweening are untouched.
- CCC-01.5 remains in force: Smart Controller binding evaluation and Form Deform pin evaluation remain independent; no multi-source composition.
- This PR remains a draft and is not merged.

## 7. Related contract documents

- [M1 — Evaluation and Host Lifecycle Contracts](M1_EVALUATION_LIFECYCLE_CONTRACT.md)
- [M1 — Host Conformance and Acceptance Test Matrix](M1_HOST_CONFORMANCE_TEST_MATRIX.md)
- [FDM v2 Architecture](FDM_V2_ARCHITECTURE.md)
- [FDM v2 Host Capability Audit](FDM_V2_HOST_CAPABILITY_AUDIT.md)
