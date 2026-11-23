# FDM-01.1 — Geometry Contract Integration

**Status:** Initial implementation on `feat/fdm-01-1-geometry-contract-integration`  
**Scope:** Forma Deform core only. No AniCraft host changes.

## Decision

FDM-01.1 reuses the existing M1 geometry and evaluation contracts. It does not introduce parallel identity, snapshot, path, fingerprint, revision, or evaluated-result types.

| Responsibility | Canonical owner |
|---|---|
| Host target identity and generation | `HostTargetRef` in `host_contract.rs` |
| Geometry/topology revisions and immutable source snapshot | `GeometrySnapshot` in `host_contract.rs` |
| Path commands and canonical geometry fingerprint | `Path2D` in `path2d.rs` |
| Request and immutable evaluated output | `EvaluationRequest` and `EvaluatedGeometryResult` in `evaluation_contract.rs` |
| Operator contract and runtime geometry value | `DeformerOperator` and `GeometryValue` in `composition_evaluation.rs` |
| Convenient borrowed view for geometry operators | `GeometryInput` in `geometry.rs` |

## Borrowed geometry input

`GeometryInput<'a>` is a thin, read-only view over a validated `GeometrySnapshot`.

- It borrows the snapshot; it does not clone or rebuild the `Path2D`.
- It exposes the canonical path, target identity, coordinate descriptor, and snapshot stamp.
- The source snapshot remains the authority for all revisions and fingerprints.
- It cannot mutate source commands or outlive the snapshot.

FDM outputs continue to use `EvaluatedGeometryResult`. No second output wrapper is introduced in this slice.

## Coordinate-space compatibility

`require_compatible_coordinate_spaces(expected, actual)` validates both descriptors and then requires exact descriptor equality.

FDM-01 does not silently convert:
- linear units or unit scales;
- axis orientation;
- coordinate origin;
- coordinate-space identity or kind.

A mismatch returns a structured `GeometryContractError::CoordinateSpaceMismatch`. Any future conversion must be an explicit operation with its own semantics and tests; it must not be hidden inside an operator.

## Geometry fidelity boundary for FDM-01.3

The source `Path2D` remains the output container. Operators must preserve command order, subpath structure, segment kinds, and close commands.

For affine mappings, transforming Bézier control points is exact. For nonlinear fields such as radial smooth falloff, transforming only stored anchors and control handles is generally an approximation of the pointwise-transformed curve. FDM-01.3 must document and test this control-point approximation; it must not silently tessellate or replace curves with polylines.

## Diagnostics and evaluator lifecycle

FDM-01.1 does not introduce an independent diagnostic framework. FDM-01.4 will define structured evaluator diagnostics and a deterministic ordering key. The required ordering is stable across runs (including rule/code and element/command index); human-readable message text must not determine diagnostic identity.

Before operator math, the evaluator must:
1. validate the request and source snapshot envelopes;
2. validate coordinate-space compatibility for all operator inputs;
3. compare captured source stamps with current host snapshots when current snapshots are supplied;
4. execute the pure operator;
5. validate output geometry;
6. calculate the canonical output fingerprint;
7. construct the existing immutable evaluated-result contract.

Host-side atomic freshness validation at apply time remains the host's responsibility.

## Tests added in FDM-01.1

- borrowed view exposes the same canonical path and snapshot stamp;
- cubic path commands remain present in the input view;
- exact coordinate descriptor matches are accepted;
- different units, axes, and origins are rejected;
- invalid coordinate descriptors are rejected before compatibility comparison.

## Deliberate non-goals

- no geometry execution engine or deformation operator implementation;
- no sampled mesh/buffer representation;
- no curve tessellation or implicit approximation policy;
- no host scheduling, caching, persistence, UI, apply, undo/redo, or bake work;
- no Smart Controller/Form Deform composition changes (CCC-01.5 remains in force);
- no replacement for AniCraft's native Asset Warp.

## Verification status

This branch has not yet been run through Rust formatting, Clippy, workspace tests, or CI. Do not treat the implementation as verified until those checks pass.
