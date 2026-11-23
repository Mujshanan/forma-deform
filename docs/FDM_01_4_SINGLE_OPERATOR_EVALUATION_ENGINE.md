# FDM-01.4 — Single-Operator Evaluation Engine

**Status:** Initial implementation on `feat/fdm-01-4-single-operator-evaluation-engine`  
**Base:** Verified FDM-01.3 operator implementation  
**Scope:** Host-independent Forma Deform core only

## Purpose

Provide a deterministic orchestration boundary for one operator invocation. The engine compares the immutable request captured for evaluation with a caller-supplied current-state request, invokes the operator through `evaluate_operator`, and creates an immutable `EvaluatedGeometryResult`.

This is not a graph scheduler, host adapter, document transaction, cache, or result applier.

## Captured and current state

The caller supplies two validated `EvaluationRequest` values:

- **Captured request:** the exact model, time sample, dependencies, stage, composition graph and source snapshots against which the result is authored.
- **Current request:** a fresh host snapshot assembled immediately before evaluation. Its request ID may differ; all meaningful evaluation state must otherwise match.

The engine checks:
- model revision and algorithm/schema stamp;
- timebase, sample token, frame, subframe, revision and sample kind;
- dependency stamps;
- stage identity and optional composition graph stamp;
- the complete source set, including target generation, geometry revision, topology revision and content fingerprint.

A stale or inconsistent state fails before operator execution. The engine does not attempt to repair, convert, or silently refresh the captured request.

## Source freshness classification

Each captured source is matched by stable host identity (namespace, document, target kind and target ID), then compared using the existing `SnapshotFreshness` contract.

The engine distinguishes:
- missing or unexpected source snapshots;
- target generation changes;
- geometry/topology revision regression;
- ordinary source changes;
- fingerprint changes with unchanged geometry and topology revisions, treated as a host snapshot-integrity error.

Source diagnostics are sorted by stable diagnostic code and target identity. Ordering does not depend on the order in which the host supplies current snapshots.

## Operator and result flow

1. Validate captured and current requests.
2. Compare model, time, dependencies, stage, composition graph and every source snapshot.
3. Construct the shared read-only `OperatorContext`.
4. Invoke `evaluate_operator`, preserving its definition, operator identity, source-stamp, coordinate-space and value-kind checks.
5. Construct `EvaluatedGeometryResult`, which computes and validates the canonical output fingerprint and copies the captured request's dependency stamps into the immutable result.

The caller supplies a result ID and an explicit `GeometryOutputOperation`. Replacement and independent deformation contribution remain distinct; the engine never combines contributions.

## Diagnostics

Failures return an `EvaluationFailure` containing structured diagnostics, not unstable display strings. Each diagnostic has a stable `EvaluationDiagnosticCode` and may include a target plus the underlying contract, operator or context error.

The diagnostic vector is deterministically ordered. Human-readable and localized messages belong to the host/UI layer.

## Host boundary and freshness at application

The engine receives snapshots; it does not query the host or keep a hidden live cache. Therefore it cannot guarantee that the host remains unchanged after the supplied current-state request was assembled.

Before applying a result, the host must:
- revalidate the result against its captured request;
- compare all captured source stamps against the host's then-current snapshots;
- validate non-geometry dependencies in the host's transaction;
- apply the result atomically with its own undo/redo and document-state rules.

`EvaluatedGeometryResult::validate_source_freshness` remains available for the host's final source check. This engine-level preflight is not a substitute for atomic host-side validation.

## Tests

The suite covers successful result creation and fingerprint validation, stale source rejection before execution, same-revision fingerprint-integrity errors, target-generation changes, revision regression, missing/unexpected source detection, simultaneous model/time/dependency/stage mismatches, composition-graph mismatch, operator parameter failures, invalid result identity, and deterministic diagnostic ordering independent of current-source input order.

## Non-goals

- no AniCraft source changes or host adapter;
- no graph traversal, multi-node scheduling, parallel execution or cache;
- no host document mutation, result application, undo/redo or baking;
- no automatic refresh or retry of stale requests;
- no Smart Controller integration or changes to CCC-01.5;
- no composition of independent deformation contributions.

## Verification

The final branch head must pass FDM-core rustfmt, Clippy (with only the existing `host_contract.rs` `too_many_arguments` lint explicitly exempted), Rust workspace tests, TypeScript typecheck/tests/build, and all three release WASM plugin builds.
