# FDM-01.2 — Operator Interface Contract

**Status:** Draft implementation  
**Base:** FDM-01.1 geometry contract integration  
**Scope:** Host-independent single-operator invocation boundary

## Purpose

FDM-01.2 hardens the existing `DeformerOperator` extension point. It does not introduce a second operator trait, geometry payload, evaluator scheduler, or host adapter.

The existing operator interface lives in `composition_evaluation.rs`. This slice adds a validated dispatch function around that trait so callers have one contract-enforcing invocation path.

## Contract layers

- `DeformationDefinition` remains the authored source of deformation kind, target, coordinate space, and typed input/output kinds.
- `GeometryValue` remains the explicit typed intermediate value passed between composition nodes.
- `EvaluationRequest` remains the immutable source/time/model/dependency envelope.
- `GeometryInput` is the borrowed read-only view over the canonical host snapshot.
- `DeformerOperator` remains the algorithm extension point; algorithm-specific parameter and numerical validation remains with the implementation.

## Invocation sequence

`evaluate_operator` performs the following deterministic checks before calling algorithm code:

1. Validate the authored definition.
2. Validate non-empty algorithm identity and non-zero algorithm version.
3. Require the resolved operator kind to match the authored deformation kind.
4. Require the incoming `GeometryValueKind` to match the definition's declared input kind.
5. Resolve the immutable source snapshot for the definition's target from the request.
6. Require the resolved snapshot stamp to equal the definition's authored reference stamp. This prevents evaluating a definition against a different captured geometry revision or fingerprint.
7. Require exact coordinate-space descriptor equality between definition and host snapshot. No unit conversion, axis flip, or origin translation is implicit.
8. Invoke the pure operator implementation.
9. Require the returned `GeometryValueKind` to match the definition's declared output kind.

Contract rejection and algorithm failure are represented separately by `OperatorInvocationError::Contract` and `OperatorInvocationError::Operator`.

## Source versus intermediate geometry

The operator receives the current intermediate `GeometryValue` as its actual input. The context separately exposes the immutable root snapshot through `source_geometry(target)`. These are intentionally distinct: a later sequential graph node may consume an upstream result without replacing or mutating the original host snapshot.

This slice verifies the definition against the immutable snapshot captured in the request. It does not compare that captured snapshot against a second, current host snapshot; that live freshness check remains part of the evaluator lifecycle in FDM-01.4. Nor does it define graph scheduling or parallel execution.

## Determinism and purity

Operators must produce equivalent results for identical definition, input, request, and algorithm version. They must not read or mutate host documents, query a global playhead, schedule work, mutate the authored definition, or depend on hidden mutable caches.

The dispatch wrapper validates the shared interface contract; each operator remains responsible for its own parameter schema, numerical stability, resource limits, and algorithm-specific topology constraints.

## Tests

The focused tests cover:
- successful invocation with matching definition, source snapshot, coordinate space, and input/output kinds;
- rejection of an input-kind mismatch;
- rejection of an authored source-stamp mismatch;
- rejection of a coordinate-space mismatch;
- rejection of an operator-kind mismatch;
- rejection of an output-kind mismatch;
- rejection of invalid algorithm metadata.

## Non-goals

- no first deformation algorithm (FDM-01.3);
- no graph scheduler or composition evaluator;
- no host scheduling, cache lifecycle, result application, undo/redo, or baking;
- no implicit geometry conversion or curve sampling;
- no changes to AniCraft;
- no Smart Controller/Form Deform composition changes; CCC-01.5 remains in force;
- no replacement for AniCraft native Asset Warp.

## Verification

CI is required for formatting, Rust workspace tests, existing TypeScript checks/tests/build, and all release WASM plugin builds. This document does not claim verification before those checks complete.
