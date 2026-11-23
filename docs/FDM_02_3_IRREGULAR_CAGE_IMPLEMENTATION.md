# FDM-02.3 — Irregular Cage Implementation

## Purpose

Implement the concrete cage type already declared by the FDM-02.1 topology contracts. Unlike a regular lattice, an irregular cage receives an explicit list of stable control-point IDs and explicitly ordered triangle/quad faces.

## Public API

- `IrregularCage::new(topology, control_state)` constructs and validates a cage.
- `IrregularCage::validate_geometry()` revalidates topology, state, connectivity, and geometry.
- `IrregularCage::boundary_control_point_ids()` returns the deterministic outer boundary order.

The cage uses the existing `IrregularCageTopology`, `TopologyFace`, and `DeformationControlState` types. It does not allocate IDs. The caller remains responsible for reserving IDs and never reusing deleted IDs.

## Accepted face model

- Faces may be triangles or quadrilaterals.
- Mixed triangle/quad meshes are accepted.
- Vertex order is significant and authored; validation never reverses or repairs it.
- All faces must share one winding convention.
- Quads must be strictly convex. Triangles must have non-zero area.
- Every topology control point must be referenced by at least one face.

## Structural validation

1. Validate unique point IDs, face references, and no repeated vertex within a face.
2. Build an undirected edge-incidence map while retaining each face's directed edge.
3. Reject edges incident to more than two faces (non-manifold edges).
4. Require the two incidences of each interior edge to traverse it in opposite directions.
5. Traverse face adjacency and require exactly one connected face component.
6. Extract boundary edges (edges incident to one face). Require exactly one directed loop: every boundary vertex has one incoming and one outgoing edge, and the loop must consume every boundary edge.
7. This excludes disconnected components, boundary branches, and holes/multiple boundary loops.

## Geometric validation

- Finite control-point positions.
- Scale-aware tolerance: `max(1e-12, bounding_box_diagonal * 1e-9)`.
- Spatial-hash duplicate/near-duplicate point rejection.
- Zero-length face-edge rejection.
- Degenerate area and inconsistent face winding rejection.
- Strict convexity for quads.
- Zero-length and self-intersecting outer boundary rejection.
- Non-degenerate outer boundary area.

No geometry is silently clamped, reordered, triangulated, or repaired.

## Resource limits

- 100,000 control points.
- 100,000 faces.
- 4,096 boundary vertices, because boundary self-intersection validation is pairwise.

These are implementation safety limits, not mathematical limits of irregular cages.

## Deliberate limitations

- Global overlap/intersection checks between non-adjacent interior faces are not implemented here. A cage can therefore pass the present checks while containing overlapping interior regions that do not create an invalid outer boundary. A later mesh-embedding validation step must close this gap before relying on arbitrary user-edited cages in production.
- No cage editing operations, ID allocator/persistence, path binding, deformation evaluation, or AniCraft integration are introduced.
- No tests, formatter, Rust build, or WASM build were run for this step, following the instruction to skip tests and proceed.

## Relationship to regular lattices

Both cage types implement the common `TopologyView` contract and use the same stable IDs and independent topology/control-state revisions. Regular lattice connectivity remains implicit; irregular cage connectivity remains explicit. Neither type is converted into AniCraft's native Asset Warp pins.
