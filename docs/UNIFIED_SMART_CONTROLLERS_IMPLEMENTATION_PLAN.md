# Unified Smart Controllers — Architecture and Implementation Plan

**Status:** Phase 4 complete; Phase 5 adapter integration started  
**Target repository:** `Mujshanan/forma-deform`  
**Branch:** `feat/smart-controller-phase3-evaluator`  
**Core:** `smart-controller-core`  
**Host adapter:** `plugins/smart-controller`  
**Scope:** Architecture, implementation sequence and file-level build order  
**Execution status:** Phase 1 is merged. Phase 3 evaluator hardening passed CI. Phase 4 composition is complete and verified at `f08185f043141ac3f476bd2301dbcabd72d9bebb`; final CI run: https://github.com/Mujshanan/forma-deform/actions/runs/44931731464. Phase 5 host-adapter foundations are now being implemented on the branch above.

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
- **PriorityOverride**

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

### Phase 0 — Audit, fixture verification and V1 characterization (documentation/tests first)

| Order | File | Action | Required content |
|---:|---|---|---|
| 0.1 | `docs/SMART_CONTROLLER_PLAN.md` | Inspect; later reconcile | Existing V1 scope, endpoint policy, host boundaries and deferred features. Preserve its useful V1 facts; identify obsolete assumptions. |
| 0.2 | `crates/smart-controller-core/src/legacy.rs` | Inspect | Legacy three-anchor API, stable IDs, interpolation, absolute/relative modes, targets, validation and callers. |
| 0.3 | `crates/smart-controller-core/src/model.rs` | Inspect | Newer Params/Mode model, host identifiers, configuration overlap and migration needs. |
| 0.4 | `crates/smart-controller-core/src/mapping.rs` | Inspect | Rotation normalization, reversed ranges, frame endpoint policies, transform mapping and reserved semantics. |
| 0.5 | `crates/smart-controller-core/src/preview.rs` | Inspect | Existing pure preview functions, output types and V1 limitations. |
| 0.6 | `crates/smart-controller-core/src/validation.rs` | Inspect | Existing validation, error representation and missing edge cases. |
| 0.7 | `crates/smart-controller-core/src/lib.rs` | Inspect | Public exports, compatibility surface and module layout. |
| 0.8 | `crates/smart-controller-core/tests/v1_characterization_tests.rs` | Characterize existing V1 behavior | Deserialize and round-trip all three captured fixtures; assert legacy binding outputs, easing, absolute/relative modes, disabled behavior, Slider host/manual input paths, endpoint policies and reversed ranges. |
| 0.9 | `plugins/smart-controller/src/lib.rs` | Inspect | SDK entry point, bone query parsing, manual input, preview-only behaviour and host assumptions. |
| 0.10 | `crates/anicraft-plugin-sdk/*` | Inspect | Verified query/command contracts, error behaviour and adapter support. |
| 0.11 | `docs/SMART_CONTROLLERS_AUDIT.md` | Add | File inventory, reuse map, overlapping models, compatibility matrix, host capability evidence, test gaps and migration risks. |

**Phase 0 exit:** Met at `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`: the existing implementation and host boundary are documented; golden fixtures are captured; the executable V1 characterization suite passes; no existing runtime semantics are removed. Formatting and Clippy failures are documented as pre-existing baseline issues, not Phase 0 exit gates.

### Phase 1 — Canonical types, schema, diagnostics, migration and scalar mapping

Create or adapt files in this order:

| Order | File | Required content |
|---:|---|---|
| 1.1 | `crates/smart-controller-core/src/error.rs` | Typed model, validation, mapping, graph, composition, serialization and evaluation errors. Errors must be structured enough for tests and plugin diagnostics. |
| 1.2 | `crates/smart-controller-core/src/ids.rs` | Strongly typed ControllerId, InputId, BindingId and target identifiers; stable ordering and serialization rules. |
| 1.3 | `crates/smart-controller-core/src/value.rs` | Finite scalar/vector values and explicit angle kind (`wrapped` or `unwrapped`); type compatibility and finite-value validation. |
| 1.4 | `crates/smart-controller-core/src/input.rs` | InputSource definitions, dimensions, explicit validity, per-sample `age_ticks`, snapshot `max_age_ticks`, fallback policy and snapshot structures. No host API types. |
| 1.5 | `crates/smart-controller-core/src/model.rs` | Canonical Controller, mapping configuration references, nested dependency references, enabled state, bindings and composition configuration. Replace competing model directions only after conversion tests exist. |
| 1.6 | `crates/smart-controller-core/src/graph.rs` | ControllerGraph storage, dependency extraction, graph integrity checks and deterministic graph representation. |
| 1.7 | `crates/smart-controller-core/src/serialization.rs` | Schema envelope, version dispatch, versioned decode/encode interfaces and migration registry. |
| 1.8 | `crates/smart-controller-core/src/validation.rs` | Canonical model validation, IDs, finite values, graph references, mapping compatibility, binding types, parent-scoped configuration rejection on shared nodes and unsupported-feature errors. |
| 1.9 | `crates/smart-controller-core/src/lib.rs` | Public module declarations and intentionally supported API re-exports. Keep implementation details private unless needed by consumers. |
| 1.10 | `crates/smart-controller-core/tests/model_tests.rs` | Model construction, identity, validation, graph references and type compatibility tests. |
| 1.11 | `crates/smart-controller-core/tests/serialization_tests.rs` | Golden fixtures, round trips, unsupported versions and migration tests. |
| 1.12 | `crates/smart-controller-core/tests/fixtures/*` | Canonical schema examples, legacy V1 inputs and `diagnostic_codes_v1.json`. |
| 1.13 | `crates/smart-controller-core/src/diagnostic_codes.rs` | Stable public diagnostic-code registry; codes are independent of display messages. |
| 1.14 | `crates/smart-controller-core/src/migration.rs` | BindingSet v1 and unversioned Params conversion to canonical data, with explicit loss/unsupported-field diagnostics. |
| 1.15 | `crates/smart-controller-core/src/mapping/range.rs` | Pure scalar/range mapping primitives, preserving legacy three-anchor semantics. |
| 1.16 | `crates/smart-controller-core/src/mapping/easing.rs` | Pure Linear/Smoothstep easing primitives and compatibility checks. |
| 1.17 | `crates/smart-controller-core/src/mapping/frame.rs` | Pure inclusive/exclusive frame mapping, preserving V1 endpoint behaviour. |
| 1.18 | `crates/smart-controller-core/tests/migration_roundtrip_tests.rs` | Fixture → canonical → legacy-compatible round trips where lossless; assert diagnostics for lossy conversions. |
| 1.19 | `crates/smart-controller-core/tests/diagnostic_registry_tests.rs` | Validate unique stable codes and exact registry fixture coverage. |
| 1.20 | `crates/smart-controller-core/tests/angle_interpolation_tests.rs` | Wrapped shortest-path, unwrapped linear, antipodal tie and additive-angle semantics. |
| 1.21 | `crates/smart-controller-core/tests/range_mapping_tests.rs` | Reversed ranges, endpoints, clamping, easing and frame mapping V1 regression tests. |
| 1.22 | `crates/smart-controller-core/src/contribution.rs` | Public typed Contribution record and mode-specific field invariants; composition execution remains Phase 4. |
| 1.23 | `crates/smart-controller-core/tests/dependency_boundary_tests.rs` | Verify Smart Controller core and Form Deform core remain mutually independent. |

**Phase 1 exit:** One authoritative model represents all controller behaviours, scalar/2D inputs, nested dependencies and multiple bindings. Schema, migration, diagnostic codes, contribution shape and pure scalar/frame mapping are explicit and covered by compatibility tests.

### Phase 2 — Input contracts and pose mapping

| Order | File | Required content |
|---:|---|---|
| 2.1 | `crates/smart-controller-core/src/input.rs` | Complete snapshot lookup, validation of supplied sample status and fallback resolution contracts. |
| 2.2 | `crates/smart-controller-core/src/mapping/mod.rs` | Shared Mapping1D/Mapping2D contracts, mapping configuration dispatch and common output types. |
| 2.3 | `crates/smart-controller-core/src/mapping/pose1d.rs` | One-dimensional authored pose interpolation and explicit endpoint behaviour. |
| 2.4 | `crates/smart-controller-core/src/mapping/pose2d.rs` | Typed Pose2D per-binding values, five-pose regions, triangle selection, barycentric interpolation and non-uniform pose handling. |
| 2.5 | `crates/smart-controller-core/src/mapping/boundary.rs` | Unit-square clamp, radial unit-circle clamp and reject policies. |
| 2.6 | `crates/smart-controller-core/tests/pose2d_tests.rs` | Center/cardinal exactness, triangle edges, continuity, non-uniform poses, per-binding typed values and boundary policies. |

The exact module split may be adjusted after Phase 0. Do not retain duplicate scalar mapping implementations merely because their original files differ.

**Phase 2 exit:** Slider and joystick use the same mapping infrastructure; typed pose interpolation and angle semantics are tested; bone-driven and manual inputs resolve through the same input contract.

### Phase 3 — Pure DAG evaluation

| Order | File | Required content |
|---:|---|---|
| 3.1 | `crates/smart-controller-core/src/graph.rs` | Deterministic topological ordering, canonical-ID tie-breaks, self-loop and cycle detection. |
| 3.2 | `crates/smart-controller-core/src/diagnostic.rs` | Structured warnings and diagnostic instances using the Phase 1 stable code registry. |
| 3.3 | `crates/smart-controller-core/src/evaluation.rs` | Four-stage pipeline: resolve complete snapshot; evaluate nodes; collect contributions; post-pass composition. Per-invocation memoization and explicit unavailable propagation. |
| 3.4 | `crates/smart-controller-core/src/lib.rs` | Public evaluator and result exports after API review. |
| 3.5 | `crates/smart-controller-core/tests/graph_tests.rs` | Chains, independent nodes, diamond DAG, deterministic ties, self-loop, indirect cycle and evaluator-side unchecked-cycle defence. |
| 3.6 | `crates/smart-controller-core/tests/evaluation_tests.rs` | Memoization, identical-snapshot determinism, disabled/unavailable dependencies, missing/stale/NaN inputs, fallback and diagnostics. |
| 3.7 | `crates/smart-controller-core/tests/no_op_graph_tests.rs` | Empty graph and all-disabled graph; zero-eligible-contribution targets are covered by Phase 4 composition tests. |

Evaluation memoization is local to one call. No global caches, last-value state, wall-clock reads or random values are permitted.

**Phase 3 exit:** Valid graphs follow the four-stage evaluation pipeline deterministically; shared nodes are evaluated once per invocation; invalid cycles fail safely; empty and all-disabled graph outcomes are documented.

### Phase 4 — Bindings and composition

| Order | File | Required content |
|---:|---|---|
| 4.1 | `crates/smart-controller-core/src/binding.rs` | Host-independent target identity, property identity, typed output and binding-level configuration. |
| 4.2 | `crates/smart-controller-core/src/composition.rs` | Replace, Additive, WeightedBlend and PriorityOverride contracts; deterministic ordering; explicit base references; conflict and unsupported-mode errors. |
| 4.3 | `crates/smart-controller-core/src/evaluation.rs` | Integrate evaluated controller values with binding collection and final target output generation without host mutation. |
| 4.4 | `crates/smart-controller-core/tests/composition_tests.rs` | All modes, unsupported modes, zero weights, equal priority, conflicting Replace, additive bases and type mismatch. |
| 4.5 | `crates/smart-controller-core/tests/binding_tests.rs` | Multiple outputs, multiple contributors, target identity and unavailable target cases. |

**Phase 4 exit:** Outputs are typed and deterministic; composition-mode conflicts, Replace conflicts, invalid weights and no-eligible-contribution outcomes are explicit; accumulation never relies on incidental iteration order or silent fallback.

### Phase 5 — Plugin adapter and compatibility integration

| Order | File | Required content |
|---:|---|---|
| 5.1 | `plugins/smart-controller/src/adapter.rs` | Host-specific bone/input queries, source identity resolution and conversion to core snapshots using verified SDK commands only. |
| 5.2 | `plugins/smart-controller/src/targets.rs` | Host target identity resolution and explicit capability checks; no speculative write or playback commands. |
| 5.3 | `plugins/smart-controller/src/compatibility.rs` | Adapter wiring to the already-tested core migration API; preserve request compatibility and surface conversion diagnostics. |
| 5.4 | `plugins/smart-controller/src/lib.rs` | Thin plugin entry point: parse request, call adapter, invoke core evaluator, serialize/log preview and diagnostics. |
| 5.5 | `plugins/smart-controller/tests/adapter_tests.rs` | Host response parsing, missing bones, malformed response and capability failures. |
| 5.6 | `plugins/smart-controller/tests/compatibility_tests.rs` | Verify adapter-level legacy request conversion and equivalent V1 preview outputs; core migration round trips are already tested in Phase 1. |
| 5.7 | `docs/SMART_CONTROLLER_PLAN.md` | Reconcile or supersede the V1 plan after migration is implemented and verified. |
| 5.8 | `plugins/README.md` | Update plugin description and verified capability boundaries if needed. |

Preserve V1 preview-only behaviour. Do not claim playback integration, durable arbitrary metadata persistence or document mutation unless the host contract is independently verified and separately approved.

**Phase 5 exit:** The unified plugin delegates core work to smart-controller-core; legacy data has tested migration; host-specific details remain in the adapter.

### Phase 6A — Core mapping extensions; Phase 6B — contingent host authoring

Phase 6A is core-only and requires no host UI capability. Phase 6B authoring is deferred until a separate host-capability audit verifies the required extension mechanisms. Do not invent UI integration APIs.

Potential core additions, only when justified:

| Order | File | Required content |
|---:|---|---|
| 6.1 | `crates/smart-controller-core/src/mapping/triangulated.rs` | Arbitrary authored 2D pose triangulation and interpolation. |
| 6.2 | `crates/smart-controller-core/src/mapping/grid.rs` | Grid-based 2D pose interpolation. |
| 6.3 | `crates/smart-controller-core/tests/advanced_pose_tests.rs` | Triangulation, grid boundaries, degeneracy and continuity tests. |
| 6.4 | Host-supported authoring/UI files (Phase 6B only) | Controller editing, pose capture, hierarchy, bindings, diagnostics and preview visualization; contingent on a separate host-capability audit and verified APIs. |
| 6.5 | `docs/SMART_CONTROLLERS_USER_GUIDE.md` (Phase 6B only) | Animator-facing guide and examples after a verified authoring workflow exists. |

**Phase 6A exit:** Core triangulated/grid mapping is independently tested and host-independent.

**Phase 6B exit:** Only after a separate host-capability audit, animators can author, inspect and reuse controllers through a verified host-supported workflow.

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

1. **PR 1 — Characterization and canonical contracts:** V1 output baselines, typed model/schema, diagnostics, migration converters and pure range/frame mapping.
2. **PR 2 — Pose mapping:** typed Pose2D, angle interpolation, joystick regions and boundaries.
3. **PR 3 — DAG evaluator:** four-stage evaluation, deterministic ordering, cycle protection, memoization and diagnostics.
4. **PR 4 — Bindings and composition:** common contributions, conflict handling and target composition post-pass.
5. **PR 5 — Plugin compatibility integration:** thin adapter wiring to tested core migration/evaluation, preserving V1 preview behaviour.
6. **PR 6A — Core mapping extensions:** triangulated and grid mapping without host UI dependencies.
7. **PR 6B — Host authoring:** contingent on a separate, current host-capability audit.

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

## 11. Phase 1 implementation log

### 11.1 Foundation increment

**Branch:** `feat/smart-controller-phase1-foundation`  
**Base:** `17e58fd75b585f77fdc7f10758c47c64c58d0e0c`

Implemented the first four Phase 1 responsibilities without replacing the existing V1 models:

- `error.rs`: structured `CoreError` variants for identifier/value/input/model/mapping/graph/composition/schema/serialization/evaluation failures.
- `ids.rs`: strongly typed, ordered, hashable `ControllerId`, `InputId`, `BindingId` and `TargetId`; string serialization and validated deserialization.
- `value.rs`: finite `Scalar`, `Vector2`, explicit `Angle { radians, wrapped }` semantics, and the `TypedValue` union. Numeric deserialization rejects non-finite values.
- `input.rs`: host-independent manual/bone/controller-output sources, dimensions, explicit fallback and validity policies, per-sample optional monotonic age, snapshot maximum age, duplicate-ID checks, type checks and strict stale comparison (`age_ticks > max_age_ticks`).
- `lib.rs`: exports the new foundation while retaining the legacy and Params APIs.
- `tests/foundation_types_tests.rs`: adds tests for identifiers, finite values, angle serialization, input identity/dimensions, snapshot validation, freshness boundaries and unknown freshness.

**Compatibility boundary:** Existing `legacy.rs`, `model.rs`, `mapping.rs`, `preview.rs`, `validation.rs` and host adapter runtime behavior have not been replaced or changed by this increment.

**Verification:** The Phase 0 results were supplied for base commit `17e58fd`. The newly added Phase 1 foundation tests have not yet been run with Cargo in this implementation environment. Do not treat this increment as test-passing until `cargo test -p smart-controller-core` and the workspace checks are rerun on this branch.

**Next Phase 1 work:** add canonical model/schema and conversion tests before replacing competing models; then implement the diagnostic registry, migration and pure scalar/range/frame mapping in the approved order.


### 11.2 Canonical contracts increment (implementation in progress)

Added without removing the V1 Params or legacy.rs APIs:

- canonical.rs: one host-independent controller shape with input references, dependency references, mapping configuration variants, bindings, enabled state and a parent-scoped marker. Local validation rejects malformed ranges, duplicate binding IDs, invalid joystick pose IDs and additive bindings without an explicit base.
- graph.rs: validated graph storage, missing-dependency checks, shared-node parent-scope rejection, cycle detection and deterministic topological order with canonical ID tie-breaking.
- schema.rs: explicit smart-controller schema identity and version-1 envelope.
- diagnostic_codes.rs plus tests/fixtures/diagnostic_codes_v1.json: initial stable diagnostic code registry.
- contribution.rs: typed contribution shape and mode-specific field validation. Composition execution remains Phase 4.
- mapping/range.rs, mapping/easing.rs, mapping/frame.rs: pure range/easing/frame primitives. Existing mapping.rs functions remain intact for compatibility.
- Tests added: diagnostic registry fixture, graph ordering/invalid graph, range/frame/easing, schema identity and contribution field invariants.

**Important incomplete work:** No legacy BindingSet/Params migration registry or loss-reporting converters have been implemented yet. Canonical model serialization dispatch and golden canonical model round-trip coverage are also outstanding. The canonical types are additive alongside the legacy models; the competing APIs have deliberately not yet been replaced.

**Execution:** No Phase 1 Cargo tests have been run in this environment. Treat every newly authored test as pending until CI or a Rust toolchain executes them. Phase 1 exit is not met.


### 11.3 Migration, diagnostics and mapping-contract increment

Added the following Phase 1 work on this branch:

- migration.rs now migrates versioned BindingSet V1 and unversioned Params JSON into the canonical document. migrate_json dispatches by the legacy document shape and uses strict deserialization.
- Migration outputs include a structured report with notices and explicit losses. Unversioned Params receives deterministic migration IDs, which are called out as migration identities rather than durable authored identities.
- export_binding_set_v1 and export_unversioned_params provide reverse conversion only when the canonical document retains the exact legacy-representable shape. Unsupported or altered canonical data is rejected rather than silently discarded.
- BindingSet V1 → canonical → BindingSet V1 round-trip tests assert exact equality. Params → canonical → Params tests cover slider, transform, absent optional endpoint policy and bone-driven input identities.
- Canonical JSON now has validated to_json/from_json entry points, schema-version checks, unknown-field rejection and round-trip tests.
- diagnostic_codes.rs now exposes structured diagnostics with registered stable codes, severity, explanatory message and optional controller/input/target context.
- Canonical mapping types no longer depend on legacy V1 types. Legacy mapping data is translated into canonical-owned range/easing types.
- Pure range/frame/easing functions reject non-finite and overflowing public inputs. New range/frame parity tests compare the scalar/frame results against the characterized V1 functions.
- Graph validation rejects duplicate dependency edges; tests cover shared-node parent-scoped configuration.

**Phase 1 implementation and CI gates:** the dependency-boundary test, Pose2D value contract, wrapped/unwrapped angle interpolation, and typed-value interpolation contracts have now been added. The full CI workflow passed on commit d2536c09c7ff32d25bc4d5eed1cd41807ab31195. The CI workflow does not run rustfmt or Clippy; those are not claimed as passing. No competing V1 API has been removed.


The explicit dependency-boundary test is now present at crates/smart-controller-core/tests/dependency_boundary_tests.rs. It checks both crate manifests (including normal, dev, build and target-specific dependency sections) and fails if either core directly depends on the other.


## 12. Phase 3 evaluator orchestration — in progress

**Branch:** `feat/smart-controller-phase3-evaluator`  
**Base:** Phase 1 squash merge `9e5b57f08fbd6f5b25de8ec1bb401692b1e4749b`

The first Phase 3 increment adds `evaluator.rs` and exports the module from `smart-controller-core`.

Implemented in this increment:

- A host-independent `ControllerMapper` boundary so the evaluator owns orchestration while mapping algorithms remain independently implemented.
- Full preflight of the graph, controller-local invariants, input definitions, snapshot samples, input references, and controller-output dependency declarations before the first mapper call.
- Explicit external input resolution with stale/missing/invalid/unavailable diagnostics and the configured `UseValue`, `Unavailable`, or `Reject` fallback policy.
- Controller-output input resolution from successful upstream binding outputs only. Disabled or skipped producer outputs are unavailable; no cached or fabricated value is substituted.
- Deterministic topological node evaluation with a second evaluator-side cycle check.
- Binding contribution collection with target/value compatibility validation. Absolute-pose bindings currently produce Replace contributions; relative-delta bindings produce Additive contributions with an explicit authored base reference.
- No target composition or host mutation. The result is an uncomposed contribution list for the separate Phase 4 post-pass.
- Empty-graph and all-disabled no-op behavior, unavailable dependency propagation, explicit fallback use, stale input fallback, full-snapshot preflight, and unchecked-cycle defense tests.

**Important scope boundary:** Phase 2's concrete Pose2D/joystick mapping algorithms have not been implemented as part of this increment. Phase 3 currently receives mapping results through `ControllerMapper`; a production mapper must be supplied by the mapping layer. Unsupported mapping configurations must not be treated as a successful empty output. The trait boundary allows Phase 3 orchestration to be developed and tested without prematurely duplicating Phase 2 mapping semantics.

**Remaining beyond this increment:** a concrete mapping adapter backed by the separately specified mapping algorithms; Phase 4 target composition; and host integration (which remains outside the core evaluator). The mapper receives all declared inputs as a typed, ID-keyed collection. Controller-local mapping-domain and non-finite calculation failures are diagnosed and invalidate that controller's outputs; unexpected mapper contract/internal errors abort the evaluation. The latest Phase 3A/3B commit is CI-verified as recorded below.

**Preserved boundaries:** Smart Controller V1 remains preview-only. The evaluator is pure and host-independent. CCC-01.5 remains unchanged: no Smart Controller output is combined with Form Deform pin-deformation evaluation.


### 12.1 Phase 3A/3B hardening and verification

**Verified commit:** `1eaea3fd92752ce557af77c3c2c151045d724a28`  
**CI run:** https://github.com/Mujshanan/forma-deform/actions/runs/44927336954

Hardening completed:

- Complete structural preflight validates all controller nodes, all input definitions and references, all supplied snapshot samples against their declared dimensions, and rejects externally injected samples for controller-output inputs.
- Disabled controllers do not require runtime samples and do not invoke the mapper, while their graph/configuration references remain structurally validated.
- `FallbackPolicy::Reject` consistently aborts when a required external or controller-output input is unavailable. `Unavailable` skips the consumer with diagnostics; `UseValue` uses only its explicitly configured, dimension-validated fallback.
- `InvalidMapping` and `NonFiniteValue` from a mapper are controller-local failures: all that controller's outputs become unavailable, diagnostics are emitted, and independent branches can continue. Unexpected mapper errors and unknown emitted BindingIds remain evaluation errors.
- Missing or target-incompatible binding outputs are marked unavailable and diagnosed. Contributions are collected but never target-composed.
- Determinism test compares equivalent graphs with different insertion orders and snapshots with different sample orders.
- Added coverage for mapping failure propagation, Reject and explicit fallback behavior, non-finite mapping errors, unknown output bindings, unused-sample type validation, controller-output snapshot injection, empty/all-disabled graphs, stale input fallback, and cycle defense.

**Evaluator-specific tests:** 18 authored and executed successfully.  
**Workspace verification:** `cargo test --workspace` passed with 135 tests total. Bun typecheck, Bun tests (9/9), and Bun build passed. Release WASM builds passed for bone-query, form-deform, and smart-controller plugins. The complete GitHub Actions job concluded successfully.

**Not claimed:** rustfmt and Clippy were not run by this workflow. No Phase 3 PR or merge has been created. V1 remains preview-only; CCC-01.5 and Form Deform evaluation independence are unchanged.


## 13. Phase 4 implementation log

### 13.1 Composition core and binding policy

**Branch:** `feat/smart-controller-phase3-evaluator`  
**Base:** verified Phase 3 implementation `c83182d8c5039977baaf0b8fcac2317066368a09`  
**Verified implementation commit:** `da644fa1bf91aab74a716524895145375595547f`  
**CI run:** https://github.com/Mujshanan/forma-deform/actions/runs/44929171315  
**Status:** initial composition implementation and tests passed CI; remaining review and hardening continue.

Implemented in this increment:

- Added a pure `composition.rs` post-pass that groups contributions by canonical TargetId and processes targets in BTreeMap order.
- Added explicit source identity to each contribution: ControllerId plus BindingId. Since binding IDs are only unique within one controller, deterministic contribution ordering is `(BindingId, ControllerId)`.
- Added an explicit authored base value to additive contributions. Existing RelativeDelta bindings preserve their legacy default: Additive with BaseRef::Authored and their configured base value.
- Added optional per-binding CompositionConfig while retaining old canonical documents through a serde-defaulted optional field. Existing AbsolutePose bindings default to Replace; RelativeDelta bindings default to Additive(Authored).
- Canonical bindings can now explicitly select Replace, Additive, WeightedBlend or PriorityOverride, with validation of mode-specific fields and compatibility with AbsolutePose/RelativeDelta.
- Additive resolves exactly one explicit base reference (Authored, Neutral target base, or named input base), requires all contributions to agree on that reference and base value, and accumulates in canonical contribution order.
- WeightedBlend normalizes positive weights; zero weights are excluded. Negative/non-finite weights and overflow are typed errors. If all weights are zero, no target value is emitted and the stable no-eligible diagnostic is returned.
- PriorityOverride selects the highest numeric priority. Equal priorities select the lexicographically smallest BindingId, then ControllerId as a necessary deterministic secondary key.
- Multiple Replace contributions, mixed composition modes, missing additive bases, incompatible types and ambiguous wrapped-angle weighted means return typed composition conflicts.
- Scalar and Vector2 additive/blend paths are implemented. Additive wrapped angles normalize to [-π, π); weighted wrapped angles use a circular mean and reject the undefined opposing-direction case.
- Added `evaluate_and_compose` as a pure orchestration convenience: it first runs the existing Phase 3 evaluator, then the composition post-pass. It does not read or mutate host state.

**Tests executed:** 15 composition-module tests plus one canonical binding composition-validation test. The successful workspace workflow reports 151 Rust tests passed, Bun typecheck/tests (9/9)/build passed, and release WASM builds passed for bone-query, form-deform and smart-controller plugins. No compiler warnings or errors appeared in the successful job log.

**Not verified:** Rustfmt and Clippy were not run by this workflow. Host in-app smoke tests remain separate. No PR or merge has been created.

**Preserved boundaries:** composition consumes only Smart Controller contributions. It does not compose with Form Deform pin outputs, change the Form Deform formula, or make V1 playback-capable.


### 13.2 Phase 4 typed-angle target compatibility and final verification

**Verified commit:** `f08185f043141ac3f476bd2301dbcabd72d9bebb`  
**CI run:** https://github.com/Mujshanan/forma-deform/actions/runs/44931731464

A post-pass review found that the evaluator's target compatibility gate allowed only Scalar values for Property targets, which prevented Angle values from reaching the already angle-aware composition layer. Property targets now accept Scalar or Angle values; SymbolFrame and FrameRange remain Scalar, and GeometryDeform remains Vector2. Authored additive Property bases may likewise be Scalar or Angle, with exact value-type compatibility enforced during composition.

Added an end-to-end `evaluate_and_compose` regression test proving an Angle value survives mapping output validation and weighted composition. The first run exposed a boundary-normalization issue at exactly +π; wrapped weighted-angle results are now normalized to the established [-π, π) interval.

**Final verification:** 153 workspace Rust tests passed; Bun typecheck, Bun tests (9/9), Bun build, and release WASM builds for bone-query, form-deform, and smart-controller all passed. The workflow completed successfully. Rustfmt and Clippy are not included in this workflow and are not claimed as passing.

Phase 4 remains core-only. No host state is mutated; no Smart Controller contribution is combined with Form Deform pin-deformation output; V1 remains preview-only. No PR or merge has been created.


## 16. Phase 4 completion record

Phase 4 is complete at `f08185f043141ac3f476bd2301dbcabd72d9bebb`.

- Deterministic composition post-pass for Replace, Additive, WeightedBlend and PriorityOverride.
- Explicit additive bases, positive-weight normalization, zero-eligible diagnostics, priority tie-breaking and typed conflicts.
- Target-specific typed-value acceptance, including Scalar/Angle Property outputs and Vector2 GeometryDeform outputs.
- Wrapped weighted-angle outputs normalized to the documented half-open interval.
- End-to-end evaluator-plus-composition coverage.
- CI run https://github.com/Mujshanan/forma-deform/actions/runs/44931731464 passed: 153 workspace Rust tests, Bun typecheck, 9/9 Bun tests, Bun build and all three release WASM plugin builds.
- Rustfmt/Clippy were not part of this run. No PR or merge was created.
- Composition remains a pure core operation. It does not mutate the host or combine Smart Controller output with Form Deform pin evaluation.

## 17. Phase 5 adapter integration — started, not complete

Initial adapter foundations are being added:

- `plugins/smart-controller/src/adapter.rs`: parses the verified read-only `bone.info` response by stable numeric bone ID; converts degrees to radians; makes spring-swing inclusion and wrapped/unwrapped angle semantics explicit; produces same-invocation input samples without reading a clock.
- `plugins/smart-controller/src/targets.rs`: assesses host target capabilities without issuing mutating commands, queries the documented read-only `warp.info` endpoint, and verifies geometry targets against numeric element/pin IDs rather than array positions. Geometry pin application is marked command-preview-only (`warp.pin_move`); symbol-frame, frame-range and generic property targets remain calculation-only.
- `plugins/smart-controller/src/compatibility.rs`: delegates legacy BindingSet and unversioned Params decoding/migration to the core migration API and preserves MigrationReport.
- The plugin entry point registers these modules; its existing V1 Slider preview path is unchanged.

**Important remaining work:** Phase 5 is not complete. The adapter still needs full request orchestration, canonical evaluation wiring, target-specific preview planning, adapter-level compatibility regression tests, and documentation reconciliation. The Phase 2 concrete pose/joystick mapper remains a separate prerequisite for general canonical mapping; this increment must not pretend the injected evaluator mapper is a complete production mapper. No mutating target command, playback hook, durable metadata API or Form Deform composition is introduced.
