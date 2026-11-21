# Smart Controllers — Phase 0 Existing Implementation Audit

**Repository:** `Mujshanan/forma-deform`  
**Audit branch:** `docs/unified-smart-controllers-plan`  
**Audit scope:** Existing Smart Controller core, plugin adapter, AniCraft SDK boundary, tests, documentation and Form Deform separation  
**Audit type:** Static source and repository-document review  
**Status:** Phase 0 audit and characterization baseline — production implementation not started

> This report records what the inspected source currently does. It does not claim that tests or builds were executed. The GitHub source inspection did not provide a local Rust execution environment.

---

## 1. Executive summary

The repository contains a useful foundation for the unified Smart Controllers design, but it does not yet implement that design.

The most important finding is that `smart-controller-core` currently exposes **two overlapping model directions**:

1. A substantial legacy three-anchor binding/evaluation API in `src/legacy.rs`.
2. A newer Slider/Transform parameter model split across `model.rs`, `mapping.rs`, `preview.rs` and `validation.rs`.

These models are not yet one canonical controller representation. The legacy API already contains useful three-anchor interpolation, stable binding IDs, target variants, enabled state, validation, collection management and versioned JSON. The newer model provides the current plugin's Slider/Transform preview path and endpoint-aware frame mapping.

The recommended strategy is therefore **consolidate and evolve, not rewrite**. Preserve tested V1 semantics through explicit compatibility fixtures and migration tests before retiring or changing either model.

The current plugin is a thin command adapter in broad terms and is explicitly preview-only. The inspected SDK and repository audit document do not establish continuous playback evaluation, durable arbitrary binding persistence, or a verified nested-symbol frame setter.

No production source or runtime files were changed during Phase 0. A test-only characterization file was subsequently added under `crates/smart-controller-core/tests/`.

## 2. Scope and evidence

### 2.1 Files and repository areas inspected

**Workspace and CI**
- `Cargo.toml`
- `.github/workflows/ci.yml`

**Smart Controller core**
- `crates/smart-controller-core/Cargo.toml`
- `crates/smart-controller-core/src/lib.rs`
- `crates/smart-controller-core/src/legacy.rs`
- `crates/smart-controller-core/src/model.rs`
- `crates/smart-controller-core/src/mapping.rs`
- `crates/smart-controller-core/src/preview.rs`
- `crates/smart-controller-core/src/validation.rs`
- `crates/smart-controller-core/tests/mapping_tests.rs`
- `crates/smart-controller-core/tests/validation_tests.rs`

**Plugin and SDK**
- `plugins/smart-controller/Cargo.toml`
- `plugins/smart-controller/src/lib.rs`
- `plugins/smart-controller/tests/plugin_suite.rs`
- `plugins/smart-controller/README.md`
- `crates/anicraft-plugin-sdk/Cargo.toml`
- `crates/anicraft-plugin-sdk/src/lib.rs`
- `crates/anicraft-plugin-sdk/UPSTREAM.md`

**Form Deform and project documentation**
- `crates/form-deform-core/src/lib.rs`
- `plugins/form-deform/src/lib.rs`
- `docs/SMART_CONTROLLER_PLAN.md`
- `docs/ANICRAFT_SDK_AUDIT.md`
- `docs/CAPABILITIES.md`
- `README.md`
- `docs/UNIFIED_SMART_CONTROLLERS_IMPLEMENTATION_PLAN.md`

The source directories were also listed to establish current file inventory.

### 2.2 Verification limitation

This was a static review of source and repository documentation. The following commands were **not run** during this audit:

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- package-specific Rust tests
- WASM release builds
- AniCraft in-app smoke tests

Existing CI configuration lists workspace tests and WASM builds, but the presence of those commands in a workflow is not evidence that the current branch passed them. Phase 0 therefore records test files and coverage visible in source, not a successful execution result.

## 3. Current source inventory

### 3.1 Workspace

The Rust workspace currently contains:

- `smart-controller-core`
- `form-deform-core`
- `anicraft-plugin-sdk`
- `bone-query` plugin
- `form-deform` plugin
- `smart-controller` plugin

The Smart Controller core is a normal Rust library using Serde and Serde JSON. It has no direct AniCraft or WASM dependency.

### 3.2 Smart Controller core files

| File | Current responsibility | Audit assessment |
|---|---|---|
| `src/lib.rs` | Public module declarations and re-exports | Small facade; currently exports both model directions |
| `src/legacy.rs` | Three-anchor binding model, evaluation, validation, collection and JSON | Strong reusable foundation; preserve during consolidation |
| `src/model.rs` | Slider/Transform `Params`, `Mode`, `EndpointPolicy` | Useful current plugin model, but overlaps conceptually with legacy model |
| `src/mapping.rs` | Rotation normalization, frame mapping and scalar transform | Reusable scalar primitives; additive argument is not implemented here |
| `src/preview.rs` | Pure Slider and Transform preview calculations | Reusable preview logic; V1 host adapter exposes Slider only |
| `src/validation.rs` | Mode-specific parameter validation | Useful, but returns unstructured `String` errors |
| `tests/mapping_tests.rs` | Endpoint and reversed-range tests | Focused baseline coverage |
| `tests/validation_tests.rs` | Required field and mode validation tests | Focused baseline coverage |

### 3.3 Legacy API capabilities

The legacy implementation currently provides:

- `InputRange` with negative, neutral and positive anchors.
- Strictly ordered finite input range validation.
- Linear and Smoothstep easing.
- Absolute Pose and Relative Delta modes.
- Symbol-frame and geometry-deformation target variants.
- Stable binding ID and controller ID fields.
- Enabled/disabled binding state.
- Finite-value and target validation.
- Duplicate binding ID detection.
- Binding-set lookup, insert, update and remove operations.
- A `BindingSet` JSON representation with `schema_version`.
- Deterministic evaluation of one explicit scalar input.
- Interpolation between negative, neutral and positive anchors.

This is the richest existing reusable controller logic and should be treated as a compatibility source for the canonical model.

**Important limitation:** `BindingSet` versioning currently validates one supported version. The inspected implementation does not provide a version-to-version migration registry or legacy upgrader.

### 3.4 Newer Slider/Transform model

The newer `Params` model currently contains:

- Slider and Transform modes.
- Driver layer ID and bone name.
- Action layer ID.
- Minimum and maximum rotation.
- Action start and end frames.
- Endpoint policy.
- Target bone name and property.
- Scale and additive flag.
- Optional manual input value.

Its comment describes host identity resolution as an adapter responsibility, but the model still carries numeric host layer IDs and host-oriented concepts. This is a boundary-design tension to resolve: decide which IDs are stable domain references and which belong only in the plugin adapter.

The newer model is not a recursive graph. It has no canonical controller node identity, nested dependency references, general input collection or multiple typed output bindings.

### 3.5 Mapping and preview

Current mapping functions:

- Normalize rotation to `[0, 1]).
- Permit reversed endpoints.
- Clamp input to the endpoint range.
- Map normalized values to inclusive or exclusive frame ranges.
- Calculate a scaled scalar transform value.

The `additive` argument to `transform_value` is explicitly ignored/reserved. This confirms that additive composition is not implemented by the core mapping function.

The preview layer provides:

- Slider preview with normalized input and mapped frame.
- Transform preview with normalized input and scaled output value.
- Notes describing endpoint policy and deferred host application.

The Transform function is a pure calculation foundation; the plugin rejects Transform mode in V1.

## 4. Reusability findings

### Finding R1 — Legacy binding engine is the strongest reusable foundation

**Priority:** High  
**Evidence:** `legacy.rs`

Retain and adapt its three-anchor interpolation, stable binding IDs, target variants, validation, and collection operations.

Do not copy its implementation into a second controller engine. Instead, map its existing concepts into the future canonical model through explicit compatibility conversion.

### Finding R2 — Two competing model directions must be reconciled

**Priority:** Critical  
**Evidence:** `legacy.rs`, `model.rs`, `preview.rs`, `lib.rs`

The legacy `ControllerBinding` / `BindingSet` model and newer `Params` / `Mode` model represent overlapping concepts with different structures and semantics.

Examples:
- Legacy uses a binding ID and controller ID; newer Params has no canonical controller ID.
- Legacy uses three-anchor ranges; newer Slider uses two rotation endpoints and frame endpoints.
- Legacy binds directly to SymbolFrame or GeometryDeform; newer Params carries layer/bone/property fields.
- Legacy has AbsolutePose/RelativeDelta; newer model has Slider/Transform and an additive flag.

**Recommendation:** Phase 1 must define one canonical graph/model and explicit conversion paths. Do not delete either existing API until compatibility and migration tests pass.

### Finding R3 — Shared scalar mapping can be consolidated

**Priority:** High  
**Evidence:** `legacy.rs`, `mapping.rs`, `preview.rs`

There are two related but different scalar mapping approaches:
- Legacy negative/neutral/positive interpolation.
- Newer normalized rotation-to-frame/scalar mapping.

They should share validated primitives where semantics match, but they must not be forced into one formula where their ranges or target semantics differ.

### Finding R4 — Duplicate vector types exist

**Priority:** Medium  
**Evidence:** `legacy.rs` and `form-deform-core/src/lib.rs`

Both crates define a `Vec2` with public `x` and `y` fields. They serve different domain contracts today.

Do not couple the two cores merely to remove a small duplicate type. First decide whether a tiny shared math/value crate is justified by actual reuse and dependency direction. It is also acceptable to retain separate value types and convert at an explicit boundary.

### Finding R5 — Error representation is inconsistent

**Priority:** High  
**Evidence:** `legacy.rs` versus `mapping.rs`, `preview.rs`, `validation.rs`

The legacy API has a typed `ControllerError`; the newer mapping, preview and validation APIs mostly return `Result<_, String>`.

The canonical API should use structured error types with stable variants/codes. Human-readable messages should be generated at the plugin/UI boundary.

### Finding R6 — No controller DAG or shared-node evaluation exists

**Priority:** Critical  
**Evidence:** Current core module inventory and model structures

There is no graph type, dependency validator, topological ordering, memoization, cycle handling or recursive controller evaluator.

The approved DAG design is new functionality. It should be implemented as a distinct graph/evaluation responsibility rather than embedded into mapping functions or the plugin.

### Finding R7 — No joystick or general 2D pose mapping exists

**Priority:** High  
**Evidence:** Existing mapping and preview modules

The legacy API has a 2D target offset, but its driver input is scalar. This is not a 2D joystick input or a 2D pose interpolator.

The five-pose joystick, boundary policies, triangle selection and barycentric interpolation are new capabilities.

### Finding R8 — Composition is incomplete

**Priority:** Critical  
**Evidence:** `mapping.rs`, `preview.rs`, `legacy.rs`

The existing `additive` flag is not applied by `transform_value`. The inspected core does not implement the approved Replace, Additive, WeightedBlend and Priority composition engine.

Do not interpret the current additive flag as evidence that additive composition is already supported.

### Finding R9 — Plugin is a thin but monolithic adapter

**Priority:** Medium  
**Evidence:** `plugins/smart-controller/src/lib.rs`

The plugin currently:
- Deserializes `Params`.
- Calls core validation.
- Rejects Transform mode in V1.
- Uses manual input when supplied.
- Otherwise invokes `bone.info`.
- Resolves the named bone's angle.
- Calls pure Slider preview.
- Serializes and logs the result.

The adapter has a small local bone response parser. This is reasonable at current size. Do not prematurely create a host-support crate; extract helpers only when multiple plugins demonstrably need the same abstraction and the ownership boundary is clear.

## 5. Host and SDK capability findings

### 5.1 Existing SDK boundary

The vendored `anicraft-plugin-sdk` exposes the ABI v1 guest helpers, including:

- `command(id, params)`
- `document()`
- `selected_elements(paths)`
- `log(message)`
- plugin/exporter ABI macros
- exporter-specific movie/frame helpers

Native host-call stand-ins return errors, which supports testing pure core code natively. They do not simulate a functioning AniCraft host.

The SDK is already a shared crate used by the plugins. Continue using it; do not introduce a second host communication layer.

### 5.2 Host capability conclusions

The existing `docs/ANICRAFT_SDK_AUDIT.md` records the inspected AniCraft contract. For this Smart Controller plan, retain these constraints:

| Capability | Audit conclusion |
|---|---|
| On-demand `bone.info` query | Documented and used by current adapter |
| Manual input preview | Supported by current plugin |
| Continuous custom playback/scrubbing evaluator | Not established by current command-plugin ABI |
| Durable arbitrary plugin-owned document metadata | Not established by inspected SDK |
| Nested symbol instance displayed-frame setter | Not established by inspected command reference |
| Host mutation for Form Deform pin movement | Available as an explicit command; separate from Smart Controller V1 preview |

The Smart Controller plugin must not claim live playback, persistent controller storage or symbol-frame visual application until the host surface is independently verified and implemented.

### 5.3 SDK provenance

The vendored SDK has an explicit upstream source path, pinned commit and update procedure in `UPSTREAM.md`. Preserve this provenance and do not silently track upstream main.

## 6. Form Deform boundary review

### 6.1 Core separation

`form-deform-core` owns a separate deterministic pin-position formula:

`neutral_position + (current_angle - neutral_angle) * gain_per_degree`

It validates finite inputs and returns an absolute stage-space pin position. It is not the same calculation as legacy three-anchor controller interpolation.

Keep this evaluator independent.

### 6.2 Integration-test discrepancy requiring resolution

`plugins/smart-controller/tests/plugin_suite.rs` currently evaluates a legacy Smart Controller geometry binding, takes its relative offset, uses that offset to construct the neutral position passed into `form-deform-core`, and then evaluates Form Deform.

This test is a mathematical cross-crate integration example, not a runtime plugin composition pipeline. However, its current naming and comments can be read as endorsing controller-output-to-Form-Deform composition.

That is ambiguous against the accepted CCC-01.5 rule: Smart Controller binding evaluation and Form Deform pin evaluation remain independent until FDM v2 explicitly defines multi-source composition semantics.

**Recommendation:** Do not silently delete the test or change either formula in Phase 0. In a later explicitly scoped test/documentation change, clarify that the test only checks independent pure calculations and value conversion, or replace it with independent regression tests that do not imply composition. Any actual composition requires a separate approved FDM v2 decision.

## 7. Test coverage inventory and gaps

### 7.1 Existing coverage visible in source

| Area | Existing coverage | Gap |
|---|---|---|
| Legacy interpolation | Embedded unit tests in `legacy.rs` | No golden compatibility fixture inventory established |
| Legacy binding collection | Embedded tests for binding set behaviour | No canonical graph migration tests |
| Rotation mapping | Reversed range tests | No general property/fuzz coverage |
| Frame mapping | Inclusive/exclusive and degenerate helper tests | No full plugin endpoint regression fixture |
| Parameter validation | Slider/Transform required fields and manual input | String errors; no structured diagnostics |
| Plugin bone lookup | Name lookup and malformed response tests | No host-invocation test double for `bone.info` request/error contract |
| Cross-crate suite | Smart Controller legacy math plus Form Deform calculation | Clarify against CCC-01.5 as described above |
| DAG | None identified | Entire feature absent |
| Joystick / 2D mapping | None identified | Entire feature absent |
| Composition | None identified | Entire feature absent |
| Schema migration | Current version check only in legacy BindingSet | No migration path or golden schema fixtures identified |
| Stale input / diagnostics | None identified | Entire policy and implementation absent |
| Determinism / memoization | Pure scalar functions exist | No graph determinism or diamond memoization tests |

### 7.2 Baseline fixture requirement

Before replacing or reshaping the legacy model, capture representative serialized examples and expected outputs for:

- SymbolFrame negative, neutral and positive anchors.
- GeometryDeform AbsolutePose.
- GeometryDeform RelativeDelta.
- Linear and Smoothstep easing.
- Disabled binding.
- BindingSet round trip.
- Duplicate binding ID rejection.
- New Slider exclusive and inclusive endpoints.
- Reversed rotation range.
- Manual input and bone-query parameter paths.

Fixtures should be committed under a stable test fixture directory in the implementation phase. This audit did not create fabricated fixture files or claim fixture tests passed.

## 8. Compatibility and migration matrix

| Existing concept | Proposed destination | Migration / compatibility action |
|---|---|---|
| Legacy `ControllerBinding` | Canonical Controller + Binding representation | Write explicit converter; preserve IDs and target semantics |
| Legacy `BindingSet` schema v1 | Versioned canonical schema | Add fixture and upgrader; reject unknown unsupported versions clearly |
| Legacy three-anchor `InputRange` | Shared scalar range/anchor mapping | Preserve strict neutral-between-endpoints semantics |
| Legacy Linear/Smoothstep | Shared easing module | Preserve output values with regression tests |
| Legacy AbsolutePose / RelativeDelta | Explicit output/reference semantics | Do not equate with Replace/Additive without formal mapping |
| Legacy SymbolFrame target | Host-independent target descriptor | Host adapter resolves target; actual frame application remains unverified |
| Legacy GeometryDeform target | Host-independent target descriptor | Keep independent of Form Deform evaluation |
| New `Params` Slider | Canonical input + frame mapping + binding | Preserve inclusive/exclusive endpoint behaviour |
| New `Params` Transform | Canonical property mapping | Keep host application deferred until verified |
| New `additive` flag | Explicit composition configuration | Do not treat current ignored flag as implemented semantics |
| Plugin JSON manifest | Host-facing request schema | Keep adapter compatibility until migration is tested |

## 9. Findings by priority

### Critical — Resolve before Phase 1 API is locked

1. Define the canonical model that reconciles legacy three-anchor bindings and newer Slider/Transform Params.
2. Define stable domain identities separately from host-specific IDs.
3. Establish schema envelope/version/migration strategy that can ingest the existing BindingSet v1 and current Params JSON as appropriate.
4. Specify DAG node identity and dependency semantics.
5. Define typed output/binding ownership and composition semantics.
6. Clarify the Smart Controller/Form Deform integration test's intent against CCC-01.5.

### High — Address in early implementation phases

1. Consolidate mapping primitives without changing existing results.
2. Replace string errors with structured typed errors.
3. Implement 2D joystick mapping and explicit boundary rules.
4. Add explicit input validity/freshness/fallback representation.
5. Add deterministic graph evaluation and per-invocation memoization.
6. Add golden serialization and evaluation fixtures.

### Medium — Address when justified

1. Consider common vector primitives only if a dependency-neutral shared crate offers real reuse.
2. Extract shared host query parsing only if there is proven cross-plugin reuse.
3. Split plugin adapter modules when the unified plugin grows beyond its current small entry point.
4. Keep documentation and CI package names synchronized with actual workspace package names.

## 10. Recommended Phase 1 adjustments

The proposed file-level build order remains usable, with these refinements:

1. **Do not immediately replace `model.rs` or `legacy.rs`.** First introduce canonical types alongside compatibility conversion, then migrate callers after tests prove equivalence.
2. **Decide identity ownership before defining `ids.rs`.** Distinguish controller, binding, input, graph-node and host-target identities.
3. **Keep the core independent of host layer IDs.** Adapter-resolved stable references should enter through explicit snapshots or opaque host-independent source identities.
4. **Do not put every enum in one universal `Value`.** Define a minimal typed value system with explicit compatibility rules.
5. **Treat `BindingSet` v1 and `Params` JSON as separate legacy input formats** unless evidence proves they are one persisted format.
6. **Make schema migration deterministic and loss-aware.** Unknown fields/data must not be silently discarded; current `deny_unknown_fields` behaviour should be accounted for.
7. **Separate graph validation from evaluation.** The evaluator must still defend itself against invalid graphs.
8. **Preserve V1 plugin output and preview-only behaviour** until the unified plugin adapter has compatibility tests.

## 11. Follow-up plan review: adopted design enhancements

A subsequent architecture review identified underspecified contracts in the initial plan. The following decisions are now adopted in the implementation plan before source implementation begins:

1. **Pose algebra:** `Pose2D` is an ordered set of `(BindingId, TypedValue)` entries, interpolated per property. Participating poses require compatible binding schemas. Wrapped and unwrapped angles have distinct interpolation semantics; wrapped angles use deterministic shortest-path interpolation.
2. **Evaluation stages:** resolve the complete input snapshot first, evaluate nodes in deterministic topological order, collect binding contributions, then compose targets in a separate post-pass. Disabled outputs are unavailable to dependants; no implicit neutral/zero/cache substitution is permitted.
3. **Composition model:** use a common typed contribution record. All contributions to one target must agree on composition mode or return a typed conflict. The priority mode is named `PriorityOverride`; Additive and WeightedBlend accumulation use canonical BindingId order.
4. **WeightedBlend eligibility:** a contribution is eligible only when it produced a valid value and has a finite positive weight. Zero weight is excluded; invalid negative/non-finite weights are errors; no eligible contributions means no target output plus a diagnostic.
5. **Freshness:** each sample carries monotonic `age_ticks`; the snapshot carries `max_age_ticks`. Stale means strictly greater than the maximum. The core does not read a clock; unknown host freshness is explicit/unavailable.
6. **Floating point:** reject non-finite values, sort accumulation deterministically and use a combined absolute/relative test tolerance of (10^{-9}).
7. **Diagnostics:** Phase 1 owns a stable diagnostic-code registry and checked-in fixture; diagnostic meanings cannot silently change.
8. **Graph correctness:** reject parent-scoped configuration on nodes shared by multiple parents; retain evaluator-side cycle defence through a test-only unchecked graph builder.
9. **Compatibility:** Phase 0 adds executable V1 characterization tests. Phase 1 adds legacy-to-canonical conversion and round-trip/loss diagnostics for both BindingSet v1 and unversioned Params fixtures, before evaluator/adapter integration.
10. **Phase sequencing:** pure range/frame mapping moves into Phase 1; Pose2D remains in Phase 2; Phase 3 owns the four-stage evaluator; Phase 4 completes composition; Phase 5 only wires tested migration; Phase 6 is split into core mapping extensions and host-authoring work contingent on a new host audit.
11. **No-op cases:** empty graphs, all-disabled graphs and targets with no eligible contributions produce documented empty/unavailable results and diagnostics rather than fabricated values.
12. **Portability and independence:** audit and enforce core dependency/WASM constraints. Add CI dependency-boundary checks in both directions between Smart Controller core and Form Deform core.

These are plan-level contracts only. The Phase 0 source audit and three JSON fixture captures remain complete. The executable characterization suite has now been authored at `crates/smart-controller-core/tests/v1_characterization_tests.rs`; it covers fixture deserialization/round trips and captured V1 outputs. The three fixture files were independently parsed as JSON and their expected Slider normalization/frame outputs were checked, but the Rust/Serde test suite has not yet been executed. Rust workspace tests, WASM builds and host smoke tests remain pending. No production implementation or runtime semantics changed.

### Additional acceptance tests

- `v1_characterization_tests.rs`
- `migration_roundtrip_tests.rs`
- `angle_interpolation_tests.rs`
- `pose2d_tests.rs`
- `composition_conflict_tests.rs`
- `freshness_tests.rs`
- `diagnostic_registry_tests.rs`
- `graph_tests.rs`, including unchecked-cycle defence
- `no_op_graph_tests.rs`
- `dependency_boundary_tests.rs`

The earlier source findings remain valid: no DAG, joystick, composition or migration implementation was found in the inspected code. These requirements are future acceptance criteria, not claims that the features exist.

## 12. Phase 0 exit assessment

| Deliverable | Status |
|---|---|
| Source inventory | Complete for inspected Smart Controller/core/plugin/SDK/Form Deform files |
| Reusability report | Complete in Sections 3–4 |
| Compatibility matrix | Complete in Section 8 |
| Host capability report | Reconciled with existing SDK audit in Section 5 |
| Test coverage/gap report | Complete in Section 7 |
| Migration risks | Complete in Sections 8–10 |
| Golden fixture baseline | Three fixtures captured; Rust/Serde deserialization and round-trip assertions passed in the V1 characterization suite |
| Runtime test/build evidence | Verified at `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`; details in §12.1 |
| Formatting/lint | `cargo fmt --all -- --check` and Clippy fail on both Phase 0 and `main`; pre-existing baseline findings plus formatting in the new characterization test are documented in §12.1 |
| AniCraft in-app smoke test | Not performed |

**Phase 0 status: exit criteria met at `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`.**

Captured fixtures:
- `legacy_binding_set_v1.json` — representative version-1 binding set with SymbolFrame and GeometryDeform targets.
- `legacy_params_slider_unversioned.json` — current host-driven Slider Params payload.
- `legacy_params_slider_manual_unversioned.json` — current manual-input Slider Params payload.

The two Params fixtures are deliberately labelled unversioned: the current `Params` structure has no schema envelope/version field. The fixtures are source-derived compatibility inputs, not proof that deserialization or round-trip tests passed. They must be loaded by tests and verified before any model migration. The BindingSet fixture must likewise be checked against the current Serde representation by an executable test.

Phase 0 is complete for architectural discovery, baseline capture and V1 characterization. The user ran the core and workspace Rust suites, the Smart Controller plugin WASM build, TypeScript checking and Bun tests at the recorded commit. The remaining WASM builds for Bone Query and Form Deform, plus the AniCraft in-app smoke test, remain outstanding. Formatting and Clippy failures are documented as baseline maintenance findings; they are not listed as Phase 0 exit gates.

## 13. Authorization and hard boundaries

On 2026-11-21, the user explicitly authorized Phase 1 implementation. That authorization applies to the Phase 1 scope on `feat/smart-controller-phase1-foundation`; it does not authorize unrelated repository changes.

The following remain outside scope unless separately approved:

- Changes to Form Deform formulas or runtime behaviour.
- Combining Smart Controller outputs with Form Deform pin evaluation.
- Host API changes in AniCraft.
- Pull-request creation, review approval or merge.

The accepted CCC-01.5 separation remains in force. No merge has been performed or authorized.


### 12.1 Verification record

**Verified base:** `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`  
**Branch:** `feat/smart-controller-phase0-characterization`

| Command/check | Result |
|---|---|
| `cargo test -p smart-controller-core` | 36 passed, 0 failed |
| `cargo test --workspace` | 52 passed, 0 failed; 7 more than `main` (45), attributable to the V1 characterization additions |
| WASM build of `anicraft-smart-controller-plugin` | Finished successfully; Cargo reused the previous build artifacts. Plugin/core sources did not differ from `main`, so this was not a fresh recompilation. |
| `tsc --noEmit` | Clean |
| `bun test` | 9 of 9 passed |
| `cargo fmt --all -- --check` | Failed on both this branch and `main`. Diffs span 15 files across all six crates. The legacy unit tests identified in the original comparison pre-date Phase 0; the new `v1_characterization_tests.rs` also has formatting diffs. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Failed on both this branch and `main`. Reported errors: `derivable_impls` at `crates/smart-controller-core/src/model.rs:15` and `collapsible_if` at `plugins/bone-query/src/lib.rs:55`. |

**Baseline comparison:** `main` was `6b53aac` (`6b53aac51ddc15249c5100745726027fd9c75907`) and failed the same formatting and Clippy checks. These failures are not regressions introduced by the Phase 0 runtime/test additions, except that the newly added characterization test file itself needs formatting.

**Clippy completeness:** The two reported Clippy errors prevented dependent crates from being linted. Additional findings may appear after those blockers are addressed.

**CI policy:** CI installs rustfmt but does not currently enforce a formatting check or Clippy step.

**Not run:** WASM builds for `plugins/bone-query` and `plugins/form-deform`; AniCraft in-app smoke test. The Phase 0 verification does not claim these checks passed.

**Toolchain note:** This record intentionally does not infer Rust/Cargo versions; record the exact local toolchain versions alongside the verification log if they are available.

## 14. Phase 1 foundation progress

**Branch:** `feat/smart-controller-phase1-foundation`  
**Base:** `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`

Implemented the first four Phase 1 responsibilities without replacing existing V1 models:

- `error.rs`: typed core errors for identifiers, finite values, input validation, model/mapping/graph/composition, schema, serialization and evaluation.
- `ids.rs`: strongly typed, ordered and serializable ControllerId, InputId, BindingId and TargetId.
- `value.rs`: finite Scalar and Vector2, explicit wrapped/unwrapped Angle, and TypedValue. Deserialization validates finite numeric values.
- `input.rs`: host-independent input sources, dimensions, explicit fallback and validity policies, sample ages, snapshot maximum age, duplicate checks, dimension checks and freshness classification. Staleness uses the strict rule `age_ticks > max_age_ticks`; absent age remains Unknown.
- `lib.rs`: exports the new modules while preserving the existing legacy and Params public API.
- `tests/foundation_types_tests.rs`: tests the new foundational contracts.

**Unchanged:** `legacy.rs`, `model.rs`, the existing scalar/frame mapping and preview functions, and host adapter runtime behaviour have not been replaced by this increment. Smart Controller core and Form Deform core remain independent.

**Verification status:** The Phase 0 results above apply to the base commit, not to this new Phase 1 branch. The new foundation test file and Phase 1 branch have not yet been executed with Cargo; rerun core/workspace tests and WASM checks on the Phase 1 branch before treating this increment as verified.


### 14.1 Canonical contracts increment — not yet verified

Additional files on feat/smart-controller-phase1-foundation:

- canonical.rs: additive canonical controller/configuration/binding types and local validation.
- graph.rs: validated DAG storage, dependency existence, shared-node parent-scope checks, cycle detection and deterministic topological ordering.
- schema.rs: schema identity and version-1 envelope.
- diagnostic_codes.rs and tests/fixtures/diagnostic_codes_v1.json: initial stable diagnostic registry.
- contribution.rs: typed contribution data shape and mode-specific optional-field validation; no composition execution.
- mapping/range.rs, mapping/easing.rs, mapping/frame.rs: pure mapping primitives; legacy mapping functions remain present.
- Tests: graph, range/frame/easing, diagnostic registry, schema and contribution contract tests.

No legacy migration converter, migration registry, explicit loss report, canonical schema dispatch, or full canonical model golden round-trip tests are present yet. Existing legacy.rs and Params model remain unchanged. This increment is implementation work, not a verified Phase 1 completion.

**Verification:** These changes have not been compiled or tested with Cargo. Do not claim the Phase 1 tests pass until a Rust CI run or local toolchain confirms them.


### 14.2 Migration and contract increment — implementation status

The Phase 1 branch now includes:

- migration.rs: strict dispatch and migration from BindingSet V1 and unversioned Params to CanonicalDocument.
- MigrationReport: explicit notices and losses; generated Params IDs are documented as deterministic migration IDs, not durable authored IDs.
- Reverse exporters for BindingSet V1 and unversioned Params. They reject canonical structures that cannot be represented exactly; they do not silently discard unsupported data.
- Validated canonical JSON persistence with schema identity/version checks and strict unknown-field rejection.
- Structured diagnostics tied to the stable diagnostic_codes_v1.json registry.
- Canonical-owned three-anchor/easing types, independent of the legacy model.
- Pure range/frame/easing mapping primitives with finite-input/overflow checks and parity tests against the characterized V1 range/frame functions.
- Graph tests for duplicate dependency edges and parent-scoped configuration on shared DAG nodes.

Migration tests now cover exact BindingSet round-trip, Params slider/transform round-trip, optional endpoint-policy preservation, driver identity preservation, migration dispatch, strict unknown-field rejection, empty BindingSets and explicit rejection of non-representable canonical exports.

**Outstanding:** the core/Form Deform dependency-boundary test and final source review. Rust execution remains unverified: no Phase 1 Cargo test, formatting, Clippy or WASM result is claimed. The existing V1 legacy.rs/Params APIs remain in place; no runtime integration or CCC-01.5 behavior has been changed.


The dependency-boundary test now checks both core crate manifests for direct normal/dev/build/target-specific dependencies in either direction. This test is authored but has not been executed with Cargo.
