# FDM-02.5 — Regular Lattice Editing and Unified Topology Editing

## Scope

This step completes the editing gap identified in FDM-02.4. Regular lattices receive explicit row/column editing semantics, and both topology representations are exposed through one editing facade. The facade dispatches to the native implementation for the active cage type; it does not convert regular lattices into irregular meshes or reinterpret irregular connectivity as a grid.

## Regular lattice structural edits

Implemented in `crates/form-deform-core/src/lattice_edit.rs`:

- `InsertRow { after_row }`: inserts a row halfway between the selected row and its following row.
- `RemoveRow { row }`: removes an existing row.
- `InsertColumn { after_column }`: inserts a column halfway between the selected column and its following column.
- `RemoveColumn { column }`: removes an existing column.

Insertion indices identify an existing interval, so insertion is currently between existing rows or columns. Edge extrapolation is intentionally excluded. Removal must leave at least two rows and two columns.

For inserted controls:
- Positions are computed by component-wise midpoint interpolation between corresponding neighboring controls.
- Fresh stable IDs come from the shared monotonic `ControlPointIdAllocator`.
- New controls start unpinned.
- Existing IDs and their positions/pin states are preserved.

The order of operations in a batch is sequential: each later index applies to the dimensions resulting from earlier operations.

## Transaction and revision semantics

- A batch is applied to a candidate copy.
- The topology revision increments exactly once after the full batch has been applied.
- The control state's topology-revision reference is updated to match.
- The control-state revision is unchanged for structural edits.
- The final candidate must pass structural and geometric validation before replacing the live lattice.
- On failure, the lattice is unchanged. IDs already allocated remain consumed and are never returned to the allocator.
- Row/column removal may make previously separated controls adjacent. The existing geometric validator is the final gate; invalid or folded resulting cells are rejected.

Control movement and pinning are also available for regular lattices. Actual position changes increment only the control-state revision; pin-state changes increment only the control-state revision. No-op updates do not increment revisions.

## Unified editing facade

`EditableDeformationCage` represents either:
- `RegularLattice(RegularLattice)`
- `IrregularCage(IrregularCage)`

`DeformationCageEdit` wraps the corresponding native edit operation. `apply_edits` accepts one batch and dispatches it to the active cage implementation. A batch containing the other cage type's edit variant is rejected. Mixed topology kinds are not implicitly converted.

The facade also exposes:
- `topology()` as a read-only `TopologyView`.
- `control_state()` as a read-only control-state reference.
- Unified `set_control_point_position` and `set_control_point_pinned` methods.

The existing irregular-cage editor remains the implementation for explicit face connectivity and point/face operations.

## Deliberate limitations

- Row/column insertion currently supports only intervals between existing grid lines; no extrapolating insertion at the outer boundary.
- Inserted controls are unpinned by default; pin interpolation policies are not inferred.
- No arbitrary regular-grid point insertion/deletion is allowed because it would destroy implicit rectangular connectivity.
- No face stable IDs, undo/redo, persistence integration, UI, collaboration, Path2D binding, deformation evaluation, or AniCraft integration is added.
- The existing global interior-face overlap limitation from FDM-02.3 remains unresolved.
- Tests, formatter, Rust builds, and WASM builds were not run, following the instruction to skip tests and continue.

## Files

- `crates/form-deform-core/src/lattice_edit.rs`
- `crates/form-deform-core/src/lib.rs`
- `docs/FDM_02_5_REGULAR_LATTICE_EDITING_AND_UNIFIED_TOPOLOGY_EDITING.md`
