# FD-M7 — Multi-axis keyform interpolation evaluation

**Decision:** proceed with a native, host-independent regular pose-grid mapper using multilinear interpolation. Do not port or add an Aether Canvas dependency. Keep Aether Canvas as an algorithmic/design reference only.

**Status:** evaluated; implementation not started by this document.

## 1. Product gap and existing foundations

The M7 five-pose joystick mapper supports two scalar inputs and five fixed anchors (center/up/right/down/left), blended with inverse-squared-distance weighting. It is useful for a compact joystick, but it does not represent a regular authored grid, arbitrary key values, or an explicit keyform at every grid intersection.

The repository already has the pieces needed for a native implementation:

- `MappingConfig` defines canonical mapping variants and validates local model invariants.
- `Controller.inputs` is an ordered list of stable `InputId` references.
- `Pose2D` stores typed values keyed by stable `BindingId`, rejects duplicate binding IDs, and validates pose-to-pose type and angle-semantic compatibility.
- `PoseLibrary` resolves runtime pose IDs without pretending to be durable document persistence.
- `TypedValue` and `Angle` define finite scalar, Vector2, and wrapped/unwrapped angle semantics.
- The evaluator preflights and resolves controller inputs; the mapper can remain pure and host-independent.

The new mapping should reuse these modules. It should not create a second pose library, input sampler, evaluator, or host scheduler.

## 2. Candidate source evaluation: Aether Canvas

Reviewed source: [Aether Canvas `crates/aether-rig/src/keyform.rs`](https://github.com/syuumaimikan/Aether-Canvas/blob/claude/aether-canvas-2d-suite-22102t/crates/aether-rig/src/keyform.rs), at repository tree revision `11f36260e6abca32a422798016dcda18ce1f93c5`.

Relevant design features:
- N-dimensional Cartesian keyform grids with non-uniform key values.
- Axis-local segment lookup and per-axis interpolation weights.
- Multilinear interpolation by multiplying per-axis weights.
- Axis 0 varies fastest in the flattened form array.
- Optional Catmull–Rom interpolation, cyclic axes, key insertion, and structural editing.
- A generic `Blend` trait for weighted accumulation.

### Reuse decision

**Reference only; no source code or dependency adoption.**

The source is conceptually useful, especially the axis-local weight calculation, Cartesian product indexing, and tensor-product interpolation. The complete implementation is not a drop-in fit: Aether's generic shape blending and editing model differs from Forma Deform's typed pose and BindingId contracts. Its cyclic parameter behavior and Catmull–Rom mode are not yet required by the product contract.

License/provenance gate: GitHub's repository metadata currently reports `license: null`; the default-branch tree has no root LICENSE or NOTICE file; the workspace `Cargo.toml` declares `MIT OR Apache-2.0`. That is not enough to establish file-level provenance and legal notices for copying. Do not port code or add a dependency unless this is resolved and exact file-level review is recorded.

## 3. Proposed Forma Deform contract

Add a new canonical mapping variant conceptually shaped as:

- `PoseGrid`
- `axes: Vec<PoseAxis>`, with one axis per ordered `Controller.inputs` entry
- each `PoseAxis` contains a strictly increasing finite list of authored scalar key values
- `pose_ids: Vec<String>`, ordered as a flattened Cartesian product with axis 0 varying fastest

This is a new mapping mode; do not change the semantics or serialized representation of `JoystickFivePose` or `PoseSet1D`.

### Input and grid rules

1. Require 2 or 3 axes for the first implementation. Axis order is exactly `Controller.inputs` order.
2. Each axis has at least two finite, strictly increasing keys. Duplicate or near-duplicate keys are rejected; do not silently sort or deduplicate authored data.
3. Each input must resolve to a finite Scalar. Angle/vector inputs are rejected for this first version; normalization belongs in explicit input definitions or a later mapping contract.
4. Values outside an axis range clamp to its nearest endpoint. No extrapolation.
5. Use checked multiplication to calculate the total number of grid forms. Cap the first version at 256 forms; reject larger grids before pose lookup or allocation.
6. The number of pose IDs must exactly equal the product of axis key counts. Pose IDs must be non-empty and unique.
7. Every referenced pose must exist. All poses must have exactly the controller's declared BindingId set and compatible typed values, including identical wrapped/unwrapped angle semantics.
8. The pose grid is runtime-resolved through the existing `PoseLibrary`. This does not add durable pose persistence.

### Interpolation

Use tensor-product multilinear interpolation over the enclosing cell:

- Find the lower and upper key and local (t in [0,1]) independently for each axis.
- Enumerate the (2^N) cell corners (at most 8 for the initial 3-axis limit).
- Compute each corner's weight as the product of the relevant (t) or (1-t) factors.
- Accumulate typed pose values by BindingId.
- Exact grid-key hits should return the exact authored pose (or have mathematically exact unit weight), preserving authored values.
- Scalar and Vector2 outputs use weighted arithmetic sums.
- Unwrapped angles use weighted arithmetic sums in radians.
- Wrapped angles use a weighted circular mean; reject a near-zero resultant as ambiguous. Do not sequentially interpolate corners in arbitrary order because wrapped-angle interpolation is order-dependent.

Multilinear interpolation is deterministic, continuous across cell boundaries, reproduces all authored keyforms, and does not overshoot between keys. It is a safer first default than Catmull–Rom for deformation values, where overshoot can produce pin excursions or unstable geometry.

## 4. Explicitly deferred

Do not include these in the first implementation:

- Catmull–Rom / cubic smooth interpolation. It can overshoot, requires a deliberate boundary policy, and complicates multidimensional tensor weights.
- Cyclic axes and seam wrapping. They need explicit cycle metadata and a defined seam interpolation contract.
- Missing-input fallback to a default pose. The existing evaluator's declared fallback/reject behavior must remain authoritative; a missing required grid input must not silently select a pose.
- Grid authoring/edit operations such as inserting keys or axes while preserving appearance.
- Additive blend-shape grids or composition across multiple controllers.
- Durable pose storage, host document changes, live playback, and scheduling.
- Unbounded dimensions or unbounded Cartesian products.

## 5. Test and acceptance plan

### Model validation
- Correct axis count versus ordered controller input count.
- 0/1 axis, 4+ axes, fewer than two keys, unsorted keys, duplicates, NaN/infinity, and overflowing/over-limit grid products.
- Pose ID count mismatch, duplicate/empty IDs, missing runtime pose.
- Binding ID mismatch, duplicate binding, scalar/vector/angle type mismatch, wrapped/unwrapped angle mismatch.

### Numerical behavior
- Every authored grid corner is reproduced exactly.
- Bilinear center and trilinear center produce expected weighted results.
- Non-uniform axis spacing produces correct local interpolation fractions.
- Values below/above each axis clamp to the boundary.
- Interpolation is continuous across adjacent cell boundaries.
- Axis permutation with correspondingly permuted flattened forms produces identical results.
- Deterministic repeated evaluation.
- Wrapped angles near the (-pi/pi) seam blend correctly; ambiguous circular result is rejected.
- Non-finite inputs are rejected before weights are produced.

### Regression and integration
- Existing `PoseSet1D` and `JoystickFivePose` behavior remains unchanged.
- Missing required input follows existing evaluator diagnostics/fallback policy.
- Preview remains pure and does not mutate host targets or authored poses.
- Run Rust workspace tests and WASM builds after implementation; AniCraft GUI testing is not required for this pure-core slice.

### Performance
- Benchmark representative 2D and 3D grids at the 256-form cap, with realistic binding counts.
- Record evaluation time and allocation count; avoid per-evaluation heap allocations where practical.
- Compare only against the existing five-pose IDW mapper as a context baseline, not as a direct quality-equivalent algorithm.
- Do not add a dependency for this feature.

## 6. Registry decision record

| Field | Decision |
|---|---|
| Capability | Multi-axis pose/keyform interpolation |
| Existing alternatives | `PoseSet1D`, `JoystickFivePose`, `PoseLibrary`, typed `Pose2D` |
| External source | Aether Canvas `keyform.rs`, revision `11f36260e6abca32a422798016dcda18ce1f93c5` |
| Reuse strategy | Native implementation; external reference only |
| License | Aether source-copy gate blocked pending file-level license/provenance confirmation |
| Dependencies | No new dependencies |
| Correctness gate | Typed pose validation, exact key reproduction, boundary and wrapped-angle tests |
| Performance gate | 2D/3D benchmark, <=256 forms |
| Status | **Approved for design; implementation pending** |

## 7. Go / no-go

**Go** for a bounded 2D/3D multilinear pose grid after the schema and validation contract above are accepted.

**No-go** for copying Aether Canvas code, adopting its crate, enabling Catmull–Rom/cyclic behavior, or expanding into editing operations in this slice.

This evaluation does not modify runtime code or AniCraft. It is the implementation brief for the next M7 change.
