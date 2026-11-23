# CQ1 — Architecture and Module Responsibility Map

**Status:** Initial architecture review complete; findings are proposals for review, not refactoring instructions.  
**Reference branch:** `main`  
**Reference commit:** `e7c7fef4e82345af311370426450146a4795523d`  
**Audit working branch:** `quality/cq0-audit-baseline`  
**Review date:** 2026-11-23

## 1. Executive summary

The repository must be understood as an evolutionary product, not just as a set of currently independent plugins. The product began as three plugins, those capabilities were merged into one unified plugin, and that unified product was subsequently developed into a native AniCraft tool. The code-quality audit therefore evaluates both the current architecture and the migration from the three-plugin design to the unified/native product. The current workspace still contains three plugin crates, but their presence in Cargo does not by itself establish that they are still independently shipped or are the intended product architecture.

At the code level, the repository has a sound high-level separation between AniCraft-facing WebAssembly adapters and host-independent Rust domain crates. In particular, Smart Controller's canonical model, input snapshots, graph evaluator, mapping layer, and target composition are separated from host input collection and preview planning.

The most important architecture risks are at the edges of that structure:

1. Smart Controller exposes both a legacy binding model and a newer canonical model from the same crate root, including a wildcard legacy re-export. This makes the public domain surface harder to understand and evolve.
2. The Smart Controller WASM entrypoint currently executes the legacy V1 Slider preview path, while the richer canonical DAG/host-adapter pipeline is exposed through separate public modules and functions. The intended product status of that newer path needs to be explicit.
3. Several high-responsibility core modules are large (evaluator, composition, legacy compatibility, migration). Their internal phases and invariants need clear sub-boundaries before adding more features.
4. The AniCraft SDK is intentionally the only low-level ABI/unsafe boundary, which is a good design, but it should remain small and should not become a home for product-specific policy.

No evidence from this pass justifies merging the Smart Controller and Form Deform domain cores. They represent different concepts and currently have clean, independent dependency roots.

## 2. Product evolution and architectural interpretation

The product history supplied by the project owner is:

```mermaid
flowchart TD
  A["Three original plugins"] --> B["Capabilities consolidated into one plugin"]
  B --> C["Unified product developed into a native AniCraft tool"]
```

This history changes the audit criteria:

- The three plugin crates in the current workspace must be classified as historical source packages, compatibility/development adapters, or actively shipped products based on evidence; do not assume the third category from their directory names.
- Shared functionality should be evaluated across the original three capabilities and the unified/native implementation. Repetition may represent intentional compatibility or migration code, but it must be documented and must not become a second place to add new features.
- The native tool is the target product architecture. Plugin entrypoints and the guest SDK should be assessed as integration/compatibility boundaries in that migration, not automatically treated as the product's permanent top-level architecture.
- Domain cores should remain independent where they represent genuinely distinct domain concepts. Consolidation of the user experience does not require collapsing all domain logic into one crate.
- Persistence, document lifecycle, undo/redo, and native host integration belong to the AniCraft host boundary; the product adapter should not recreate host responsibilities.

The code snapshot alone does not establish whether the original plugin entrypoints remain separately distributed. Until packaging/release evidence is checked, the audit will refer to them as workspace plugin crates rather than claiming they are independently shipped.

## 3. Workspace responsibility map

| Workspace member | Responsibility | Owns | Must not own |
|---|---|---|---|
| `anicraft-plugin-sdk` | Guest-side AniCraft ABI v1 bridge | WASM imports/exports, JSON host requests, command/document helpers, plugin/exporter/ease macros | Controller domain rules, deformation math, application workflows |
| `smart-controller-core` | Host-independent Smart Controller domain | Canonical controller schema, typed inputs/snapshots, graph validation/order, mapping, evaluation, contribution composition, migration, diagnostics | AniCraft requests, WASM imports, UI, document writes, clocks |
| `form-deform-core` | Host-independent deformation calculation | Deterministic absolute pin-position calculation and its finite-value contract | AniCraft queries/commands, persistence, plugin UI |
| `bone-query` plugin | Read-only armature/bone inspection command | Parameter decoding, `bone.info` request, bounded report formatting | Document mutation, controller evaluation |
| `form-deform` plugin | AniCraft adapter for one bone-driven warp-pin operation | Host bone/warp queries, parameter validation, pure-core invocation, optional pin-move command | Reusable deformation mathematics |
| `smart-controller` plugin | AniCraft adapter and preview boundary | Host input acquisition, compatibility decoding, target capability checks, preview plan construction, command entrypoint | Core DAG/mapping/composition algorithms |

### 2.1 Smart Controller core module responsibilities

| Module | Current responsibility | Main collaborators |
|---|---|---|
| `canonical` | Canonical controller, binding, target and mapping configuration types | IDs, endpoint, value, contribution |
| `ids` | Strongly typed controller/input/binding/target identifiers | All canonical/evaluation modules |
| `value` | Typed scalar, angle and vector values | Input, mapping, contribution, composition |
| `input` | Input source/dimension definitions, validity/freshness metadata and immutable snapshots | Evaluator, adapter |
| `graph` | Validated controller DAG and deterministic topological order | Canonical model, evaluator |
| `evaluator` | Input preflight, dependency ordering, fallback handling, controller mapping dispatch, unavailable-output propagation, contribution collection | Graph, input, mapper trait, diagnostics |
| `mapping` | Pure mapping algorithms; canonical mapper plus easing/frame/range helpers | Canonical model, pose, value |
| `pose` | Pose/pose-binding data model and compatibility rules | Canonical mapper |
| `contribution` | Typed contribution and composition policy data | Evaluator, composition |
| `composition` | Deterministic post-pass combining contributions per target | Evaluator, contribution, value |
| `diagnostic_codes` | Stable machine-readable diagnostic codes and diagnostic value types | Evaluator, composition |
| `error` | Core error vocabulary | Core modules |
| `endpoint` | Shared inclusive/exclusive endpoint policy | Model, canonical mapping |
| `schema` | Versioned schema identity/envelope | Migration and serialized formats |
| `migration` | Legacy-format decoding, canonical migration and legacy export | Legacy model, canonical model, schema |
| `legacy` | Original V1 binding/parameter model and preview evaluator | Compatibility wrappers and migration |
| `model` | Original V1 parameters and mode | Legacy preview and validation |
| `preview` | Original V1 Slider/Transform preview result and orchestration | Model, mapping |
| `validation` | Original V1 parameter validation | Plugin entrypoint |
| `lib` | Module declarations and public re-exports | Public crate API |

### 2.2 Smart Controller plugin module responsibilities

| Module | Current responsibility |
|---|---|
| `lib` | ABI manifest and `run` entrypoint; current entrypoint validates V1 `Params`, queries `bone.info` or uses manual input, and logs Slider preview |
| `adapter` | Orchestrates canonical document validation, graph evaluation, composition and host preview session construction |
| `input_provider` | Converts explicit host context and `bone.info` data into typed core input snapshots |
| `compatibility` | JSON decoding and wrappers around core migrations/exports |
| `preview_plan` | Converts composed typed outputs into target-specific, read-only preview items |
| `targets` | Queries geometry information and assesses whether a canonical target is supported/previewable |

## 4. Dependency diagram

The following shows declared Cargo dependencies and the principal runtime call direction. Solid arrows represent crate dependencies; dotted arrows represent internal orchestration/call relationships.

```mermaid
flowchart TD
  SDK["anicraft-plugin-sdk\nABI v1 / JSON host bridge"]
  SC["smart-controller-core\ncanonical model / DAG / mapping / evaluation"]
  FD["form-deform-core\ndeterministic pin math"]
  BQ["bone-query plugin"]
  FDP["form-deform plugin"]
  SCP["smart-controller plugin"]
  HOST["AniCraft host\ncommands / document / undo"]

  BQ --> SDK
  FDP --> SDK
  FDP --> FD
  SCP --> SDK
  SCP --> SC
  BQ -. "bone.info" .-> HOST
  FDP -. "bone.info / warp.info / warp.pin_move" .-> HOST
  SCP -. "bone.info / geometry queries" .-> HOST
  SDK -. "ABI requests" .-> HOST
```

### 4.1 Smart Controller internal call graph

```mermaid
flowchart TD
  Entry["plugin lib::run\nV1 Slider entrypoint"]
  Legacy["model / validation / preview / legacy"]
  Adapter["plugin adapter"]
  Provider["input_provider"]
  CoreGraph["graph"]
  Eval["evaluator"]
  Mapper["mapping::canonical"]
  Compose["composition"]
  Preview["preview_plan"]
  Targets["targets"]
  Compat["compatibility"]
  Canonical["canonical document / schema / migration"]
  SDK["AniCraft SDK"]

  Entry --> SDK
  Entry --> Legacy
  Adapter --> Provider
  Provider --> SDK
  Adapter --> Canonical
  Adapter --> CoreGraph
  CoreGraph --> Eval
  Eval --> Mapper
  Eval --> Compose
  Compose --> Preview
  Preview --> Targets
  Compat --> Canonical
```

**Important distinction:** this is a responsibility/call map, not a claim that the current `run` entrypoint invokes every module. The V1 entrypoint in `plugins/smart-controller/src/lib.rs` currently uses `Params`, `validation::validate`, `preview_slider`, and `bone.info`. The canonical adapter workflow is available separately through public adapter functions.

## 5. Architecture findings

### CQ1-01 — Legacy and canonical models share one crate-level public surface

**Priority:** P1 (public API clarity / future compatibility)  
**Evidence:**
- `crates/smart-controller-core/src/lib.rs:1-27` declares both canonical and legacy modules and uses `pub use legacy::*`.
- `crates/smart-controller-core/src/legacy.rs:1-10` describes the legacy pure model.
- `crates/smart-controller-core/src/canonical.rs:1-10` describes the canonical host-independent configuration model.
- `crates/smart-controller-core/src/migration.rs:1-15` bridges legacy and canonical formats.

**Why it matters:** A wildcard re-export makes the legacy API look like the default crate API and makes it harder for readers to distinguish stable canonical types from compatibility-only types. Given the product's progression from three plugins to a unified plugin and then a native tool, the legacy surface may be a deliberate bridge for existing data or workflows. The issue is not that legacy code exists; it is that its supported lifecycle and intended consumers are not obvious from the public API.

**Recommendation:** Keep both models while migration/compatibility requires them. First document which historical plugin formats and persisted documents the legacy model serves, which code path is authoritative for the native tool, and the deprecation/support policy. Then stop expanding the root wildcard surface where compatibility permits. Prefer explicit namespaces such as `legacy::` and `canonical::`; make any breaking export cleanup a planned API change with call-site inventory and migration tests.

**Acceptance:** New code uses canonical types through explicit canonical APIs; legacy types remain available through an explicitly named compatibility namespace; migration tests continue to pass.

### CQ1-02 — Canonical Smart Controller pipeline is not the current plugin entrypoint path

**Priority:** P2 (feature wiring clarity; promote to P1 if canonical authoring/evaluation is represented as user-available functionality)  
**Evidence:**
- `plugins/smart-controller/src/lib.rs:1-8` describes the entrypoint as a thin V1 preview adapter and declares the newer adapter modules.
- `plugins/smart-controller/src/lib.rs:44-75` implements the current `run` path: deserialize V1 `Params`, validate, query a bone or use manual input, call `preview_slider`, and log a preview.
- `plugins/smart-controller/src/adapter.rs:1-65` exposes a separate canonical document evaluation and composition workflow.
- `plugins/smart-controller/src/input_provider.rs:1-25` describes explicit host input collection.
- `plugins/smart-controller/src/preview_plan.rs:1-25` describes read-only target preview planning.

**Why it matters:** The repository's product direction is now a native AniCraft tool, while the visible WASM plugin entrypoint still runs the V1 preview path. A maintainer cannot infer how the canonical pipeline relates to the native tool, whether the plugin entrypoint is a compatibility path, or which execution path is authoritative. This risks maintaining parallel workflows or mistaking a callable library API for the actual user-facing native workflow.

**Recommendation:** Map the complete native-tool execution path first: native entrypoint, command/event registration, document state, canonical controller loading, input acquisition, evaluation, preview/apply, and persistence. Identify exactly which plugin code is reused, wrapped, or retained only for backward compatibility. Do not assume that wiring the canonical adapter into the old V1 `run` function is the right next step; the native tool may need its own composition root. Keep the legacy entrypoint explicitly classified as compatibility, if that is its actual role.

**Acceptance:** A documented workflow map identifies the actual WASM entrypoint for every user-visible mode; no public API is mistaken for a shipped host workflow.

### CQ1-03 — Evaluator is a high-responsibility module and should be decomposed by phase only when seams are stable

**Priority:** P2 (maintainability)  
**Evidence:**
- `crates/smart-controller-core/src/evaluator.rs` is approximately 1,071 lines.
- Its module documentation and public types describe input preflight, dependency ordering, unavailable-output propagation and contribution collection (`evaluator.rs:1-95`).
- The evaluation body also handles dependency-output fallback, mapper failures, binding validation, diagnostics and unavailable-output propagation (`evaluator.rs:260-380`).

**Why it matters:** This is central domain logic. A future contributor must understand multiple failure policies and execution stages in one file. The current phase-oriented comments are helpful, but the file's breadth increases the cost of reviewing changes and locating invariants.

**Recommendation:** First document the evaluator stages and their invariants. Consider extracting private helpers or private submodules for (a) snapshot/input preflight, (b) dependency resolution and fallback, and (c) output validation/contribution collection. Keep the public `evaluate` API and deterministic behavior unchanged. Do not split merely to reduce line count.

**Acceptance:** Each extracted unit has a single responsibility, focused tests, and no duplicated validation or policy logic.

### CQ1-04 — Legacy compatibility implementation remains a large, distinct domain surface

**Priority:** P2 (maintenance cost)  
**Evidence:**
- `crates/smart-controller-core/src/legacy.rs` is approximately 701 lines and contains the V1 model, validation, interpolation and tests.
- `crates/smart-controller-core/src/migration.rs` is approximately 478 lines and contains multiple migration and export paths.
- `plugins/smart-controller/src/compatibility.rs:1-25` delegates format compatibility work into core migrations.

**Why it matters:** Compatibility is necessary during the transition from the three original plugins through the unified plugin to the native tool. It can nevertheless become a permanent parallel implementation if new features continue to be added to legacy models instead of the canonical/native path.

**Recommendation:** Treat legacy modules as compatibility/migration code unless a specific supported workflow proves otherwise. New behavior should target the canonical/native path. Keep legacy changes limited to required bug fixes, import/export, and compatibility. Before declaring a module maintenance-only, verify the native tool no longer relies on its behavior.

**Acceptance:** Feature PRs identify whether a change belongs to canonical behavior or compatibility support; no new product capability is implemented only in legacy code without an explicit reason.

### CQ1-05 — AniCraft SDK is the correct low-level boundary; keep product policy out of it

**Priority:** P3 (guardrail / positive architecture observation)  
**Evidence:**
- `crates/anicraft-plugin-sdk/src/lib.rs:1-12` documents the ABI and sandbox contract.
- `crates/anicraft-plugin-sdk/src/lib.rs:14-55` contains the host imports and generic JSON request/command/document helpers.
- `crates/anicraft-plugin-sdk/src/lib.rs:90+` contains ABI entrypoint macros and allocation/dispatch helpers.

**Why it matters:** Centralizing unsafe ABI interactions is a strong safety and readability choice. The SDK should remain a generic transport/ABI layer; target-specific policies and Forma Deform behavior belong in plugin adapters or domain crates.

**Recommendation:** Preserve this boundary. Consider splitting the SDK internally only if the public file becomes difficult to navigate; do not create separate crates or abstractions without a demonstrated need. Add focused tests for malformed/empty host responses and ABI boundary assumptions where they can be tested safely.

**Acceptance:** Product-specific host command policy remains outside the SDK; unsafe operations remain localized and documented.

### CQ1-06 — Form Deform and Smart Controller cores should remain independent

**Priority:** P3 (positive architecture observation)  
**Evidence:**
- `crates/form-deform-core/src/lib.rs:1-45` contains a small pure deformation model and deterministic absolute-position function.
- `crates/smart-controller-core/src/lib.rs:1-27` defines a separate host-independent controller/evaluation domain.
- `plugins/form-deform/Cargo.toml` depends on the SDK and Form Deform core.
- `plugins/smart-controller/Cargo.toml` depends on the SDK and Smart Controller core.

**Why it matters:** The domains have different responsibilities: Smart Controller evaluates typed inputs into target contributions; Form Deform computes a deformation pin position. Combining them would create unnecessary conceptual coupling.

**Recommendation:** Keep the cores separate. If a future workflow needs both, coordinate them at a higher-level orchestration boundary, not by making one core depend on the other.

**Acceptance:** No cross-core dependency is introduced without a concrete shared domain concept and explicit design review.

### CQ1-07 — Plugin entrypoint files are suitable as composition roots, but should remain thin

**Priority:** P2 (maintainability guardrail)  
**Evidence:**
- `plugins/form-deform/src/lib.rs:1-120` currently performs parameter decoding, host queries, validation, core calculation, reporting and optional `warp.pin_move`.
- `plugins/bone-query/src/lib.rs:1-100` starts with manifest/entrypoint, host query, and report formatting in one file.
- `plugins/smart-controller/src/lib.rs:1-75` contains the current V1 manifest and run orchestration.

**Why it matters:** The old plugin entrypoints may now be migration shells or compatibility adapters rather than the main product composition roots. The native tool should have a clear composition root of its own. Treating the old plugin `lib.rs` files as the sole application architecture would obscure the actual product execution path.

**Recommendation:** Keep entrypoints as composition roots. Extract a module only when a concern grows independently (e.g. host response decoding, parameter parsing, reporting, or apply workflow). Avoid speculative one-function modules.

**Acceptance:** Each entrypoint reads like a short workflow and delegates substantive policy/algorithm logic to named modules.

## 6. Architecture strengths to preserve

- Domain cores have no direct AniCraft SDK dependency.
- Form Deform core is deterministic and non-accumulating.
- Smart Controller evaluator injects a `ControllerMapper` trait, keeping orchestration independent from mapping algorithms.
- Evaluation and target composition are separate stages.
- Host input collection requires explicit identity context instead of guessing layer/bone identities.
- Target preview planning is documented as read-only and separate from mutating host commands.
- AniCraft host operations are isolated behind the guest SDK.

## 7. Recommended dependency direction

Maintain this direction:

1. Domain models and algorithms depend only on domain types and serialization primitives.
2. Plugin adapters depend on the SDK and relevant domain core.
3. The SDK knows the ABI and generic host request envelope, not Forma Deform product policy.
4. The AniCraft host remains authoritative for document mutation, persistence, undo/redo, and evaluation lifecycle.
5. Cross-domain workflows coordinate at a plugin/application orchestration layer rather than coupling the two core crates.

## 8. CQ1 review decisions needed

Before refactoring, resolve these architecture questions through repository and host integration evidence:

- Which of the three original plugin crates remain independently distributed, if any? Which are retained only as source/history, compatibility adapters, or test fixtures?
- Where is the native tool's actual composition root and how does it call into these workspace crates?
- Is the current Smart Controller V1 WASM entrypoint a supported compatibility path, or is it expected to be replaced by the native tool path?
- Which historical document/data formats must remain readable and writable, and for how long?
- Should the legacy API remain publicly re-exported at crate root for downstream compatibility, or can future major versions move it behind `legacy::`?
- Which evaluator phases are stable enough to extract without obscuring the overall algorithm?
- Is any shared plugin report formatting still used by the native tool, or is it purely historical?

## 9. CQ1 exit criteria

- [x] Workspace responsibility map created.
- [x] Cargo dependency direction recorded.
- [x] Smart Controller module responsibility map created.
- [x] Main architecture findings tied to source references.
- [x] Product evolution recorded: three plugins → unified plugin → native AniCraft tool.
- [ ] Native tool composition root and actual execution path mapped to source files.
- [ ] Original plugin crates classified as shipped, compatibility, migration, or historical.
- [ ] Persisted data and legacy format compatibility policy documented.
- [ ] Public API compatibility policy for legacy exports is decided.
- [ ] Findings are reviewed against the native-tool architecture before refactoring.

## 10. Next step

Before CQ2, complete a targeted native-tool integration trace across the AniCraft host repository and this repository. CQ2 should then assess readability in the context of the actual native-tool path, while treating historical plugin code as compatibility/migration code where confirmed. CQ2 should focus on naming, function size, nesting, documentation quality, and consistency; it should not implement architecture changes from this report.
