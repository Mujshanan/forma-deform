# CQ2 canonical-core decomposition proposal

**Status:** Proposal only; no application source changes.  
**Base:** `975045cbcd43bd9b1e2195041d06a057d514721d`  
**Branch:** `quality/cq2-decomposition-proposal`  
**Scope:** Three canonical Smart Controller core hotspots identified by manual review and rustqual 1.8.3.

## 1. Goals and non-goals

### Goals
- Make validation, evaluation, and weighted composition easier to read, reason about, and test.
- Expose existing semantic phases through private, intention-revealing functions.
- Reduce the size and cognitive load of the three hot functions without changing their contracts.
- Preserve deterministic evaluation and composition behavior, diagnostics, and compatibility.

### Non-goals
- No new feature or algorithm.
- No public API, serialized schema, diagnostic code, or error type changes.
- No change to host integration, plugin lifecycle, or AniCraft ownership.
- No cleanup of legacy exports, tests, or `expect` calls merely to satisfy a metric.
- No simultaneous formatting changes. This work starts only after the standalone rustfmt patch is reviewed and applied.

## 2. Recommended implementation order

Implement and review each item as an independent change. Prefer three separate PRs (or, at minimum, three independently revertible commits). Run focused tests after each and the full workspace/WASM suite before integration.

1. **D1 — Controller local validation decomposition** (`canonical.rs`): smallest semantic surface and a good first proof that the extraction pattern preserves error precedence.
2. **D2 — Weighted blend decomposition** (`composition.rs`): isolate numerical policies while keeping the composition dispatcher and other modes unchanged.
3. **D3 — DAG evaluation decomposition** (`evaluator.rs`): largest and most behavior-sensitive extraction; do this only after D1/D2 establish the review/test pattern.

Do not combine these three into a single large refactor.

## 3. D1 — `Controller::validate_local`

**Current source:** `crates/smart-controller-core/src/canonical.rs:76-209` (method begins at line 78).  
**Analyzer signal:** 103 lines; cognitive complexity 51; cyclomatic complexity 86.

### Proposed private phases

Keep the public method as a small, ordered coordinator:

1. `validate_controller_references`
   - non-empty inputs
   - no self-dependency
   - unique dependency IDs
   - unique input IDs
2. `validate_bindings`
   - unique binding IDs
   - binding mode/base-value relationship
   - explicit composition configuration validity
   - target/mode compatibility
   - target identity and numeric constraints
3. `validate_mapping_config`
   - Range1D and ThreeAnchorRange finite/order checks
   - DirectedRange1D finite/distinct checks
   - PoseSet1D and JoystickFivePose identity/cardinality checks
   - Discrete non-empty check

A private helper for composition configuration may be introduced if it makes the Replace/Additive/WeightedBlend/PriorityOverride policy clearer. Avoid a generic validation framework.

### Critical behavior-preservation rule

Preserve the exact current validation order, including which error wins when a model contains multiple invalid fields:
- controller-level reference checks first;
- binding iteration in authored vector order;
- per-binding duplicate ID, mode/base constraints, composition checks, then target checks in their current order;
- mapping configuration validation last.

Preserve every existing `CoreError` variant and message, including formatting and punctuation. Do not reorder checks just because another grouping seems cleaner.

### Tests
- Retain all existing canonical/controller validation tests unchanged.
- Add table-driven cases with two simultaneous invalid conditions to lock down first-error precedence.
- Cover each binding composition mode and invalid mode/base/weight/base-reference combination.
- Cover each target variant and each mapping variant, including non-finite values and duplicate/empty identities.
- Assert exact error variant and message where current tests or API consumers rely on it.

### Acceptance
- Public signature remains `pub fn validate_local(&self) -> Result<(), CoreError>`.
- No serialized output or schema change.
- Existing error precedence and messages remain byte-for-byte equivalent.
- No change to validation coverage or accepted/rejected model set.

## 4. D2 — `compose_weighted_blend`

**Current source:** `crates/smart-controller-core/src/composition.rs:141-236` (function begins at line 141).  
**Analyzer signal:** cognitive complexity 24; cyclomatic complexity 16; 88 lines; seven `expect` sites flagged.

### Proposed private phases

Keep `compose_weighted_blend` as an orchestration function:

1. `eligible_weighted_contributions`
   - exclude zero-weight contributions before compatibility and arithmetic checks;
   - preserve input ordering (the caller already canonical-sorts contributions).
2. `sum_eligible_weights`
   - accumulate in the same iteration order;
   - preserve overflow/non-finite error binding selection and exact error variant.
3. `validate_blend_compatibility`
   - preserve scalar/vector/angle type checks and wrapped-angle semantics.
4. Dispatch to focused numeric helpers:
   - `blend_scalar`
   - `blend_vector2`
   - `blend_angle` (with wrapped circular mean and unwrapped arithmetic mean as separate named paths if helpful).

Keep zero-eligible behavior in the orchestration path: add the target to `unavailable_targets`, emit `COMPOSITION_NO_ELIGIBLE`, and return `Ok(None)`.

### Numerical invariants that must not change

- Zero weights are excluded before type compatibility checks.
- Eligible contribution order remains canonical `(BindingId, ControllerId)` order.
- Weight accumulation and normalized-weight calculations use the same `f64` operation order.
- Scalar and vector accumulation order is unchanged.
- Wrapped angles use the existing sine/cosine circular mean and `1e-12` opposing-direction threshold.
- Wrapped output uses the existing normalization function.
- Unwrapped angles retain the existing arithmetic weighted mean.
- Preserve exact typed errors for invalid total weights and composition conflicts.
- Do not replace internal `expect` calls with silent defaults or swallowed errors. Any change to panic/error behavior requires a separate explicit decision and tests; it is not part of this decomposition.

### Tests
Retain and run the existing composition tests, especially:
- `weighted_blend_normalizes_positive_weights`
- `zero_weights_are_excluded_and_all_zero_produces_diagnostic_without_output`
- `invalid_negative_or_non_finite_weights_are_typed_errors`
- `weighted_blend_rejects_incompatible_types_and_undefined_wrapped_angle_mean`
- `output_is_independent_of_input_contribution_order`
- `evaluate_and_compose_preserves_angle_typed_property_values`

Add scalar/vector/angle multi-contribution cases that compare exact existing results and verify deterministic ordering. Do not change tolerances or numerical thresholds.

### Acceptance
- `compose` and `CompositionResult` public behavior unchanged.
- No change to target grouping, sorting, diagnostics, unavailable targets, or output values.
- Existing tests pass without weakening assertions.
- No host access or mutation introduced.

## 5. D3 — DAG `evaluate`

**Current source:** `crates/smart-controller-core/src/evaluator.rs:109-381` (function begins at line 111).  
**Analyzer signal:** 306 lines; cognitive complexity 88; cyclomatic complexity 36; nesting 5.

### Proposed internal model

Keep the public `evaluate(graph, definitions, snapshot, mapper)` signature and return type. Split its existing documented stages without changing their order.

Introduce a private preflight value, for example:

```rust
struct EvaluationPreflight {
    inputs: BTreeMap<InputId, PreflightInput>,
    diagnostics: Vec<Diagnostic>,
}
```

Then extract:

1. `prepare_preflight(graph, definitions, snapshot)`
   - graph and snapshot validation;
   - controller local validation;
   - input-definition validation and duplicate-ID detection;
   - required-input reference checks;
   - supplied-sample validation and rejection of externally supplied controller-output samples;
   - controller-output producer/dependency/binding-reference checks;
   - active-input selection;
   - external input resolution and preflight diagnostics.
2. `evaluate_in_topological_order(graph, preflight, mapper)`
   - obtain the topological order at the same point as today;
   - retain evaluator-side cycle defence;
   - create the same output/result accumulators.
3. Per-controller phases, preferably backed by a small private mutable evaluation state:
   - `resolve_controller_inputs`: resolve external/controller-output inputs and fallback policy, validate dimensions, emit current diagnostics;
   - `evaluate_controller_mapping`: invoke mapper, classify recoverable mapping failures exactly as today, propagate other errors;
   - `collect_controller_contributions`: reject unknown mapper binding IDs, mark omitted/incompatible outputs unavailable, construct and validate contributions.
4. Preserve final contribution sorting in one explicit finalization step.

Do not split merely to reduce line count. Each helper should represent a real stage in the evaluator's documented contract. Avoid a helper with a long list of mutable references; a narrowly scoped private state struct is preferable if it makes ownership clearer.

### Critical behavior-preservation rules

- Preflight remains complete before the first mapper call.
- Preserve exact order of graph/snapshot/definition/reference/sample validation.
- Preserve the distinction between missing, invalid, stale, unavailable, and dimension-mismatched input states.
- Preserve fallback behavior for external and controller-output inputs.
- Preserve dependency ordering and evaluator-side topological validation.
- Preserve disabled-controller diagnostics and unavailable-output marking.
- Preserve mapping failure classification: only the same errors are recoverable; other errors still abort.
- Preserve unknown mapper binding rejection.
- Preserve target-type rejection, skipped-output diagnostics, and contribution metadata.
- Preserve output map key semantics and final stable sort order.
- No target composition or host mutation moves into Phase 3.

### Tests
Retain the full evaluator suite. In particular, keep coverage for:
- mapper is not called when any preflight check fails;
- undeclared or duplicate inputs and invalid external samples;
- controller-output input cannot be injected through the external snapshot;
- missing producer, undeclared dependency, and missing producer binding;
- disabled controllers and unavailable outputs;
- every fallback policy and controller-output fallback;
- dimension mismatch and recoverable mapping failure vs fatal mapper error;
- unknown mapper binding IDs;
- deterministic topological evaluation and final contribution ordering.

Add focused tests at helper boundaries only where they test an observable contract, not implementation details. Do not make private helper names part of public tests.

### Acceptance
- Public function and trait signatures unchanged.
- Mapper invocation count/order unchanged.
- Same preflight-before-mapping guarantee.
- Same diagnostics (codes, severities, messages, ordering), contributions, unavailable outputs, and error precedence for the same inputs.
- Existing evaluator tests pass unchanged.

## 6. Cross-cutting verification and review protocol

For each D1/D2/D3 change independently:

1. Review the diff against the approved helper map before implementation.
2. Confirm the patch contains no unrelated rustfmt, import, naming, or documentation cleanup.
3. Run focused unit/integration tests for the touched behavior.
4. Run `cargo test -p smart-controller-core`.
5. Run `cargo test --workspace`.
6. Run all three release WASM builds using the repository's established CI commands.
7. Run `cargo fmt --all -- --check` after the separate formatting-only patch has landed.
8. Compare diagnostics/error snapshots and deterministic outputs against the pre-refactor baseline.
9. Record test evidence and review each refactor as its own commit/PR.

## 7. Explicit stop conditions

Stop and request design review if implementation appears to require:
- changing a public type/signature or serialized field;
- changing validation order or first-error precedence;
- changing float operation order, angle thresholds, or canonical sorting;
- changing diagnostic codes/messages/order;
- changing mapping error classification or fallback behavior;
- changing host integration or ownership boundaries;
- deleting legacy compatibility code or changing tests to weaken assertions.

## 8. Current decision

**Proposal ready for review; implementation not started.**

The formatting-only review is tracked independently on `quality/cq2-formatting-patch`. Do not cherry-pick formatting and decomposition together. Once the rustfmt patch is inspected, applied, and verified, D1, D2, and D3 can proceed as three isolated behavior-preserving changes.
