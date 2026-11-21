# Unified Smart Controllers — Architecture and Implementation Plan

**Status:** Proposed implementation baseline  
**Target repository:** `Mujshanan/forma-deform`  
**Branch:** `docs/unified-smart-controllers-plan`  
**Core:** `smart-controller-core`  
**Host adapter:** `plugins/smart-controller`  
**Scope:** Architecture, implementation sequence and file-level build order  
**Execution status:** Documentation only; no source implementation is authorized by this plan.

---

## 1. Objective

Build one unified, host-independent Smart Controller engine. Slider, joystick, Smart Bone, master/nested controller and switch are not separate controller engines. They are configurations or capabilities of one composable system.

The system accepts one-dimensional and two-dimensional inputs, maps them to animation frames, poses, properties or discrete selections, composes reusable nested controllers, and produces typed output bindings.

The implementation must be incremental, test-driven and compatible with useful existing Smart Controller V1 behaviour.

## 2. Foundational architecture

The engine has six cooperating responsibilities:

1. **Controller model** — canonical controller identity, configuration, inputs, mapping, dependencies and bindings.
2. **Input system** — manual values, bone transforms and nested-controller outputs resolved into explicit snapshots.
3. **Mapping system** — scalar ranges, animation-frame mapping, pose interpolation and discrete mapping.
4. **Composition system** — explicit combination of multiple contributions.
5. **Evaluation engine** — pure, deterministic evaluation of a controller dependency graph.
6. **Binding system** — typed outputs associated with host-independent target identities.

The core must not query or mutate the host application. Host-specific resolution and commands belong in `plugins/smart-controller` and the existing SDK boundary.

## 3. Architectural decisions

### 3.1 One controller model

All controller behaviours use one canonical model. A controller can have multiple inputs, a mapping configuration, nested dependencies and multiple output bindings.

The model must support scalar and 2D input dimensions without requiring separate slider and joystick evaluators.

### 3.2 Controller dependencies form a DAG

Use a directed acyclic graph rather than a strict tree. This permits shared sub-controllers.

For a diamond dependency (Master → Child A and Child B → Shared Leaf):

- The graph contains one canonical Shared Leaf node.
- Topological evaluation respects all dependencies.
- Independent ready nodes are ordered by canonical controller ID.
- Each node is evaluated once per evaluation invocation.
- Both parents consume the same memoized result.
- Memoization is scoped to one invocation and never reused across snapshots.

Reject self-loops and indirect cycles during graph validation. The evaluator must also defend against cycles and return a typed error if an invalid graph reaches it.

A shared node has one configuration and one result per invocation. Parent-specific overrides must be represented explicitly as separate nodes or explicit input contexts, not hidden branch-specific evaluation.

### 3.3 Joystick mapping

The first 2D mapping is a five-pose joystick: center, up, right, down and left.

Represent 2D poses through a common `Pose2D` abstraction and evaluate them through a reusable 2D mapping contract. The initial five-pose domain is divided into four triangles:

- center–up–right
- center–right–down
- center–down–left
- center–left–up

Use barycentric interpolation inside each triangle. The mapping must support non-uniformly spaced authored poses; it must not assume equal angular spacing.

Boundary behaviour is explicit and configurable. Initial policies:

- unit-square clamping
- unit-circle radial clamping
- reject out-of-domain input

Default the five-pose joystick to radial unit-circle clamping. Future triangulated arbitrary-pose and grid mappings must fit the same 2D mapping boundary.

### 3.4 Composition modes

The schema defines four modes:

- **Replace**
- **Additive**
- **WeightedBlend**
- **Priority**

An unimplemented or unsupported mode must return a clear typed error. Never silently fall back to Replace or no-op.

Composition order:

1. Resolve the target base/neutral value from explicit input.
2. Collect all contributions for the target.
3. Validate value types and compatibility.
4. Resolve deterministic contribution order.
5. Apply the configured composition rule.
6. Validate the final value.
7. Emit the output and diagnostics.

Additive contributions must identify their reference base explicitly (neutral, authored base or supplied host-state value). They must not depend on incidental iteration order.

Weighted blend must define non-negative weight semantics, normalization, zero-total-weight behaviour and unavailable-contribution handling. Initial recommendation: non-negative weights, normalized over eligible contributions, with explicit unavailable/error behaviour when total weight is zero.

PriorityOverride uses explicit numeric priority. Equal priorities use canonical binding ID as a deterministic tie-breaker.

Conflicting Replace contributions to one target must use an explicit precedence rule or return a conflict error; they must never overwrite each other accidentally.

### 3.5 Pure deterministic evaluation

Conceptual contract:

```rust
fn evaluate(
    graph: &ControllerGraph,
    inputs: &InputSnapshot,
    host_state: &HostStateSnapshot,
) -> Result<EvaluationResult, EvaluationError>;
```

Final signatures may use borrowed data or other efficient Rust representations.

The core must not depend on wall-clock time, random numbers, global mutable state, host commands, direct document access or uncontrolled external state.

Identical graph, input snapshot, host-state snapshot and evaluation configuration must produce equivalent outputs and diagnostics. Floating-point tolerances and finite-value rules must be documented.

The result contains outputs, diagnostics/warnings, unavailable outputs and relevant evaluation metadata. Recoverable input diagnostics must not unnecessarily invalidate unrelated outputs; fatal graph/configuration failures must be distinguishable.

### 3.6 Missing, stale and disabled inputs

The core receives validity/freshness information in explicit snapshots; it does not consult a clock.

Default semantics:

| Condition | Default |
|---|---|
| Missing bone input | Configured neutral/rest fallback plus diagnostic, or unavailable if no fallback is configured |
| Missing nested output | Configured fallback or unavailable |
| NaN / infinity | Reject sample; apply configured fallback and diagnostic |
| Out-of-range value | Apply mapping boundary policy |
| Stale input | Reject according to explicit freshness policy |
| Disabled controller | Omit its outputs from composition |
| Missing required input | Apply configured fallback or mark output unavailable |

Do not implicitly hold the last value. This would introduce hidden state. If neutral output is desired for a disabled controller, configure it explicitly.

### 3.7 Schema versioning and migration

Establish schema identity and version in Phase 1. The initial version number is assigned when the canonical schema is approved; the following is illustrative only:

```json
{
  "schema": {
    "name": "smart-controller",
    "version": 1
  },
  "controller": {
    "id": "controller-001"
  }
}
```

Requirements:

- explicit schema name and version
- version-specific deserialization
- sequential migration between supported versions
- validation after migration
- legacy V1 conversion
- golden serialization fixtures
- round-trip and migration regression tests
- clear unsupported-version errors
- no silent loss of unknown or unsupported configuration data

## 4. Repository responsibilities

| Component | Responsibility |
|---|---|
| `crates/smart-controller-core` | Canonical model, input abstractions, mapping, DAG evaluation, composition, bindings and diagnostics |
| `plugins/smart-controller` | Unified animator-facing plugin and host adapter |
| `crates/anicraft-plugin-sdk` | Existing host communication, command invocation and plugin integration |
| `plugins/bone-query` | Independent read-only inspection and diagnostics |
| `crates/form-deform-core` | Independent deterministic pin-deformation evaluation |
| `plugins/form-deform` | Form Deform host integration |

Do not create a support crate merely for organizational convenience. Extract shared host adapter helpers only when Phase 0 proves that they are genuinely reused.

## 5. File-level build order

The order below is the proposed implementation sequence. Phase 0 inspects existing files and records the actual baseline before any source file is changed. During Phase 1, reconcile this proposed layout with the current crate; do not create duplicate modules if an existing file already owns the responsibility.

### Phase 0 — Audit and baseline (documentation/tests first)

| Order | File | Action | Required content |
|---:|---|---|---|
| 0.1 | `docs/SMART_CONTROLLER_PLAN.md` | Inspect; later reconcile | Existing V1 scope, endpoint policy, host boundaries and deferred features. Preserve its useful V1 facts; identify obsolete assumptions. |
| 0.2 | `crates/smart-controller-core/src/legacy.rs` | Inspect | Legacy three-anchor API, stable IDs, interpolation, absolute/relative modes, targets, validation and callers. |
| 0.3 | `crates/smart-controller-core/src/model.rs` | Inspect | Newer Params/Mode model, host identifiers, configuration overlap and migration needs. |
| 0.4 | `crates/smart-controller-core/src/mapping.rs` | Inspect | Rotation normalization, reversed ranges, frame endpoint policies, transform mapping and reserved semantics. |
| 0.5 | `crates/smart-controller-core/src/preview.rs` | Inspect | Existing pure preview functions, output types and V1 limitations. |
| 0.6 | `crates/smart-controller-core/src/validation.rs` | Inspect | Existing validation, error representation and missing edge cases. |
| 0.7 | `crates/smart-controller-core/src/lib.rs` | Inspect | Public exports, compatibility surface and module layout. |
| 0.8 | `crates/smart-controller-core/tests/*` | Inspect/add baseline fixtures | Current tests, golden V1 serialized examples and expected evaluation outputs. |
| 0.9 | `plugins/smart-controller/src/lib.rs` | Inspect | SDK entry point, bone query parsing, manual input, preview-only behaviour and host assumptions. |
| 0.10 | `crates/anicraft-plugin-sdk/*` | Inspect | Verified query/command contracts, error behaviour and adapter support. |
| 0.11 | `docs/SMART_CONTROLLERS_AUDIT.md` | Add | File inventory, reuse map, overlapping models, compatibility matrix, host capability evidence, test gaps and migration risks. |

**Phase 0 exit:** The existing implementation and host boundary are documented; golden fixtures are captured; no existing semantics are removed.

### Phase 1 — Canonical types, model and schema

Create or adapt files in this order:

| Order | File | Required content |
|---:|---|---|
| 1.1 | `crates/smart-controller-core/src/error.rs` | Typed model, validation, mapping, graph, composition, serialization and evaluation errors. Errors must be structured enough for tests and plugin diagnostics. |
| 1.2 | `crates/smart-controller-core/src/ids.rs` | Strongly typed ControllerId, InputId, BindingId and target identifiers; stable ordering and serialization rules. |
| 1.3 | `crates/smart-controller-core/src/value.rs` | Finite scalar, 2D vector and supported typed output values; explicit type compatibility and finite-value validation. |
| 1.4 | `crates/smart-controller-core/src/input.rs` | InputSource definitions, dimensions, validity/freshness metadata, fallback policy and snapshot data structures. No host API types. |
| 1.5 | `crates/smart-controller-core/src/model.rs` | Canonical Controller, mapping configuration references, nested dependency references, enabled state, bindings and composition configuration. Replace competing model directions only after conversion tests exist. |
| 1.6 | `crates/smart-controller-core/src/graph.rs` | ControllerGraph storage, dependency extraction, graph integrity checks and deterministic graph representation. |
| 1.7 | `crates/smart-controller-core/src/serialization.rs` | Schema envelope, version dispatch, versioned decode/encode interfaces and migration registry. |
| 1.8 | `crates/smart-controller-core/src/validation.rs` | Canonical model validation, IDs, finite values, graph references, mapping compatibility, binding types and unsupported-feature errors. |
| 1.9 | `crates/smart-controller-core/src/lib.rs` | Public module declarations and intentionally supported API re-exports. Keep implementation details private unless needed by consumers. |
| 1.10 | `crates/smart-controller-core/tests/model_tests.rs` | Model construction, identity, validation, graph references and type compatibility tests. |
| 1.11 | `crates/smart-controller-core/tests/serialization_tests.rs` | Golden fixtures, round trips, unsupported versions and migration tests. |
| 1.12 | `crates/smart-controller-core/tests/fixtures/*` | Canonical schema examples and legacy V1 fixture files. |

**Phase 1 exit:** One authoritative model represents all controller behaviours, scalar/2D inputs, nested dependencies and multiple bindings. Schema and errors are explicit.

### Phase 2 — Input resolution and mapping

| Order | File | Required content |
|---:|---|---|
| 2.1 | `crates/smart-controller-core/src/input.rs` | Complete snapshot lookup, validation of supplied sample status and fallback resolution contracts. |
| 2.2 | `crates/smart-controller-core/src/mapping/mod.rs` | Shared Mapping1D/Mapping2D contracts, mapping configuration dispatch and common output types. |
| 2.3 | `crates/smart-controller-core/src/mapping/range.rs` | Scalar normalization, three-anchor ranges, reversed endpoints, clamping and endpoint policy. |
| 2.4 | `crates/smart-controller-core/src/mapping/easing.rs` | Linear and Smoothstep initially; validated easing inputs and deterministic output. |
| 2.5 | `crates/smart-controller-core/src/mapping/frame.rs` | Inclusive/exclusive animation frame mapping, finite/range validation and preserved V1 semantics. |
| 2.6 | `crates/smart-controller-core/src/mapping/pose1d.rs` | One-dimensional authored pose interpolation and explicit endpoint behaviour. |
| 2.7 | `crates/smart-controller-core/src/mapping/pose2d.rs` | Pose2D, five-pose regions, triangle selection, barycentric interpolation and non-uniform pose handling. |
| 2.8 | `crates/smart-controller-core/src/mapping/boundary.rs` | Unit-square clamp, radial unit-circle clamp and reject policies. |
| 2.9 | `crates/smart-controller-core/tests/range_mapping_tests.rs` | Reversed ranges, endpoints, clamping, easing and frame mapping regression tests. |
| 2.10 | `crates/smart-controller-core/tests/pose2d_tests.rs` | Center/cardinal exactness, triangle edges, continuity, non-uniform poses and boundary policies. |

The exact module split may be adjusted after Phase 0. Do not retain duplicate scalar mapping implementations merely because their original files differ.

**Phase 2 exit:** Slider and joystick use the same mapping infrastructure; bone-driven and manual inputs resolve through the same input contract.

### Phase 3 — Pure DAG evaluation

| Order | File | Required content |
|---:|---|---|
| 3.1 | `crates/smart-controller-core/src/graph.rs` | Deterministic topological ordering, canonical-ID tie-breaks, self-loop and cycle detection. |
| 3.2 | `crates/smart-controller-core/src/diagnostic.rs` | Structured warnings, missing/stale/invalid input diagnostics, skipped/unavailable output diagnostics and stable diagnostic codes. |
| 3.3 | `crates/smart-controller-core/src/evaluation.rs` | Pure evaluation entry point, per-invocation memoization, dependency evaluation, disabled-node handling and result construction. |
| 3.4 | `crates/smart-controller-core/src/lib.rs` | Public evaluator and result exports after API review. |
| 3.5 | `crates/smart-controller-core/tests/graph_tests.rs` | Chains, independent nodes, diamond DAG, deterministic ties, self-loop and indirect cycle tests. |
| 3.6 | `crates/smart-controller-core/tests/evaluation_tests.rs` | Memoization, identical-snapshot determinism, disabled nodes, missing/stale/NaN inputs, fallback and diagnostic tests. |

Evaluation memoization is local to one call. No global caches, last-value state, wall-clock reads or random values are permitted.

**Phase 3 exit:** Valid graphs evaluate deterministically; shared nodes are evaluated once per invocation; invalid cycles fail safely.

### Phase 4 — Bindings and composition

| Order | File | Required content |
|---:|---|---|
| 4.1 | `crates/smart-controller-core/src/binding.rs` | Host-independent target identity, property identity, typed output and binding-level configuration. |
| 4.2 | `crates/smart-controller-core/src/composition.rs` | Replace, Additive, WeightedBlend and PriorityOverride contracts; deterministic ordering; explicit base references; conflict and unsupported-mode errors. |
| 4.3 | `crates/smart-controller-core/src/evaluation.rs` | Integrate evaluated controller values with binding collection and final target output generation without host mutation. |
| 4.4 | `crates/smart-controller-core/tests/composition_tests.rs` | All modes, unsupported modes, zero weights, equal priority, conflicting Replace, additive bases and type mismatch. |
| 4.5 | `crates/smart-controller-core/tests/binding_tests.rs` | Multiple outputs, multiple contributors, target identity and unavailable target cases. |

**Phase 4 exit:** Outputs are typed and deterministic; composition never relies on incidental iteration order or silent fallback.

### Phase 5 — Plugin adapter and legacy migration

| Order | File | Required content |
|---:|---|---|
| 5.1 | `plugins/smart-controller/src/adapter.rs` | Host-specific bone/input queries, source identity resolution and conversion to core snapshots using verified SDK commands only. |
| 5.2 | `plugins/smart-controller/src/targets.rs` | Host target identity resolution and explicit capability checks; no speculative write or playback commands. |
| 5.3 | `plugins/smart-controller/src/migration.rs` | Legacy V1 parameter/binding conversion into the canonical schema with explicit diagnostics for unsupported data. |
| 5.4 | `plugins/smart-controller/src/lib.rs` | Thin plugin entry point: parse request, call adapter, invoke core evaluator, serialize/log preview and diagnostics. |
| 5.5 | `plugins/smart-controller/tests/adapter_tests.rs` | Host response parsing, missing bones, malformed response and capability failures. |
| 5.6 | `plugins/smart-controller/tests/migration_tests.rs` | Legacy fixture conversion and equivalent V1 preview outputs. |
| 5.7 | `docs/SMART_CONTROLLER_PLAN.md` | Reconcile or supersede the V1 plan after migration is implemented and verified. |
| 5.8 | `plugins/README.md` | Update plugin description and verified capability boundaries if needed. |

Preserve V1 preview-only behaviour. Do not claim playback integration, durable arbitrary metadata persistence or document mutation unless the host contract is independently verified and separately approved.

**Phase 5 exit:** The unified plugin delegates core work to smart-controller-core; legacy data has tested migration; host-specific details remain in the adapter.

### Phase 6 — Authoring and advanced mapping

Add UI/authoring files only after inspecting the actual host UI/plugin extension mechanisms. Do not invent UI integration APIs.

Potential core additions, only when justified:

| Order | File | Required content |
|---:|---|---|
| 6.1 | `crates/smart-controller-core/src/mapping/triangulated.rs` | Arbitrary authored 2D pose triangulation and interpolation. |
| 6.2 | `crates/smart-controller-core/src/mapping/grid.rs` | Grid-based 2D pose interpolation. |
| 6.3 | `crates/smart-controller-core/tests/advanced_pose_tests.rs` | Triangulation, grid boundaries, degeneracy and continuity tests. |
| 6.4 | Host-supported authoring/UI files | Controller editing, pose capture, hierarchy, bindings, diagnostics and preview visualization, using verified host APIs. |
| 6.5 | `docs/SMART_CONTROLLERS_USER_GUIDE.md` | Animator-facing guide and examples after the authoring workflow exists. |

**Phase 6 exit:** Animators can author, inspect and reuse controllers through a verified host-supported workflow.

## 6. Test and verification policy

Every implementation PR must include tests for the behaviour it introduces.

Required test groups:

- model and schema validation
- golden serialization and migration
- scalar mapping and V1 compatibility
- 2D five-pose interpolation and boundaries
- deterministic topological ordering
- diamond dependency memoization
- self-loop and indirect cycle rejection
- missing, stale, non-finite and disabled input behaviour
- typed binding and all implemented composition modes
- plugin adapter parsing and capability failures
- Form Deform regression tests

Run the repository's documented Rust formatting, workspace tests and WASM build checks when implementation begins. Report actual command results; do not claim tests passed without execution. Host in-app smoke tests remain separate evidence.

## 7. Form Deform boundary

Keep Smart Controller binding evaluation and Form Deform pin evaluation independent.

The existing CCC-01.5 decision remains in force: define host-independent integration contracts if needed, but do not combine controller outputs with pin-deformation outputs until FDM v2 explicitly establishes multi-source composition semantics.

Do not alter the existing Form Deform deformation formula or behaviour as part of this Smart Controller plan.

## 8. PR sequence

1. **PR 1 — Canonical model and schema:** errors, identities, values, input/model types, graph representation, serialization and validation.
2. **PR 2 — Input and mapping:** scalar mapping consolidation, frame mapping and five-pose 2D mapping.
3. **PR 3 — DAG evaluator:** deterministic topological ordering, cycle protection, memoization, diagnostics and pure evaluation.
4. **PR 4 — Bindings and composition:** typed targets, output collection and explicit composition semantics.
5. **PR 5 — Plugin integration and migration:** thin host adapter, V1 migration and preview compatibility.
6. **PR 6A — Core mapping extensions:** triangulated and grid mapping without host UI dependencies.\n7. **PR 6B — Host authoring:** contingent on a separate, current host-capability audit.

Each PR must have a bounded scope, tests, documentation and acceptance evidence. Do not merge without explicit authorization.

## 9. Definition of completion

The system is complete when:

- one canonical model represents all controller behaviours
- inputs and mappings are reusable across scalar and 2D mechanisms
- nested controllers form a validated DAG
- shared dependencies are memoized once per invocation
- evaluation is pure and deterministic
- missing, stale, invalid and disabled input semantics are explicit
- typed bindings and composition rules are deterministic
- schema versioning and legacy migration are tested
- the plugin is a thin host integration layer
- V1 preview behaviour is preserved
- Form Deform remains independent

**Principle:** Build one controller engine with composable input, mapping, graph, evaluation and binding capabilities. Extend it through reusable abstractions rather than creating independent controller implementations.


## 10. Normative clarifications adopted before implementation

This section resolves underspecified contracts in the initial proposal. Where earlier sections use shorter or less precise wording, this section is authoritative. These are design decisions for the implementation plan, not authorization to change source code.

### 10.1 Typed pose and value interpolation

A `Pose2D` is an ordered collection of typed binding values, conceptually `Vec<(BindingId, TypedValue)>`. It is not an animation frame, host-state override or opaque trait object. Interpolation is **per binding/property**. Every participating pose must provide compatible values for the same binding identity; missing or incompatible values produce a typed mapping error.

The initial value algebra distinguishes finite scalars, required vector values and angles. Angle values carry explicit semantics equivalent to `Angle { radians: f64, wrapped: bool }`:

- Wrapped angles interpolate along the shortest angular path.
- Unwrapped angles interpolate linearly.
- Wrapped deltas use a documented half-open interval; exactly antipodal ties resolve in one fixed, documented direction.
- Unsupported value kinds or incompatible pose schemas return typed errors.

Composition follows the same angle semantics: wrapped-angle additive deltas normalize consistently; unwrapped angles remain ordinary scalar deltas. Never infer angle semantics from property names or host conventions.

### 10.2 Four-stage evaluation pipeline

Evaluation has four globally ordered stages:

```text
Input resolution
      |
      v
Node evaluation (topological order; canonical ControllerId tie-break)
      |
      v
Binding/contribution collection
      |
      v
Target composition (post-pass, after all nodes finish)
      |
      v
EvaluationResult
```

1. **Input resolution:** resolve and validate the complete input snapshot before evaluating any node. No node observes a partially resolved snapshot.
2. **Node evaluation:** evaluate enabled nodes in deterministic topological order. A disabled node emits no value and marks its outputs unavailable. A dependent node requiring such an output follows its declared fallback policy or receives a typed unavailable-input diagnostic/error. Never substitute implicit zero, neutral or cached values.
3. **Contribution collection:** collect successful typed binding contributions without composing targets during node traversal.
4. **Composition post-pass:** resolve target bases, validate all contributions and their modes, then compose each target in deterministic order.

Fatal graph/configuration failures abort evaluation. Recoverable input/binding failures are isolated to affected outputs where the contract permits; diagnostics identify the affected node, input, binding or target.

### 10.3 Contribution model and mode conflicts

All modes consume one common record, conceptually:

```rust
struct Contribution {
    binding_id: BindingId,
    target: TargetId,
    value: TypedValue,
    mode: CompositionMode,
    weight: Option<f64>,
    priority: Option<i64>,
    base_ref: Option<BaseRef>,
}
```

Mode-irrelevant fields must be absent and are validated as such. Contributions to one target must all assert the same mode; mixed modes return typed `CompositionModeConflict`. Never select a mode by traversal order or silently coerce contributions.

The canonical priority mode name is `PriorityOverride`, selecting the highest explicit numeric priority with canonical BindingId as the deterministic tie-breaker. Earlier use of `Priority` is shorthand only and must not become the serialized schema name.

Additive contributions identify an explicit base/reference contract and accumulate in canonical BindingId order. WeightedBlend eligibility requires a successfully produced valid value and a finite weight greater than zero. Zero-weight contributions are excluded without error. Disabled/unavailable contributions are excluded and diagnosed where relevant. Negative or non-finite weights are typed errors. If no eligible contributions remain, emit no target value and attach a stable diagnostic; do not invent zero or a base value.

Multiple Replace contributions to one target must use an explicitly declared deterministic precedence policy or return a typed conflict. Prefer the conflict error initially unless a separate policy is approved.

### 10.4 Snapshot freshness

Freshness is part of the input snapshot, not a hidden adapter setting or wall-clock lookup. Each input sample carries monotonic `age_ticks: u64`; the snapshot carries `max_age_ticks: u64`. Both use the same host-provided tick domain. A sample is stale exactly when `age_ticks > max_age_ticks`; equality is fresh. The adapter derives age from verified host sequencing/tick information; the core never reads time.

If the host cannot provide meaningful age, freshness must be represented explicitly as unknown/unavailable. The adapter must not fabricate a tick value or silently mark unknown data fresh; the configured fallback/unavailable policy applies.

### 10.5 Floating-point determinism

Sort contributions by canonical BindingId before Additive or WeightedBlend accumulation, and process targets by canonical TargetId. Reject NaN and infinity at every public numeric boundary. Tests use the initial combined absolute/relative tolerance:

`abs(actual - expected) <= 1e-9 * max(1, abs(actual), abs(expected))`.

This is a comparison rule, not permission to reorder arithmetic or conceal unstable algorithms. Identical inputs must follow the same operation order.

### 10.6 Stable diagnostic registry

Phase 1 establishes the public diagnostic-code registry before evaluator implementation. Codes are stable identifiers independent of human-readable messages and are covered by a checked-in fixture, for example `crates/smart-controller-core/tests/fixtures/diagnostic_codes_v1.json`. Cover missing/stale/invalid/unavailable inputs, graph invalidity/cycles, mapping domain/type errors, composition conflicts/invalid weights/no eligible contributions, and skipped/disabled outputs. Adding codes is compatible; changing an existing code's meaning requires an explicit API/schema decision.

### 10.7 Shared-node validation and cycle defense

A canonical node may have multiple incoming parent edges, but it cannot contain parent-scoped configuration. Parent-specific behaviour belongs in explicit input contexts or separate canonical nodes. Validation rejects parent-scoped configuration on a node referenced by multiple parents.

The production graph builder validates edges and cycles. The evaluator independently defends against cycles. Tests construct malformed graphs through a `#[cfg(test)]` unchecked builder; no unchecked production API is exposed.

### 10.8 No-op and unavailable outcomes

These are valid documented outcomes, not fatal errors:

- Empty graph: empty outputs and diagnostics.
- All controllers disabled: empty outputs plus disabled/skipped diagnostics.
- Target with no eligible contributions: no output for that target plus a no-eligible-contribution diagnostic.

A dependent node requiring an unavailable upstream value follows its explicit fallback policy or reports unavailable required input. No implicit zero, neutral or last-value substitution is allowed.

### 10.9 Core portability and Form Deform independence

`smart-controller-core` must build for the repository's supported WASM target and must not use wall-clock APIs, random sources, filesystem/environment/process APIs, network access or host SDK commands. Keep dependencies minimal and audited; initially allow only required serialization/error/value crates (for example `serde` and `thiserror`) plus Rust standard/core facilities. Any new dependency requires portability, license and necessity review.

CI includes a dependency-boundary check proving neither `smart-controller-core` nor `form-deform-core` depends on the other, plus the WASM build check. This separation is a hard architectural boundary.

### 10.10 Phase sequencing and acceptance corrections

- **Phase 0:** add executable V1 characterization tests against captured fixtures and current preview/binding outputs. Fixtures alone do not prove compatibility. Record expected outputs before changing public models.
- **Phase 1:** establish typed values/angles, Pose2D's value contract, schema, diagnostic registry, legacy-to-canonical converters and round-trip/loss diagnostics for both legacy formats. Move pure `range.rs` and `frame.rs` primitives into Phase 1 to prove V1 scalar/frame equivalence early.
- **Phase 2:** implement Pose2D/joystick mapping using the typed interpolation contracts established in Phase 1.
- **Phase 3:** implement the four-stage evaluator, contribution collection, unchecked-cycle defense and no-op cases. Composition remains a separate post-pass.
- **Phase 4:** complete target composition and conflict handling using the common contribution record.
- **Phase 5:** the host adapter wires already-tested migration and canonical evaluation into the host boundary; it does not invent migration rules.
- **Phase 6A:** core-only triangulated/grid mapping extensions, independently testable without host UI.
- **Phase 6B:** host authoring/UI work is deferred and contingent on a fresh, concrete host-capability audit.

The V1 characterization suite covers legacy three-anchor interpolation, easing, targets/modes, disabled bindings, BindingSet serialization, Slider endpoints, reversed ranges, and manual-input and bone-query parameter paths. Migration tests prove fixture → canonical representation → legacy-compatible representation where lossless and diagnose every non-round-trippable field.

Required test files include `v1_characterization_tests.rs`, `migration_roundtrip_tests.rs`, `angle_interpolation_tests.rs`, `pose2d_tests.rs`, `composition_conflict_tests.rs`, `freshness_tests.rs`, `diagnostic_registry_tests.rs`, `graph_tests.rs` (including unchecked-cycle defence), `no_op_graph_tests.rs` and `dependency_boundary_tests.rs`.

The implementation sequence is therefore: Phase 0 characterization; Phase 1 model/schema/diagnostics/migration plus pure range/frame mapping; Phase 2 pose mapping; Phase 3 graph/evaluation; Phase 4 composition; Phase 5 adapter integration; Phase 6A core mapping extensions; Phase 6B host authoring only after capability verification.
