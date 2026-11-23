# FDM-02.2 — Regular Lattice Implementation

**Status:** Initial construction and geometric validation implementation committed; tests/builds intentionally not run.

## Implementation

The implementation is in `crates/form-deform-core/src/regular_lattice.rs`, exported as `form_deform_core::regular_lattice`.

### Deterministic rectangular construction

`RegularLattice::rectangular` accepts:
- A topology ID and topology revision.
- An independent control-state revision.
- Row and column counts.
- Finite minimum and maximum rectangle corners.
- A caller-reserved first control-point ID.

It creates row-major points by linearly interpolating Y across rows and X across columns. Each point receives a stable ID from the caller-reserved contiguous ID range. Generated points begin unpinned. The caller remains responsible for persistent ID allocation and never-reuse semantics.

Dimensions must be at least 2×2. The checked point count is limited to 1,000,000. Bounds must have finite, strictly positive width and height. ID arithmetic is checked for overflow.

### Connectivity

The existing `RegularLatticeTopology::face_at` defines each cell in deterministic TL, TR, BR, BL order. Cells remain quads in this step; no diagonal or triangulation policy is selected. This avoids silently choosing a triangulation before the deformation operator requires one.

### Geometry validation

`validate_geometry` checks:
- Topology/control-state identity and revision agreement.
- Exactly one finite control state per topology point.
- Scale-aware tolerance: max(1e-12, bounding-box diagonal × 1e-9).
- Coincident or near-coincident control points using spatial hashing.
- Zero-length/sub-tolerance cell edges.
- Degenerate cell area.
- Consistent cell orientation (rejects inverted cells).
- Convexity of every quad (rejects folded/non-convex cells).
- A simple outer boundary, non-zero boundary edges, no boundary self-intersections, and non-degenerate boundary area.

Validation fails closed. It does not reorder points, repair geometry, clamp coordinates, or reverse faces.

## Resource limits

- Maximum control points: 1,000,000.
- Maximum boundary vertices for the current pairwise segment-intersection check: 4,096.

The boundary cap bounds the quadratic simple-polygon intersection check. It is an explicit implementation limit, not a mathematical lattice limit. A later scalable sweep-line boundary validator can raise or remove it.

## Important limitations

This step does not yet prove that arbitrary convex, consistently oriented cells form a globally non-overlapping tessellation. The simple outer boundary and local cell checks reduce invalid cases, but a dedicated global cell-overlap / mesh embedding check remains a follow-up before treating arbitrary edited lattices as production-safe.

Also deferred:
- Interactive lattice editing and ID allocation persistence.
- Triangulation/diagonal selection and interpolation.
- Binding to Path2D and outside-domain policy.
- Irregular cages.
- Persistence schema v2 and migration.
- AniCraft host adapter/UI integration.

No changes were made to AniCraft native Asset Warp or Smart Controller behavior.

## Verification

Per the user's instruction, no tests, formatting, Rust builds, or WASM builds were run. The code is therefore unverified. Before relying on it, add and run coverage for:
- 2×2, non-square, and high-resolution grids.
- Stable IDs and row-major connectivity.
- Reordered control-state storage.
- Zero/overflow IDs and unreasonable dimensions.
- Coincident points and near-tolerance points.
- Degenerate/inverted/non-convex cells.
- Self-intersecting boundary and boundary resource limit.
- Serialization round-trips.

## Next

FDM-02.3 — Irregular Cage Implementation, using the same stable point IDs, revision separation, geometry tolerance policy, and shared TopologyView interface.
