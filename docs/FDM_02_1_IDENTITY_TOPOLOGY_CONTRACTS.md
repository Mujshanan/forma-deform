# FDM-02.1 — Stable Identity and Topology Contracts

**Status:** Contract types added; implementation unverified (tests intentionally skipped at the user's direction).

## Scope

This step establishes the authored identity and structural topology boundary used by FDM deformation cages. It does not implement lattice generation, cage construction/editing, binding, geometric validity checks, interpolation, or persistence migration.

The implementation is in `crates/form-deform-core/src/topology.rs`, exported by `form-deform-core::topology`.

## Identity rules

- `ControlPointId` is an opaque, non-zero FDM-owned identity. It is never an array index.
- `DeformationTopologyId` identifies one authored topology independently of its storage location.
- IDs are stable across control-point movement and storage reordering.
- A newly created point receives a new ID. A deleted ID is never reused by the future allocator/persistence layer.
- Host `HostTargetRef` and host Path2D command positions remain separate identity domains. FDM IDs must not be derived from AniCraft's `PinId`, path command index, or generated mesh index.

The types reject zero at construction and use `NonZeroU64` internally. The future allocator must additionally guarantee uniqueness and non-reuse across save/reopen and topology edits.

## Independent revision domains

| Revision | Changes when | Does not change when |
|---|---|---|
| `DeformationTopologyRevision` | Control-point membership, lattice dimensions, or face connectivity changes | A point moves or is pinned/unpinned |
| `ControlStateRevision` | A control point moves or pin state changes | Topology membership/connectivity stays unchanged |

Both revisions are non-zero and scoped to a topology identity. A control-state snapshot carries both the topology ID and topology revision it belongs to. It is rejected if either differs from the supplied topology.

Source Path2D revisions from FDM-01 remain a separate domain. Do not reuse `host_contract::TopologyRevision` for deformation-cage topology.

## Typed topology alternatives

### Regular lattice

`RegularLatticeTopology` stores row/column dimensions and row-major stable control-point IDs. Connectivity is implicit. The common read-only view exposes each cell as a consistently ordered quad; triangulation/diagonal policy belongs to the later lattice implementation step.

Structural validation currently checks:
- At least 2 rows and 2 columns.
- Exact checked product of rows × columns.
- Unique control-point IDs.

### Irregular cage

`IrregularCageTopology` stores stable control-point IDs and explicit triangular or quadrilateral faces. Face vertex order is significant.

Structural validation currently checks:
- At least three control points and one face.
- Unique control-point IDs.
- No repeated vertex within a face.
- Every face reference resolves to a cage control point.

### Shared view

Both alternatives implement `TopologyView`, exposing topology identity/revision, stable point IDs, and deterministic face access. `DeformationTopology` is a tagged enum; no implicit conversion between lattice and cage is defined.

## Control state

`DeformationControlState` carries:
- The topology ID and topology revision it was authored against.
- Its independent control-state revision.
- Exactly one finite position and pin flag for every topology control-point ID.

The point vector's order has no identity meaning. Validation compares stable ID sets, so storage reordering does not change identity.

Pinning here is authored control state only. A constraint solver, pin weighting, and solver semantics are outside FDM-02.1.

## Explicitly deferred

The following are not claimed as implemented by this step:

- Geometric validation: connectedness, single-component/single-boundary constraints, holes, winding, self-intersection, inverted/degenerate faces, duplicate coordinates, zero-length edges, or scale-aware tolerances.
- Lattice generation and interpolation/triangulation policy.
- Irregular cage editing and geometric validation.
- Source geometry binding and outside-domain policy.
- Bézier anchor/handle mapping and curve approximation/error policy.
- Schema v2 persistence, migration, and ID allocation/non-reuse enforcement.
- AniCraft UI/host adapter integration.

These remain subsequent FDM-02 steps. No AniCraft native Asset Warp or Smart Controller behavior is changed by this contract.

## Verification status

Rust/WASM tests and builds were deliberately not run, as requested. This commit is therefore an implementation draft, not a passing or conformance-verified result. Before relying on it, run formatting, workspace tests, serialization round-trips (including zero-ID rejection), and topology/control-state contract tests. Then proceed to FDM-02.2 and FDM-02.3 geometric implementations.
