# Forma Deform — Development Guide

**Document status:** Canonical and authoritative  
**Project:** Forma Deform / AniCraft Smart Controller  
**Host application:** [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft)

This is the repository's **only development plan and technical source of truth**. It owns the current project definition, architecture, implementation stages, decisions, acceptance criteria, and verification process. The root README is only an overview. Do not create competing development plans, Smart Controller plans, or component README files.

---

## 1. Product definition and scope

Forma Deform is a set of reusable, host-independent controller/deformation libraries and thin AniCraft integrations. The immediate product objective is to deliver a dependable Smart Controller workflow in AniCraft.

The intended progression is:

1. Prove the controller mathematics and data model in a small independent Rust library.
2. Validate the host boundary and real command behavior through a thin AniCraft command plug-in.
3. Move the mature authoring experience into a first-party native AniCraft tool.
4. Expose the same registered command contracts to MCP through AniCraft's existing MCP server.
5. Add continuous playback evaluation only after AniCraft provides an explicit native frame-evaluation integration point.

The repository already contains a Smart Controller core and a WASM command-plug-in adapter. Their presence is not proof that every capability is complete or verified. The native tool, durable document-owned controller definitions, and playback integration are future host-side work.

### 1.1 Current components

| Component | Responsibility | Ownership |
|---|---|---|
| `crates/smart-controller-core` | Controller model, validation, input mapping, dependency evaluation, composition and typed preview results | Forma Deform |
| `plugins/smart-controller` | AniCraft command-plug-in adapter; on-demand host queries and preview | Forma Deform |
| `crates/form-deform-core` | Independent deterministic Form Deform calculation | Forma Deform |
| `plugins/form-deform` | Form Deform host command adapter | Forma Deform |
| `plugins/bone-query` | Read-only AniCraft armature/bone diagnostics | Forma Deform |
| `crates/anicraft-plugin-sdk` | Pinned host communication boundary used by the adapters | Integration dependency |
| TypeScript rectangle/timeline demo | Earlier geometry experiment | Separate experiment; not the Smart Controller UI |

### 1.2 What is not in scope yet

- A standalone animation editor or a parallel UI framework.
- A separate Smart Controller MCP server or protocol.
- Durable controller-definition persistence implemented inside the WASM plug-in sandbox.
- Continuous playback/scrubbing evaluation through a plug-in callback that has not been verified in AniCraft.
- Automatic composition of Smart Controller outputs with Form Deform pin-deformation outputs.
- Claims of host integration, test success, or runtime behavior without corresponding evidence.

---

## 2. Comparable tools in other software

These are useful product and interaction references. They are not dependencies and should not be copied wholesale.

### 2.1 Blender — Drivers

[Blender Drivers documentation](https://docs.blender.org/manual/en/4.4/animation/drivers/introduction.html) describes a driver as a configuration that reads one or more input properties or transform channels, applies a built-in function or expression, and uses a driver F-curve to produce the driven property value. Blender also exposes driver variables, a Drivers Editor, and dependency-aware input references.

**Relevant lessons**
- Separate input variables, mapping/evaluation, and the driven target.
- Make the evaluated value inspectable.
- Track dependencies explicitly; do not hide input references inside opaque expressions.
- Keep the evaluator deterministic and efficient.

**Difference:** Blender Drivers are integrated into Blender's property/animation system. Smart Controller should use AniCraft's own document, command, and evaluation architecture rather than recreate Blender's driver system.

### 2.2 Adobe After Effects — Expression Controls

[Adobe Expression Controls](https://helpx.adobe.com/after-effects/desktop/work-with-expressions/expression-controls/expression-controls.html) provide typed controls such as Slider, Angle, Checkbox, Color, Point, Layer, and Dropdown. Expressions on other properties can reference these controls; one control can drive several properties. Values can also be keyframed.

**Relevant lessons**
- Give artists direct, typed controls rather than requiring them to edit code.
- Allow one named controller to affect multiple targets.
- Keep the controller's identity and purpose visible in the UI.
- Make the control reusable and easy to tune.

**Difference:** After Effects uses expressions attached to properties. Smart Controller's core should remain a typed, host-independent model and should not require arbitrary user-authored script execution.

### 2.3 Toon Boom Harmony — Master Controllers

[Toon Boom Master Controllers](https://learn.toonboom.com/modules/master-controllers) are custom on-screen widgets that can control many scene elements. Harmony supports widgets such as sliders and interpolation grids, with controller behavior connected to nodes and deformers through its scripting interface. Its rigging workflow includes a dedicated Master Controller view.

**Relevant lessons**
- The controller is an artist-facing manipulator, not merely a command dialog.
- A single control can coordinate several rig parameters.
- Slider and 2D interpolation-grid interactions are useful references for scalar and multi-dimensional controllers.
- Technical rigging configuration and day-to-day animator interaction should be coherent.

**Difference:** Harmony's scripting and node model are host-specific. AniCraft's implementation must use its own native tools, Properties, document model, commands, and undo conventions.

### 2.4 Design conclusion

The common pattern is:

**Inputs → mapping/interpolation → one or more typed outputs → inspectable result → host-managed application.**

Smart Controller should adopt that conceptual separation while staying native to AniCraft. The core computes; AniCraft owns document state and user interaction.

---

## 3. Target architecture and ownership

### 3.1 Component boundaries

| Layer | Owns | Must not own |
|---|---|---|
| Smart Controller core | Canonical controller schema, validation, mapping, dependency graph, deterministic evaluation, typed outputs and diagnostics | AniCraft UI, host commands, MCP transport, document persistence, WASM imports |
| AniCraft plug-in adapter (prototype) | Parameter parsing, verified host command calls, conversion to canonical snapshots, preview result reporting | Canonical controller semantics, hidden persistence, continuous playback |
| AniCraft native tool | Tool activation, selection, authoring workflow, contextual UI and preview presentation | A second evaluator or parallel document model |
| AniCraft document/engine | Durable versioned definitions, stable host identities, undoable mutations, evaluation lifecycle | Duplicated controller mathematics |
| AniCraft command registry | Validated read/write operations shared by native UI, scripts and MCP | A second Smart Controller-only command protocol |
| AniCraft MCP | External discovery and invocation of registered commands | Evaluator internals, direct document mutation outside commands, playback subscription |

### 3.2 Data flow

```text
Native AniCraft Tool ─────┐
                         ├── AniCraft Command Registry ─── MCP generic tools
Scripts / CLI ────────────┘              │
                                        ▼
                           AniCraft host integration
                           - document-owned definitions
                           - native/manual/external inputs
                           - immutable evaluation snapshots
                                        │
                                        ▼
                              smart-controller-core
                              - validation
                              - mapping
                              - dependency evaluation
                              - typed outputs/diagnostics
                                        │
                                        ▼
                              AniCraft preview / apply
                              / frame evaluation
```

The native UI and MCP must call the same registered command contracts. Neither path may call the evaluator through a private parallel route.

### 3.3 AniCraft UI alignment

The final Smart Controller is a **first-party native AniCraft tool**, not a WASM plug-in dialog presented as a native tool.

- Place it in AniCraft's existing Tools area.
- Use the existing Properties experience for contextual controller settings and details.
- Reuse AniCraft's selection, command, document, undo, theme, keyboard, accessibility, localization, and status conventions.
- Do not create a separate dock/panel framework or an independent application shell.
- Verify actual host UI components and registration points before choosing exact layouts or APIs.

The current plug-in API is command-oriented and manifest-driven. It is suitable for discrete commands and a proof of host integration, not the final ongoing authoring workflow. A native tool requires first-party host changes in the AniCraft repository.

### 3.4 MCP alignment

Reuse AniCraft's existing generic MCP surface, including command discovery and command execution. Smart Controller operations should become ordinary registered AniCraft commands. MCP clients discover them through AniCraft's command-discovery operation and invoke them through its existing run/script operations.

Do not add:
- a second MCP server;
- Smart Controller-specific transport methods;
- a second command registry;
- direct MCP-to-core calls that bypass host validation and command semantics.

MCP input is an explicit external input/control path. A submitted value must be associated with a configured input identity, validated, attributed to its source, and included in an immutable input snapshot. Request/response command invocation is not a continuous streaming subscription.

### 3.5 State ownership

Keep three state categories separate:

1. **Durable definitions:** controller IDs, input definitions, mapping settings, dependencies and output bindings. These belong to a versioned AniCraft document schema.
2. **Transient input samples:** native, manual, or external/MCP-supplied values with source identity, type, units, frame/session sequence, freshness and validation status.
3. **Evaluation snapshots:** immutable per-invocation host-state and input snapshots. Equal graph/configuration/snapshots must produce equivalent results.

Do not hide durable state in plug-in files or assume the current WASM command plug-in can persist arbitrary document metadata.

### 3.6 Form Deform separation — mandatory

Smart Controller binding evaluation and Form Deform pin-deformation evaluation remain independent.

- Preserve the current Form Deform formula and behavior.
- Preserve Smart Controller V1 as preview-only unless a separately approved change expands it.
- Do not automatically feed Smart Controller outputs into Form Deform.
- Do not combine controller outputs with pin-deformation outputs.
- Any future multi-source composition requires a separate explicit FDM v2 architecture decision defining source identity, precedence, blending, conflict handling, and undo semantics.

---

## 4. Development process: library → plug-in → native tool

This sequence deliberately reduces risk. Each stage proves one boundary before the next stage adds host complexity.

### Stage 0 — Establish the host contract

**Goal:** know exactly what AniCraft supports before designing around it.

Tasks:
- Inspect the actual AniCraft source for tool registration, Properties UI, document schema, command registry, undo transactions, selection/identity, timeline/frame evaluation, and MCP dispatch.
- Verify the current plug-in ABI and every command ID/parameter/return schema used by the adapter.
- Record supported, unsupported, and unverified capabilities separately.
- Identify which changes belong in Forma Deform and which require first-party AniCraft changes.

Exit gate:
- Source-backed integration map with exact file/symbol references.
- No invented host APIs or assumed callbacks.
- Host gaps and implementation ownership explicitly recorded.

### Stage 1 — Small independent proof-of-concept library

**Goal:** prove the mathematical contract without AniCraft.

Start with the smallest useful vertical slice:
- One controller definition with a stable ID.
- One scalar input with declared type and finite range.
- One explicit input snapshot.
- One mapping function (initially linear; add Smoothstep only after the linear case is stable).
- One typed scalar output.
- Validation and structured errors.
- A pure evaluation function with no host, UI, clock, filesystem, network, or global mutable state.

Example conceptual contract:

```rust
fn evaluate(
    controller: &Controller,
    inputs: &InputSnapshot,
) -> Result<EvaluationResult, EvaluationError>;
```

The exact Rust signature may evolve, but the semantic boundary must remain pure and deterministic.

Tests:
- minimum, neutral and maximum input;
- values between anchors;
- reversed or invalid ranges according to the schema's explicit rules;
- NaN/infinity and missing input;
- endpoint policies;
- repeatability for identical snapshots;
- no mutation of the controller or input snapshot.

Only after this vertical slice is reliable should the library grow to multiple inputs, 2D mapping, dependency DAGs, nested controllers, composition, and typed target bindings.

**Existing repository position:** `crates/smart-controller-core` already exists and contains more than a minimal scalar prototype. The audit identified overlapping legacy three-anchor and newer Slider/Transform model directions. Do not rewrite blindly. First characterize existing behavior with tests and fixtures, choose a canonical model, and provide explicit migration/compatibility behavior before removing legacy representations.

Exit gate:
- Stable public core contract.
- Deterministic tests and serialization fixtures.
- No AniCraft dependency in the core.
- Existing V1 behavior characterized before semantic changes.

### Stage 2 — Thin AniCraft command plug-in proof

**Goal:** prove the core can communicate with the real host through the documented plug-in boundary.

Keep the adapter deliberately thin:
1. Read and validate plug-in parameters.
2. Query only verified AniCraft commands (for example, on-demand bone data through the audited `bone.info` command).
3. Convert host data into canonical input snapshots.
4. Call the same library evaluator used by native tests.
5. Return/log a structured preview with input provenance and diagnostics.

Use explicit manual input for deterministic testing. Test host input and manual input separately. Add a write operation only when its host command, target identity, preview/commit behavior, and undo semantics are verified.

The plug-in is not the canonical controller schema, document store, or evaluator. Do not build complex persistent UI in its manifest. Do not claim playback or scrubbing evaluation.

Exit gate:
- Core tests pass.
- Adapter tests cover host response parsing, malformed/missing values, identity mapping and command failures.
- WASM build succeeds.
- In-app smoke test confirms the actual host response and preview behavior.
- Any document mutation is demonstrably undoable.
- No hidden state or implicit default-to-zero behavior.

### Stage 3 — Native tool shell in AniCraft

**Goal:** establish the final user-facing integration without changing controller mathematics.

In the AniCraft repository:
- Register Smart Controller in the existing Tools area.
- Use existing Properties UI patterns for selected-controller settings.
- Reuse the host's selection, focus, command, status and undo conventions.
- Connect UI actions to the same command registry intended for MCP.
- Keep the existing plug-in available during migration until native parity is demonstrated.

The first native milestone may be a shell with controller selection/creation and read-only inspection. Do not implement an invented panel system.

Exit gate:
- Tool appears and behaves like a native AniCraft tool.
- Keyboard/accessibility/focus/theme behavior follows host conventions.
- UI uses registered host commands rather than a private evaluator route.
- No duplicate controller model is introduced in the UI.

### Stage 4 — Durable document model and shared commands

**Goal:** make controller definitions part of the AniCraft document and expose one stable operation surface.

Add a versioned document-owned schema and migration rules. Define command responsibilities for create, update, remove, get, list, validate and preview. Exact command IDs and payloads must follow AniCraft's source conventions.

Mutations must use AniCraft's normal undo/history path. Read-only inspection and preview commands must not mutate document state. External/MCP input submission must have explicit source identity, value type, units, freshness and session lifetime.

Exit gate:
- Save/reopen round trip.
- Schema migration and unsupported-version tests.
- Undo/redo tests for every document mutation.
- Native UI and MCP invoke the same command contracts.
- Invalid or stale input fails clearly or uses a configured fallback with a diagnostic; never silently becomes zero.

### Stage 5 — Unified providers and native preview

**Goal:** resolve all supported input sources into one provider-independent snapshot.

Initial provider types:
- Native AniCraft values.
- Explicit manual/test values.
- External/MCP-supplied values.
- Upstream controller outputs through the validated dependency graph.

Do not infer source precedence or blend multiple sources implicitly. One input has one configured source unless a future schema explicitly defines composition. Every preview should expose source/provenance, resolved value, freshness, mapping result, output and diagnostics.

Exit gate:
- Provider-independent evaluator tests.
- Native/manual/external input parity tests.
- Deterministic snapshots and repeatable previews.
- Missing, malformed, stale and unavailable input behavior is explicit.

### Stage 6 — Target application

**Goal:** apply previewed results to supported AniCraft properties safely.

Begin only with target types that have verified stable identities and host setters. Separate preview from apply. Define type conversion, coordinate space, composition/conflict behavior, validation and undo for each target type. Unsupported targets must fail closed.

Exit gate:
- Explicit target identity and supported type.
- Preview matches committed result.
- One undo transaction restores the prior state.
- No implicit composition with Form Deform.

### Stage 7 — Playback and scrubbing integration

**Goal:** evaluate controllers as part of AniCraft's native frame lifecycle.

This stage requires a first-party AniCraft frame-evaluation hook. Define:
- evaluation order relative to animation, constraints and deformation;
- frame/time input and deterministic scheduling;
- dependency invalidation and cache keys;
- error isolation and stale-input policy;
- lifecycle behavior during scrubbing, playback, pause, document close and reload;
- performance limits and diagnostics.

Do not simulate this with repeated command plug-in invocations or claim MCP provides a continuous playback callback.

Exit gate:
- Host-level integration tests across frame changes and scrubbing.
- Deterministic results for the same document/frame/input snapshot.
- No accumulation or mutation of authored neutral/base values.
- Explicit lifecycle and performance evidence.

---

## 5. Controller model and evaluation rules

### 5.1 One canonical model

Slider, joystick, Smart Bone-style mapping, master/nested controllers and switches should be configurations/capabilities of one controller system—not separate evaluators.

A controller model should eventually describe:
- stable controller identity and schema version;
- typed input definitions and source configuration;
- scalar or 2D mapping configuration;
- dependency references;
- typed output bindings and target identities;
- enable/disable and fallback policy;
- validation and diagnostics.

The core must own mathematical semantics. AniCraft owns persistence and host identity resolution.

### 5.2 Dependency graph

When nested controllers are introduced, represent dependencies as a directed acyclic graph (DAG). Validate self-loops and indirect cycles. Evaluate each node once per invocation in deterministic topological order. Shared subgraphs must reuse the same invocation-local result. Never reuse memoized values across different input/host snapshots.

### 5.3 Mapping

Start with scalar linear mapping and explicit endpoint behavior. Extend only with tested requirements:
- negative/neutral/positive anchors;
- Linear and Smoothstep easing;
- reversed input ranges if explicitly supported;
- explicit clamp/reject boundary policy;
- 2D mapping (for example, a five-pose joystick) with a documented interpolation domain and boundary policy;
- discrete/switch mapping with deterministic selection rules.

Do not add a mapping mode whose semantics, validation, and tests are not specified.

### 5.4 Composition

Composition is a core capability only where the controller model explicitly defines it. Define target base value, contribution identity, ordering, weights/priorities, conflict behavior and unavailable-contribution semantics. Unsupported composition modes must return typed errors rather than silently falling back.

This general controller composition does not authorize cross-engine Smart Controller/Form Deform composition.

### 5.5 Missing, malformed and stale input

- Validate finite numeric values and declared types.
- Preserve source identity and provenance.
- Define freshness explicitly.
- Never silently substitute zero.
- Do not implicitly hold the last value.
- Apply a fallback only when it is explicitly configured and report it in diagnostics.
- Keep unrelated valid outputs available when one independent input fails, where the model permits it.

---

## 6. Repository structure and documentation rules

### 6.1 Expected ownership

```text
/
├── README.md                 # only project overview
├── DEVELOPMENT.md            # only development/architecture source of truth
├── Cargo.toml
├── crates/
│   ├── smart-controller-core/
│   ├── form-deform-core/
│   └── anicraft-plugin-sdk/
├── plugins/
│   ├── smart-controller/
│   ├── form-deform/
│   ├── bone-query/
│   └── INSTALL.md             # operational build/install guide
├── docs/
│   ├── ANICRAFT_SDK_AUDIT.md  # dated evidence snapshot
│   └── SMART_CONTROLLERS_AUDIT.md # dated source audit
└── src/                       # legacy TypeScript experiment
```

The actual tree may contain additional source and test files; the diagram shows documentation and component ownership, not a requirement to rearrange source code.

### 6.2 Single-source rules

- Exactly one root `README.md`.
- Exactly one root `DEVELOPMENT.md`.
- No `README.md` inside plugin/crate directories.
- No `DEVELOPMENT_PLAN.md`, `SMART_CONTROLLER_PLAN.md`, or parallel implementation roadmap.
- The README summarizes and links to DEVELOPMENT.md; it does not repeat phase details.
- Build/install instructions may remain in `plugins/INSTALL.md`; they are operational instructions, not a second roadmap.
- Dated audits may remain as historical evidence. They must not be treated as current status or competing plans. When evidence changes, update this DEVELOPMENT.md and link the audit as supporting context.
- When an architecture decision changes, update this document and the code/tests in the same reviewed change. Do not leave contradictory decisions in older plan files.

---

## 7. Development and verification workflow

Run from the repository root.

### 7.1 Rust formatting and tests

```sh
cargo fmt --all -- --check
cargo test --workspace
```

### 7.2 WASM plug-in builds

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

### 7.3 Host verification

Use [plugins/INSTALL.md](plugins/INSTALL.md) for the current plug-in installation and smoke-test workflow. Host smoke tests must be reported separately from unit tests and WASM builds.

Every change report must distinguish:
- source inspected;
- commands actually executed and their results;
- CI results and commit SHA;
- in-app host tests actually performed;
- unverified assumptions and remaining host dependencies.

A test command appearing in documentation or CI configuration is not evidence that it passed. Do not claim successful verification without a run result.

### 7.4 Pull-request discipline

- Keep changes narrow and reviewable.
- Prefer tests before semantic changes.
- Preserve compatibility with explicit fixtures/migrations.
- Do not merge or push to a protected/default branch without explicit authorization.
- Keep AniCraft host changes separately scoped and reviewed from this repository's core/plug-in changes.
- Report files changed, tests run, tests not run, and known limitations.

---

## 8. Current capability and status baseline

This section is the current status authority. Update it when code or verified host capabilities change.

| Capability | Status |
|---|---|
| Independent Rust Smart Controller core | Present in repository; current model directions require characterization/consolidation |
| Smart Controller command plug-in | Present as an on-demand prototype/compatibility adapter |
| Deterministic preview-only V1 direction | Required behavior to preserve |
| Bone input through `bone.info` | Documented/audited on-demand host path; verify against current AniCraft source and in-app behavior |
| Durable controller definitions in AniCraft documents | Not implemented/established by the current plug-in contract |
| First-party native Smart Controller tool | Proposed; not implemented |
| Shared AniCraft command contracts for native UI and MCP | Target architecture; host implementation not established |
| External/MCP input provider | Proposed; must be validated through the host command path |
| Continuous playback/scrubbing evaluator | Not established; requires first-party AniCraft frame hook |
| Smart Controller/Form Deform output composition | Explicitly excluded pending a separate FDM v2 decision |

The audits in `docs/` are dated evidence snapshots. If they disagree with current AniCraft source or verified behavior, inspect the current source, record the new evidence, and update this table.

---

## 9. Acceptance criteria for the finished product

The native Smart Controller is ready for a production milestone only when:

1. The core has one canonical, versioned model and deterministic tests.
2. The plug-in prototype has verified host-boundary behavior and no hidden persistence assumptions.
3. The native tool follows AniCraft's existing UI conventions and host command architecture.
4. Controller definitions survive document save/reopen and schema migration.
5. Native UI, scripts and MCP use the same registered command contracts.
6. Input providers produce validated, attributed, freshness-aware immutable snapshots.
7. Preview and apply are separate, and supported mutations are undoable.
8. Playback/scrubbing is claimed only after a real host evaluation hook and lifecycle tests exist.
9. Smart Controller remains independent from Form Deform unless a separately approved architecture decision changes that boundary.
10. Verification claims are backed by actual test, CI and in-app evidence.

---

## 10. Reference links

- [Blender Drivers](https://docs.blender.org/manual/en/4.4/animation/drivers/introduction.html)
- [Adobe After Effects Expression Controls](https://helpx.adobe.com/after-effects/desktop/work-with-expressions/expression-controls/expression-controls.html)
- [Toon Boom Harmony Master Controllers](https://learn.toonboom.com/modules/master-controllers)
- [AniCraft host repository](https://github.com/Mujshanan/anicraft)
- [AniCraft SDK capability audit](docs/ANICRAFT_SDK_AUDIT.md)
- [Smart Controllers source audit](docs/SMART_CONTROLLERS_AUDIT.md)
