# FDM-05 — Deformation Output Integration

**Status:** implementation plan; no host/UI scope is authorized by this phase  
**Base:** FDM-04.5 merge `484fc2127340568b555db2ff7364da1a7aa7bb21`  
**Architecture:** FDM v2, ADM-001/007; CCC-01.5 remains in force

## 1. Purpose

FDM-05 connects the two existing FDM-04 evaluation products into one deterministic deformation-output boundary:

1. the command-preserving evaluated `Path2D` produced by `evaluate_binding`; and
2. the evaluated adaptive curve samples produced by `evaluate_curve_samples`.

The result must retain editable Path2D structure while exposing a curve-accurate derived representation for downstream consumers. The derived representation must never replace, flatten, or mutate the authoritative Path2D.

This phase is FDM-core only. It does not implement AniCraft result application, rendering, UI, scheduling, cache lifecycle, persistence of evaluated output, undo/redo, apply, or bake. Those remain host-owned FDM-06 work.

## 2. Current baseline and gap

FDM-04 already provides:

- `evaluate_binding`: maps MoveTo/LineTo endpoints and quadratic/cubic control points, rebuilding the same Path2D command sequence and segment kinds.
- `evaluate_curve_samples`: evaluates the stamped adaptive samples against a current certified cage pose, preserving command index, subpath index, sample index, and parameter `t`.
- deterministic fingerprints and source/cage provenance on each independent result.

The missing piece is a single output contract that validates and groups these related products. In addition, source-space adaptive sampling alone does not guarantee a requested deviation after a non-uniform or piecewise cage deformation. FDM-05 must measure approximation error in evaluated/output space before claiming a curve-accuracy guarantee.

## 3. Accepted output model

The core output is a paired, immutable value:

- **Structured path:** the evaluated Path2D, retaining original command count, command order, subpath structure, and MoveTo/LineTo/QuadraticTo/CubicTo/Close kinds. Bézier control handles remain explicit Path2D control points; they are not discarded into a polyline.
- **Derived curve geometry:** evaluated samples grouped by original command and subpath, retaining stable `t` parameters and deterministic order. This representation is derived and disposable.
- **Provenance:** exact source snapshot stamp, rest-cage fingerprint, current-cage fingerprint, curve policy/version, and canonical structured-path fingerprint.
- **Diagnostics:** counts for mapped, clamped, extrapolated, and unbound source features and curve samples, plus sample-budget/accuracy information.

The output must be constructed only from mutually consistent results. A mismatch in source stamp, complete validated binding stamp (including mapping/outside-cage policy), rest cage, current cage, policy/version, command/subpath identity, or sample sequence fails closed.

## 4. Curve-accuracy semantics

The source-space sampling tolerance in FDM-04 is not automatically an output-space tolerance. FDM-05 must not label a deformed polyline as meeting the configured tolerance unless the error was checked after deformation.

FDM-05.2 will add deterministic output-space adaptive refinement:

1. Evaluate the source Bézier segment at a parameter `t` using its original Path2D command.
2. Resolve that source point against the validated rest-cage mapping policy.
3. Evaluate the corresponding point on the certified current cage.
4. Compare deterministic quarter-, midpoint-, and three-quarter-parameter probes against the evaluated output chord. The v1 result is a sampled deviation estimate, not a certified upper bound.
5. Subdivide deterministically until the output-space tolerance is met or a depth/sample budget is exhausted.
6. On budget exhaustion, return an explicit error; never silently claim the requested tolerance.

The algorithm must handle cage-face crossings, outside-cage policy, degenerate chords, large finite coordinates, and deterministic tie-breaking. The precise error bound and whether the first implementation uses a conservative control-polygon bound or a documented sampled bound must be settled in the FDM-05.2 design/tests before exposing a hard accuracy guarantee.

## 5. Invariants

- No mutation of source snapshots, validated bindings, topology, cage state, or prior outputs.
- No evaluation from a previously deformed output; every result is derived from the original immutable source snapshot.
- Preserve Path2D command order and command kinds exactly.
- Preserve all Bézier handles as explicit structured-path control points.
- Derived samples never become authoritative authored geometry.
- Identical validated inputs and policy produce byte-for-byte equivalent logical output and fingerprints.
- Stale or inconsistent source/rest/current cage provenance fails closed.
- No implicit coordinate conversion.
- No implicit Smart Controller + pin-deformation composition; CCC-01.5 remains unchanged.
- No AniCraft host calls or guessed host APIs.

## 6. Proposed implementation slices

### FDM-05.1 — Paired Output Contract
- Define an immutable output value that joins the structured Path2D result and evaluated curve result.
- Validate exact source stamp, rest/current cage fingerprints, policy/version, and curve command/subpath identity.
- Expose read-only accessors and deterministic output fingerprinting.
- Add mismatch and non-mutation tests.
- Do not claim output-space curve tolerance yet.

### FDM-05.2 — Output-Space Adaptive Refinement
- Evaluate original Bézier points at stable parameters through the validated rest-cage/current-cage mapping.
- Refine in deterministic left-to-right order using output-space probes at 1/4, 1/2, and 3/4 of each candidate interval.
- The current v1 metric is explicitly a **three-probe deviation estimate**, not a certified global maximum-error bound. It is not safe to label it a strict geometric tolerance.
- Enforce explicit depth, per-curve sample, total sample, and total evaluation budgets; any exhausted budget fails closed.
- Use the same point locator and outside-cage policies as FDM-04. Outside-policy behavior is evaluated at every probe.
- Face-crossing behavior is observed through the output-space probes; the finite probe set cannot prove that an arbitrarily narrow crossing was detected. This limitation is part of the v1 contract.
- Regression coverage crosses the shared edge of two cage triangles under a warped current pose, checks all three outside-cage policies and the bounded-extrapolation distance failure, and exercises invalid tolerances, degenerate chords, and extreme finite coordinates.
- A future certified bound requires a separate proof/algorithm (e.g. face-aware subdivision and conservative mapped control-hull bounds); it is not claimed by this milestone.

### FDM-05.3 — Structured/Derived Output Assembly
- Assemble each original curve command with its corresponding refined derived samples.
- Validate complete, ordered coverage across multiple subpaths and mixed line/quadratic/cubic/close commands.
- Keep the original Path2D structure intact and independently inspectable.

### FDM-05.4 — Numerical and Adversarial Regression
- Add reference-curve deviation tests, nonlinear cage deformations, discontinuity/face-crossing cases, determinism, stale provenance, and no-accumulation tests. FDM-05.2 currently covers triangle-edge crossings, all outside policies, extrapolation limits, invalid policy values, degenerate chords, and overflow.
- Strengthen finite-value and resource-budget failure coverage.

### FDM-05.5 — Performance Baseline and Quality Gates
- Add representative output-integration workloads and benchmark reporting.
- Run workspace tests, benchmark smoke tests, and all three release WASM builds.
- Audit branch lineage and CI evidence before opening the final review PR.

## 7. FDM-05.2 v1 accuracy contract

`OutputRefinementPolicy.max_deviation` is compared with the maximum distance of three deterministic interior probes (quarter, midpoint, three-quarter) from the evaluated endpoint chord. `RefinedCurveSampleSet.max_observed_deviation` records the largest accepted-interval probe deviation. Both are estimates over the probes actually evaluated; neither is a mathematical upper bound on the entire deformed curve. The implementation must not expose this value as a certified tolerance or use wording such as “guaranteed maximum deviation.”

The algorithm returns an error on subdivision-depth, sample, or evaluation-budget exhaustion. It never returns a partial result as if it met the policy. It evaluates every probe from the original immutable Path2D snapshot, never from a prior refined output. The output-space mapping uses the validated binding's outside policy and certified rest/current cage geometry.

## 8. Explicit exclusions

- AniCraft UI or 360 Studio integration.
- AniCraft host result application or native geometry mutation.
- Scheduler, cache, persistence, undo/redo, apply, or bake.
- Smart Controller/FDM result composition.
- Changes to native Asset Warp or the retired Bun/TypeScript demo.
- Flattening or replacing the source Path2D with a polyline.
- Merging any PR without explicit user authorization.

## 9. Completion criteria

FDM-05 is complete only when the paired output contract, output-space accuracy behavior, structured/derived assembly, numerical regressions, performance baseline, and CI gates are all verified on one clean branch descended from the merged FDM-04.5 base. Passing source-space curve sampling tests alone is insufficient.
