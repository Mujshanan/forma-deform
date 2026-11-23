# FDM-02.6 — Global Cage Embedding Validation

**Branch:** `feat/fdm-02-6-global-cage-embedding-validation`  
**Scope:** Forma Deform core only. No AniCraft host changes and no binding model.

## Purpose

FDM-02.6 establishes whether a complete indexed triangular cage is a valid, unambiguous planar embedding. Per-vertex and per-face checks cannot detect global self-intersection, disconnected face containment, or invalid edge incidence. Bindings must not be authored against a cage until this whole-cage validation succeeds.

## Canonical input and output

The implementation is in `crates/form-deform-core/src/cage_embedding.rs`:

- `CageVertex`: stable nonzero-or-zero-opaque ID plus finite `Point2` position. The validator requires IDs to be unique; it does not allocate IDs.
- `CageTriangle`: three indices into the cage vertex array.
- `CageEmbedding`: the complete vertex and triangle collections.
- `CageEmbedding::validate_global`: validates the complete embedding and returns `ValidatedCageEmbedding` with vertex, face, boundary-edge, interior-edge, and connected-component counts.

The module uses the existing canonical `Point2` primitive. It does not define another coordinate space, target identity, geometry snapshot, topology revision, or fingerprint.

## Validation rules

1. The cage has at least one vertex and one triangle.
2. Stable vertex IDs are unique, and distinct vertices are not coincident within the declared numerical tolerance.
3. Every face references three in-range, distinct vertex indices.
4. Duplicate faces are rejected independent of vertex ordering.
5. Every face has non-zero, numerically resolvable signed area.
6. All faces use one consistent winding.
7. Every undirected edge has one incident face (boundary) or two incident faces (interior); more than two is non-manifold.
8. Two faces sharing an edge traverse that edge in opposite directions.
9. Non-adjacent edges do not cross or touch.
10. No disconnected face lies wholly inside another face.

The validator does not reorder vertices, repair winding, weld vertices, split intersections, triangulate input, or infer missing topology.

## Numerical policy

Coordinates are finite f64 values. Orientation and segment predicates use a scale-aware tolerance derived from the cage bounding-box diagonal (`1e-12` relative scale). Near-degenerate or numerically ambiguous cases fail closed. This is a conservative floating-point policy, not an exact-arithmetic geometric kernel; a future exact-predicate implementation would be a separately versioned semantic change.

## Scope boundary

This is a pure, host-independent geometry gate. It does not create or persist bindings, map cage vertices to artwork, evaluate deformation, mutate a document, or schedule host work. The caller must retain the validated cage and its identity/topology revision, and must revalidate when the source topology or coordinates change. A successful result is valid only for the exact input value that was checked.

## Tests

The module tests cover a valid two-triangle square, summary counts, duplicate identities and positions, invalid indices, duplicate and degenerate faces, inconsistent winding, non-manifold incidence, crossing edges, fully contained overlapping faces, and valid disconnected components.

## Verification status

The source has been added to the existing FDM-02.6 branch. Rust formatting, Clippy, workspace tests, and CI have not been run by this editing session. Do not treat the milestone as verified until those checks pass.
