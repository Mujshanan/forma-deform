# FDM-05.4 — Integration Readiness & Output Contract Hardening

**Status:** implementation and CI verification passing; integration contract review in progress  
**Parent:** FDM-05.3 constructor regression branch, exact-head CI passed at `ea5bf9a0cc85cc11bb7a9148d449d547ce40a578`  
**Scope:** audit and harden existing FDM-05 output contracts; no new deformation algorithm.

## 1. Objective

Establish that the authoritative structured `Path2D` result and its derived output-space curve samples form one internally consistent, deterministic, provenance-checked result that a native AniCraft host can consume safely.

FDM-05.4 is not an algorithm expansion. The existing binding evaluation, cage mapping, and three-probe output-space refinement semantics remain unchanged.

## 2. Audit invariants

- The structured evaluated `Path2D` remains authoritative and preserves authored command kinds, order, subpaths, and Bézier handles.
- Each quadratic/cubic command has exactly one derived refined segment in original command/subpath order.
- Refined source endpoints agree with the immutable source snapshot, and refined output endpoints agree with the corresponding evaluated structured-curve endpoints within a scale-relative floating-point comparison tolerance.
- Sample parameters are finite, endpoint-inclusive, and strictly increasing.
- Source snapshot, binding stamp, rest-cage fingerprint, current-cage fingerprint, topology identity/revision, and outside-cage policy are mutually consistent.
- Evaluation/sample counts agree with actual samples and do not exceed declared policy budgets.
- Repeated evaluation with identical immutable inputs and policy is deterministic.
- Every evaluation starts from the immutable original source snapshot; no output is fed back as a new source implicitly.
- Probe deviation remains an estimate, not a certified global maximum-error bound.

## 3. Hardening work

### FDM-05.4.1 — Constructor/output alignment
Validate refined first/last source samples against the immutable source snapshot and output samples against the authoritative structured path. Reject correctly ordered samples disconnected from either geometry.

### FDM-05.4.2 — Metadata and budget consistency
Validate that evaluation counts are at least the emitted sample count and do not exceed the refinement policy's maximum evaluation budget. Retain checked arithmetic for sample totals.

### FDM-05.4.3 — Adversarial regression
Exercise endpoint mismatch, invalid evaluation accounting, mixed commands, multiple subpaths, stale provenance, outside-policy mismatch, and deterministic repeat evaluation. Add further tests only for confirmed gaps in existing behavior.

### FDM-05.4.4 — Native host integration contract
Document the output as a core-owned immutable value. AniCraft owns scheduling, lifecycle, caching, persistence, UI, rendering, undo/redo, apply, and bake. The host must not flatten or mutate the authoritative Path2D as a side effect of consuming derived samples. Define cache invalidation, recomputation, failure handling, and separation from Smart Controller composition.

## 4. Explicit exclusions

- No new deformation operator, interpolation, or cage-mapping algorithm.
- No change to the existing output-space refinement method or its accuracy semantics.
- No bone deformation, warp, or Smart Controller/FDM composition.
- No rendering, UI, or AniCraft host implementation in the core.
- No adapter or SDK retirement in this milestone.
- No PR merges without explicit authorization.

## 5. Completion criteria

1. Confirmed output-contract defects are fixed with focused regression tests.
2. Existing FDM-05.1–05.3 semantics remain unchanged.
3. Structured/derived endpoint and identity checks pass.
4. Provenance, policy, determinism, and resource-budget tests pass.
5. Native host responsibilities and core guarantees are documented.
6. Exact final-head CI passes `cargo test --workspace`, benchmark smoke test, and release builds for Bone Query, Form Deform, and Smart Controller WASM targets.
7. PR lineage remains explicit and no stacked PR is merged without approval.

## Verification record

The hardening branch currently passes CI on exact head `3e90ffa89e2dd5e14c84e6357b5befd462ad4b27` (workflow run [#45167359427](https://github.com/Mujshanan/forma-deform/actions/runs/45167359427)):

- `cargo test --workspace`: passed, including 143 `form-deform-core` tests.
- Curve-sampling benchmark smoke test: passed (20 measured iterations per workload).
- Bone Query WASM release build: passed.
- Form Deform WASM release build: passed.
- Smart Controller WASM release build: passed.

The verification run covers the source/output endpoint checks, command-structure parity, invalid metadata/budget rejection, and deterministic repeated assembly tests. This is not yet a claim that all FDM-05.4 audit work is complete; native host contract review remains in progress.
