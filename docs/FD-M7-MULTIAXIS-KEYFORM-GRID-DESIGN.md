# FD-M7 — Multi-axis keyform grid evaluation design

**Status:** design evaluated; implementation not started.
**Decision:** approve a host-independent, multilinear tensor-grid mapper as the next M7 algorithm, subject to the schema and test gates below.
**Reuse strategy:** extend Forma Deform's existing pose/value/mapping infrastructure; use Aether Canvas as an algorithmic reference only. Do not copy its implementation or add dependencies.

## 1. Problem and fit

The current M7 JoystickFivePose mapping supports exactly two scalar inputs and five fixed anchors (center/up/right/down/left). It is intentionally not a complete keyform grid: there are no authored diagonal corners, variable key positions, or additional axes.

A multi-axis keyform grid is the natural next step because it generalizes authored poses to a regular N-dimensional parameter space while reusing:
- PoseLibrary for runtime pose-ID lookup;
- Pose2D for typed values keyed by BindingId;
- TypedValue and its scalar/vector/angle semantics;
- the existing canonical mapper and evaluator preflight/diagnostics;
- the current pure, host-independent mapping boundary.

It should be a separate mapping variant, not a silent change to JoystickFivePose. The five-pose field remains useful when an artist wants cardinal controls without authoring a complete grid.

## 2. Registry decision

| Registry candidate | Finding | Decision |
|---|---|---|
| Existing internal PoseLibrary | Already resolves unique runtime pose IDs | **Reuse** |
| Existing Pose2D / TypedValue | Already validates binding identities and value kinds; scalar, Vector2 and Angle are supported | **Reuse**, with a shared weighted-pose helper considered to avoid duplicate blend code |
| Existing canonical mapper | Owns pure mapping and rejects missing/ambiguous inputs | **Extend** |
| AniCraft host systems | No host-side keyform-grid capability is needed for pure interpolation | **No host API change** |
| Aether Canvas aether-rig/src/keyform.rs | Implements N-dimensional tensor grids, axis-local weights, axis-0-fastest flattening, and multilinear evaluation; also has Catmull-Rom, cyclic axes and structural edit operations | **Reference only** |
| Aether Canvas license/dependencies | Its inspected workspace Cargo metadata declares MIT OR Apache-2.0; the inspected repository tree did not contain a root LICENSE file. Exact source headers, notices and dependency closure were not cleared | **No code port or dependency approval** |

Aether's design demonstrates a useful regular-grid representation, but Forma Deform should not import its rig/document model, generic edit operations, parameter runtime, UI, or dependency graph. Its missing-parameter behavior (pinning an axis to its first key) is not suitable here: Forma Deform must reject an unresolved required input through its existing mapping diagnostic path.

## 3. Proposed model contract

Add a distinct mapping variant, provisionally:

~~~rust
MultiAxisKeyformGrid {
    axes: Vec<KeyformAxis>,
    pose_ids: Vec<String>,
}

struct KeyformAxis {
    input: InputId,
    keys: Vec<f64>,
}
~~~

The exact public type names can be adjusted during implementation, but the semantic contract should remain:

- Each axis names its driver input explicitly. Do not infer axis identity from vector position alone.
- Every axis has at least two finite, strictly increasing key values.
- The axis input IDs are unique and must match the controller's declared input set exactly; no undeclared or unused input is accepted.
- Each axis input must resolve to a finite Scalar. Angle and Vector2 drivers are rejected in this first version; normalized or cyclic parameter semantics can be added as separate, explicit contracts later.
- Every Cartesian key combination has one pose ID. The product of axis key counts must equal pose_ids.len().
- Flattening order is deterministic: axis 0 varies fastest. Document this in the schema and test it with asymmetric values.
- Each pose must resolve in the runtime PoseLibrary, and all poses must have exactly the controller's binding IDs, compatible value kinds, and identical wrapped/unwrapped angle semantics.
- Pose definitions remain runtime-supplied. This mapping does not claim durable pose authoring or persistence.

### Bounded complexity

For the first implementation:
- Require 2–4 axes.
- Require at least 2 keys per axis.
- Require the checked product of key counts to be at most 256 poses.
- Require no more than 16 active corners for one evaluation (2^4).
- Reject overflow and over-limit grids during local validation, before evaluation.

These limits bound both storage and evaluation cost. They are explicit product limits, not silent truncation.

## 4. Interpolation contract

Use **piecewise multilinear interpolation** over the containing cell.

For each axis:
1. Reject missing or non-scalar input.
2. Clamp the value to that axis's first/last key (no extrapolation).
3. Find the adjacent lower/upper keys and local fraction t in [0,1].
4. If the value is exactly on a key, that axis contributes one key index with weight 1; otherwise it contributes two indices with weights 1-t and t.

Take the Cartesian product of the per-axis choices. A corner's weight is the product of its axis weights. The active corner weights are non-negative and sum to one. Resolve those corner pose IDs and blend each binding by identity, never by authored vector position.

Value semantics:
- Scalar: weighted arithmetic sum.
- Vector2: weighted arithmetic sum per component.
- Unwrapped Angle: weighted arithmetic sum in radians, preserving unwrapped semantics.
- Wrapped Angle: weighted circular mean (weighted sine/cosine); reject a near-zero resultant as ambiguous.
- Mixed value kinds or mixed angle semantics: reject.

If all axes land exactly on keys, return the exact authored pose. Zero-weight corners should not be resolved or blended. This preserves exact keyform values and avoids errors from irrelevant adjacent corners.

The result must be deterministic for a fixed controller, pose library and input snapshot. No random triangulation, runtime topology inference, or dependence on map iteration order.

## 5. Why multilinear first; why not smooth/cyclic yet

**Multilinear is the correct first contract.** It is local, deterministic, bounded, convex for scalar/vector components, and straightforward to test. It is the expected baseline for a regular grid.

Do not include Catmull-Rom/Smooth in the first slice:
- Catmull-Rom may use negative weights and overshoot authored values.
- Negative weights do not directly support the wrapped-angle circular-mean contract.
- N-dimensional smooth interpolation needs a defined axis-order and tensor-product policy plus overshoot tests.
- The current typed-value model does not define tangent or monotonicity constraints.

Do not include cyclic axes yet:
- Seam behavior requires an explicit cycle interval and a clear relationship between cycle endpoints and authored keys.
- A cyclic axis with sparse keys must define wrap-segment interpolation and exact seam behavior.
- The current canonical input contract does not carry cyclic metadata.

Do not include key insertion/removal, axis editing, additive blend shapes, sparse grids, triangulation, or RBF interpolation in this evaluator slice. Those are separate authoring/data-model features.

## 6. Alternatives considered

| Alternative | Decision | Reason |
|---|---|---|
| Extend five-pose joystick with inferred diagonal poses | Reject | Invents un-authored data and changes current mapping semantics |
| Keep only five-pose inverse-distance weighting | Retain as a separate option | Useful for sparse cardinal controls, but cannot represent an explicit complete grid |
| Triangulated sparse 2D field | Defer | Requires topology, triangulation stability, boundary behavior and authoring UI |
| RBF interpolation | Defer | More global behavior and numerical/conditioning choices; benchmark only against a concrete need |
| Import Aether KeyformGrid<T> | Reject for now | Its parameter, geometry, editing and generic Blend abstractions are broader than this mapper; exact source-file licensing and dependency closure are not cleared |
| N-dimensional multilinear grid in existing core | **Select** | Reuses current pose/value contracts and keeps implementation small and pure |

## 7. Validation and failure behavior

Reject the mapping before returning contributions if any of the following occurs:
- axis count outside 2–4;
- duplicate or empty axis input IDs;
- axis input set differs from controller input set;
- an axis has fewer than two keys, non-finite keys, duplicate keys, or unsorted keys;
- key-count product overflows or exceeds 256;
- pose count differs from the product;
- empty/duplicate pose IDs, missing runtime poses, or incompatible pose bindings/types;
- missing input, unavailable input, non-scalar input, or non-finite input;
- non-finite/invalid corner weights or output values;
- ambiguous wrapped-angle blend.

Use the existing CoreError::InvalidMapping / non-finite error path so the evaluator emits the established mapping-domain diagnostic and marks outputs unavailable. Do not return partial binding contributions.

## 8. Test and acceptance plan

### Model validation
- 2-, 3-, and 4-axis valid grids.
- Duplicate axes/input IDs; missing/extra input IDs.
- Empty, singleton, unsorted, duplicate, and non-finite key arrays.
- Product overflow and the 256-pose cap.
- Wrong pose count, duplicate/empty IDs, missing pose, binding-set mismatch, type mismatch and angle-semantic mismatch.

### Numerical behavior
- Exact authored corners reproduce exact values.
- Bilinear interpolation of an affine 2D surface.
- Trilinear interpolation with asymmetric corner values to verify flattening order.
- Irregularly spaced keys and different intervals on each axis.
- Clamping below/above every axis range.
- Axis-order independence when axes are reordered with their input IDs and pose data reordered consistently.
- Scalar/vector interpolation remains within corner component bounds.
- Wrapped-angle interpolation across the -pi/pi boundary and ambiguous antipodal rejection.
- Unwrapped-angle interpolation preserves revolutions.
- Zero-weight corners are not required to resolve.
- Determinism and non-finite/overflow edge cases.

### Integration and performance
- Canonical mapper integration test with resolved input snapshots.
- Missing/unavailable input produces the standard mapping diagnostic and no partial outputs.
- Serialization round-trip for the new mapping variant; confirm existing schema-v1 documents still deserialize unchanged.
- Benchmark representative grids (2–4 axes, 2–5 keys per axis, 1–1000 bindings) against the five-pose field and record evaluation cost. Do not optimize before measuring.

## 9. Implementation sequence and exit gates

1. Confirm schema/migration policy for adding a new tagged mapping variant.
2. Add the validated axis model and mapping variant, with backward-compatibility tests.
3. Add a bounded corner-weight enumerator and checked indexing helper.
4. Add typed weighted-pose blending; share with five-pose only if the refactor remains behavior-preserving.
5. Integrate the mapping into the canonical mapper.
6. Add model, numerical, diagnostic, serialization and benchmark coverage.
7. Update the registry from **Approved design** to **Implemented** only after tests pass and all limits are enforced.

**Not included:** AniCraft changes, persistent pose storage, authoring UI, live playback integration, multi-controller composition, smooth interpolation, cyclic axes, or any external dependency.
