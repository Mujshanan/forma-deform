# PR #47.3 — Path2D Persistence CAS and Stale-Write Hardening

## Scope

Close the Path2D binding persistence validation-to-commit race identified in PR #47.2. This changes only the host-independent Path2D persistence contract and its integration tests. The generic point-array cage-binding payload/adapter remains separate and unchanged.

## Contract changes

### Host-owned slot revision

- Add `BindingPayloadRevision`, a host-issued monotonically increasing revision for the Path2D binding slot. It is separate from source geometry/topology revisions and is not serialized into FDM JSON.
- Add `BindingSlotSnapshot`, which returns the slot revision and optional payload atomically. Revision zero denotes a never-mutated, empty slot.
- The host must retain the revision token across deletion. Every write and delete advances it; it must never reset when the payload becomes absent. This prevents an absent → present → absent ABA sequence from looking unchanged.
- Revision overflow must fail closed; the host must not wrap or reuse a revision.

### Atomic conditional write

`BindingHostAdapter::write_binding_payload` now receives:
1. The expected host-owned binding-slot revision.
2. The exact `GeometrySnapshotStamp` observed for validation.
3. The replacement payload.

The host adapter must compare both the current source stamp and current slot revision and replace the payload in one authoritative document transaction. It returns either `Written` or a structured `Conflict` indicating which precondition changed. Host I/O/transaction failures remain ordinary adapter errors.

`persist_binding` now captures the source stamp and slot revision, validates the binding against the captured context, serializes it, and performs the conditional write. It maps conflicts to `PersistBindingError::Conflict`; it never retries or silently revalidates/rebinds the payload.

Loading reads the payload from the same revisioned slot snapshot and continues to validate it against a current immutable geometry context. Evaluation freshness remains a separate responsibility: a validated capability is tied to the source stamp against which it was validated.

## Regression coverage

The in-memory integration adapter models the host's atomic comparison and covers:
- Initial write from revision zero and subsequent successful revision advancement.
- Source geometry changing after validation but before commit.
- A competing binding write after the slot read but before commit.
- A competing write followed by deletion (ABA); the tombstone revision still changes and rejects the stale writer.
- Rejection of an impossible never-mutated revision-zero slot containing a payload.
- Existing round-trip, stale-source, target-generation, and current-cage validation behavior.

These tests verify the core contract and conflict propagation. They do not prove a concrete AniCraft host transaction implementation: no concrete host adapter implementation is present in this repository. Any future host adapter must implement the comparison and replacement atomically in its authoritative document transaction.

## Compatibility and boundaries

- This is a breaking change to the `BindingHostAdapter` trait: implementers must return `BindingSlotSnapshot` and implement the two-precondition atomic write.
- The Path2D JSON envelope and schema version remain unchanged.
- The generic point-array `CageBindingHostAdapter` and its payload slot are not unified or modified.
- No document ownership, host UI, undo/redo implementation, cache, scheduling, apply/bake, or deformation algorithm is added.
- Smart Controller evaluation and Form Deform pin evaluation remain independent under CCC-01.5.

## Verification

The CI workflow is ordered so functional checks run before the known workspace-wide rustfmt debt reported in PR #47.2. The workspace formatting audit remains strict and visible at the end of the job; this PR does not mass-format the 71 pre-existing files. Functional CI evidence must be reported against the exact final PR head.
