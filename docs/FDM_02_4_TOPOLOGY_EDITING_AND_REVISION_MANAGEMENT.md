# FDM-02.4 — Topology Editing and Revision Management

## Scope

This step adds transactional editing for irregular cage topology and separates structural revision changes from control-state changes. It does not implement regular-lattice dimension editing because lattice connectivity is implicit and requires dedicated row/column insertion/removal semantics.

## Structural editing API

Implemented in `crates/form-deform-core/src/topology_edit.rs`:

- `ControlPointIdAllocator`: monotonic non-zero ID allocator with serializable-owner-friendly next-ID state access and restoration.
- `IrregularCageTopologyEdit`: batch operations to add/remove points and add/remove/replace faces.
- `IrregularCage::apply_topology_edits`: applies a batch to a candidate copy, increments topology revision once, synchronizes the control state's topology revision, validates the final cage, and commits only if valid.

Operation ordering is explicit:
- Add a point before adding faces that reference it.
- Remove or replace incident faces before removing their points.
- Removing a referenced point is rejected; incident faces are never silently deleted.
- Face removal/replacement uses the authored face value and requires exactly one match. This avoids introducing unstable face indices as persistent identities.

A failed batch leaves the cage unchanged. IDs already allocated during that failed batch remain consumed in the allocator, preventing accidental reuse.

## Revision semantics

| Operation | Topology revision | Control-state revision |
|---|---:|---:|
| Add/remove point | +1 per successful batch | unchanged |
| Add/remove/replace face | +1 per successful batch | unchanged |
| Move control point | unchanged | +1 per actual change |
| Pin/unpin control point | unchanged | +1 per actual change |
| No-op position/pin update | unchanged | unchanged |
| Failed operation/batch | unchanged | unchanged |

Each successful structural batch increments topology revision exactly once, regardless of the number of operations in the batch. It also updates the topology revision carried by `DeformationControlState`. The independent control-state revision is preserved.

Each successful control-state change increments its own revision exactly once. Moving a point is validated against cage geometry; invalid movement is rejected atomically. Pin-only changes do not alter geometry.

Revision overflow is rejected rather than wrapping.

## Identity allocation and persistence boundary

The allocator never decrements or reissues an ID during its lifetime. If an allocated ID is part of a failed batch, it remains consumed. The allocator exposes `next_id()` and `from_next_id()` so the document owner can persist and restore its high-water mark.

This module does not itself persist allocator state. The document/persistence owner must store the next-ID value atomically with the topology and must restore it before accepting further edits. Restoring an old allocator high-water mark from a stale save would violate non-reuse; persistence integration remains a prerequisite for durable guarantees across save/reopen.

## Transaction and validation behavior

- Edits operate on a clone and replace the live cage only after final validation succeeds.
- Structural batches are validated after all operations, allowing a point and its faces to be added together without exposing an invalid intermediate cage.
- A point cannot be removed while any remaining face references it.
- Existing geometric validation remains the final gate. The known global interior-face overlap limitation from FDM-02.3 remains unresolved.
- The allocator is intentionally not rolled back after a failed batch, preserving the no-reuse property.

## Deliberate limitations

- Editing is implemented for irregular cages only. Regular lattice edits need explicit semantics for inserting/removing rows and columns while preserving stable IDs and deterministic implicit connectivity.
- Faces do not yet have their own stable IDs. Face removal/replacement therefore matches a unique authored face value; duplicate face records are rejected as ambiguous for those operations.
- Durable allocator persistence and atomic save integration are not implemented here.
- No UI, undo/redo history, collaboration conflict resolution, Path2D binding, deformation evaluation, or AniCraft integration is included.
- Tests, formatter, Rust builds, and WASM builds were not run, following the existing instruction to skip tests and proceed.

## Files

- `crates/form-deform-core/src/topology_edit.rs`
- `crates/form-deform-core/src/lib.rs`
- `docs/FDM_02_4_TOPOLOGY_EDITING_AND_REVISION_MANAGEMENT.md`
