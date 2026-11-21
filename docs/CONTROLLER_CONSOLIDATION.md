# Controller Consolidation — CCC-01.4

- **Milestone:** CCC-01.4 — Controller Consolidation
- **Status:** Implemented; CI passed; pending review
- **Baseline:** CCC-01.3 shared-primitives branch
- **Compatibility contract:** `docs/CODE_COMPATIBILITY.md` (CCC-COMP-001)

## 1. Objective

Establish one authoritative implementation for the persisted three-anchor controller-binding model and evaluator, while retaining existing public import paths and keeping the separate plugin-preview configuration contract intact.

## 2. Canonical ownership

The implementation now lives in `smart_controller_core::binding`:

- `InputRange` and its asymmetric neutral-aware normalization.
- `EaseCurve` and its Linear/Smoothstep easing.
- `ControllerMode` and its AbsolutePose/RelativeDelta semantics.
- `ControllerTarget` and its SymbolFrame/GeometryDeform variants.
- `ControllerBinding` validation and deterministic evaluation.
- `PreviewResult` output types.
- `BindingSet`, schema-version validation, stable-ID lookup/CRUD, and JSON serialization.
- `validate_bindings` and the binding-specific error contract.

There is no second binding evaluator. The original implementation was moved without changing its algorithm or serialized field definitions.

## 3. Compatibility facade

The following paths expose the same underlying public Rust types:

- Canonical: `smart_controller_core::binding::ControllerBinding`
- Existing module path: `smart_controller_core::legacy::ControllerBinding`
- Existing root path: `smart_controller_core::ControllerBinding`

The same applies to the other binding types, `BindingSet`, errors, helpers, and `Vec2`.

`legacy` is now a compatibility re-export only. It contains no independent model, evaluator, state, or serialization implementation.

## 4. Separate plugin configuration and preview contract

The `Params`, `Mode`, `EndpointPolicy`, `mapping`, `preview`, and `validation` modules remain separate.

They represent the Smart Controller plugin's command/configuration and on-demand preview workflow. They are not interchangeable with the persisted three-anchor binding-set schema:

- `Params` describes one plugin invocation/configuration, including optional host layer/bone selectors and Slider/Transform settings.
- The binding model represents stable-ID bindings with three target anchors, easing, enablement, and versioned collection serialization.
- Their normalization semantics differ: `InputRange` maps a three-point range with an explicit neutral value to [-1, 1], while rotation normalization maps two possibly reversed endpoints to [0, 1].
- Frame mapping and Transform preview are not binding-set evaluation and must not be silently folded into it.

Keeping these contracts separate is intentional, not a second implementation of the same binding API. Whether plugin `Params` should later become an adapter-only projection of the binding model remains a separate architecture decision.

## 5. Compatibility and regression coverage

- All existing binding unit tests moved with the implementation and remain in the canonical module.
- A public integration test proves canonical, legacy, and root import paths are the same Rust types.
- The integration test exercises version-1 binding-set JSON round-trip and representative evaluation through the canonical path.
- No schema version, field name, identity representation, easing, interpolation, clamping, disabled behavior, CRUD behavior, or error variant was intentionally changed.
- No plugin adapter, AniCraft SDK, or ABI code was changed.

## 6. Deferred

- Mapping controller output into deformation evaluation (CCC-01.5).
- Deciding whether plugin command `Params` becomes a projection of the persistent binding model.
- Additive/multi-source composition semantics.
- Binding identity newtypes and host identity conversion.
- Runtime scheduling, playback, persistence in AniCraft, and FDM v2 document ownership.

## 7. Verification results

GitHub Actions pull-request run [44898942033](https://github.com/Mujshanan/forma-deform/actions/runs/44898942033) completed successfully on the implementation commit. It passed TypeScript typecheck/tests/build, `cargo test --workspace`, and release WASM builds for Bone Query, Form Deform, and Smart Controller.

The workflow does not run `cargo fmt --all -- --check`; formatting was not independently verified by CI.

## 8. Verification required

- Full Rust workspace tests.
- Existing TypeScript checks/tests/build.
- All three release WASM plugin builds.
- Public API and schema-v1 regression tests.
- Review of the PR diff to ensure the compatibility module is re-export-only and no implementation was duplicated.
