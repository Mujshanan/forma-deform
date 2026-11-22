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
| Independent Rust Smart Controller core | Canonical DAG/evaluation/composition plus a built-in one-dimensional mapper for range, directed-range, discrete and pose-set configurations; joystick and multi-input mapping remain deferred |
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


---

## 11. AniCraft native-tool feasibility audit

**Audit date:** 2026-11-22  
**Repository reviewed:** [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft)  
**Source snapshot:** [`bcfea816478f2a00ab172cefe67f0bd5696b3886`](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886)  
**Scope:** source inspection of the native editor shell, tool dispatch, Properties UI, engine command/session/history, document schema/evaluation, WASM plug-in host, and MCP/remote command path. This is a source audit, not an in-app runtime test.

### 11.1 Executive finding

**The native-tool direction is feasible, but it requires first-party AniCraft changes.** AniCraft already has the exact host concepts Smart Controller should integrate with: a closed native tool enum, explicit stage-event dispatch, contextual Properties UI, a shared command registry, session-owned document state, undo snapshots, and generic MCP command discovery/execution.

The WASM plug-in API is not a native tool-extension API. It is suitable for the thin command proof described in Stage 2, but it cannot supply the final always-available authoring tool or own durable controller definitions.

No Smart Controller native tool, controller document schema, or general per-frame controller-evaluation hook was found in the inspected source. Treat those as first-party host work, not existing capabilities.

### 11.2 Source-backed integration map

| Concern | Existing AniCraft implementation | Audit result |
|---|---|---|
| Native tool identity and listing | `src/app.rs`: `Tool`, `Tool::ALL`, `label`, `shortcut`, `hint` | Explicit, closed enum/list. No plug-in tool-registration registry was found. A new tool must be added to the native enum and its exhaustive mappings. |
| Tool icon and Tools UI | `src/panels.rs`: `tool_icon`, `tools`, `tool_items` | Native icon mapping and existing Tools-area layout. Iteration over `Tool::ALL` drives the visible tool buttons. |
| Stage interaction | `src/stage.rs`: `show`, drag-start dispatch, click dispatch, drag/update handling | Tool behavior is wired directly into stage pointer-event handling. A native tool needs explicit hit-test, click, drag-start, drag-update, drag-end/cancel and drawing/overlay integration as applicable. |
| Contextual Properties | `src/panels.rs`: `properties`, `bind_props`, `bone_props`, `warp_props` | Existing pattern is contextual: selected bones expose bone settings; Bind mode exposes weight-brush and point controls; Warp mode exposes mesh/pin controls. Smart Controller should follow this pattern rather than create a separate panel framework. |
| Shared command registry | `crates/engine/src/lib.rs`: `CommandSpec`, `commands()`, `find_command()` | Commands are registered by collecting each command module's specs. `find_command` indexes the same registry. Add controller operations as ordinary engine commands. |
| Command execution and atomicity | `crates/engine/src/lib.rs`: `Session::execute`, `execute_nested` | The session executes registered commands, validates through each command implementation, and handles mutation/rollback and history recording. Native UI and external callers should use this path. |
| Gesture undo | `crates/engine/src/lib.rs`: `begin_gesture`, `end_gesture`; `crates/engine/src/history.rs`: `Snapshot`, `History` | Drag-like operations can group repeated mutations into one undo step. Snapshot includes the document and editor context such as scene, frame, layer and selection. |
| Stable host identity | `crates/doc/src/lib.rs`: `ElementId`, `LayerId`, `BoneId`; `crates/engine/src/selection.rs` | Numeric host IDs and session selection already exist. Smart Controller bindings should persist host IDs with explicit type/ownership validation; do not rely on display names. |
| Durable document schema | `crates/doc/src/lib.rs`: `Document`, `DocumentFile`, `Scene`, `Timeline`, `Layer`, `Keyframe` | The document is a typed Serde model with explicit fields and a conversion from `DocumentFile`. The inspected `Document` has no generic custom-metadata extension map. Durable controller definitions therefore require a deliberate first-party schema addition and backward-compatible defaults/migration tests. |
| Timeline evaluation | `crates/doc/src/eval.rs`: `evaluate`, `evaluate_scene`, `evaluate_timeline`, `evaluate_player`; `crates/engine/src/lib.rs`: `Session::set_frame`, `display_list` | Timeline rendering/evaluation is a deterministic document/frame operation. Session frame changes invalidate the editor context; this is not, by itself, a callback for arbitrary plug-in controller evaluation. |
| Existing frame scripts | `crates/doc/src/player.rs`; `crates/engine/src/player.rs` | There is a player/frame-script mechanism, but it is not a general controller callback. The inspected frame-script host exposes player state and queued actions/logging; document/command/elements requests are refused while the movie runs. Frame scripts are associated with frame-entry actions, not a general every-frame controller provider. |
| WASM plug-in commands | `crates/engine/src/cmds_plugin.rs`; `crates/plugins/src/manifest.rs`, `registry.rs`, `runtime.rs`; `docs/plugins.md` | Plug-ins are installed session-side and expose manifest-defined commands/settings. They can query the document/elements and call permitted commands through the host. A run is bounded and treated as one undo step; it is not a persistent native UI extension. |
| MCP discovery and execution | `crates/cli/src/mcp.rs`: `tools()`, `call_tool()`, `Host`; `src/remote.rs`: `Live`, `poll()`; `docs/mcp.md` | MCP exposes generic command discovery/run/script/document/render tools. The live desktop bridge executes through the same `Session::execute` command path and queues edits to avoid merging them with an active user gesture. No separate Smart Controller MCP server is needed. |

### 11.3 Native tool integration requirements

The native UI is explicitly wired rather than dynamically extended by WASM plug-ins. The current `Tool` enum contains 24 built-in tools, and `Tool::ALL` drives the tool list. The stage separately dispatches tool-specific pointer interactions. Consequently, a first-party Smart Controller tool requires coordinated changes in at least:

1. **Tool definition:** add the tool to `src/app.rs` and update its complete tool list, label, shortcut/hint and any tool-state requirements.
2. **Visual identity:** add its icon mapping in `src/panels.rs` and use the existing Tools-area button/accessibility conventions.
3. **Properties experience:** add contextual controls through `panels::properties`, following the existing Bone/Bind/Warp patterns. Avoid a parallel panel/docking system.
4. **Stage behavior:** integrate tool selection, hit-testing, click/drag lifecycle and controller overlay/manipulator drawing in `src/stage.rs`. If the initial native milestone is a non-canvas authoring tool, keep its stage interaction minimal and explicit.
5. **Engine operations:** add controller create/update/delete/query operations to the existing `CommandSpec` registry. Native UI must call these commands rather than mutating document fields directly.
6. **Document persistence:** define a versioned controller-definition representation in AniCraft's document model, with Serde defaults, sanitization/validation, round-trip fixtures and migration behavior. The correct ownership level (document, scene, symbol or another host object) must be decided from the controller use cases before implementation; the current schema does not provide an arbitrary metadata bag.
7. **Undo and identity:** make durable definition changes normal mutating commands so existing snapshots and undo/redo cover them. Persist stable host IDs and reject stale or wrong-kind references.
8. **MCP:** expose no new transport. Register the same commands so `anicraft_commands`, `anicraft_run` and `anicraft_run_script` discover and execute them through the existing host.

The native tool and MCP are two front ends to the same command contracts. The UI may maintain transient interaction state, but the engine/document commands remain the authority for durable definitions and mutations.

### 11.4 What the plug-in proof can and cannot establish

The inspected WASM ABI has a deliberately narrow host boundary. The plug-in host supports requests for `command`, `document`, `elements` and `log`; command requests are routed through the engine's command registry and execute nested within the plug-in run. The plug-in manifest supplies command parameters/settings, and the app can present a generated parameter dialog with preview/OK/Cancel behavior.

This is useful for proving:
- the independent Rust core can be called from the adapter;
- actual AniCraft IDs and command responses can be parsed;
- read-only host snapshots can be converted to explicit controller inputs;
- a preview result can be produced without changing the document;
- any permitted command mutation participates in the plug-in's atomic undo step.

It does **not** prove:
- native tool registration or custom always-visible UI;
- arbitrary custom Properties panels or canvas manipulators;
- durable document-owned controller definitions;
- a persistent plug-in process or state between runs;
- a continuous controller evaluation callback during timeline playback;
- safe Smart Controller → Form Deform composition.

Keep the plug-in as a narrow proof/compatibility adapter while the native implementation is developed. Do not expand its manifest into a substitute authoring application.

### 11.5 MCP contract confirmed

AniCraft's MCP tool list is generic. `anicraft_commands` enumerates the engine's `commands()`; `anicraft_run` and `anicraft_run_script` call the host's `execute` method. The desktop bridge's live host executes `Session::execute` on the open app session. This confirms the intended alignment:

```text
Native Smart Controller UI ─┐
                            ├── AniCraft CommandSpec registry ── Session::execute
MCP generic run/script ─────┘
```

A controller command should have one engine implementation and one parameter/result contract. Do not duplicate its validation or semantics in a second MCP server. Note that `CommandSpec.params` is currently a human-readable parameter description; the command implementation remains responsible for actual validation.

### 11.6 Playback and evaluation finding

AniCraft already evaluates the document at an explicit frame and has a wall-clock/audio-driven editor playback loop. However, the inspected native playback loop advances the session frame; it does not expose a general extension callback for evaluating arbitrary external controller graphs. The movie player has a frame-script facility, but its script host is restricted to player-state reads and queued player actions, and frame scripts are tied to frame-entry behavior.

Therefore:
- Do not implement Smart Controller playback by repeatedly invoking MCP/plug-in commands.
- Do not infer that the frame-script facility is a general controller runtime.
- Before playback integration, define a first-party host evaluation lifecycle with explicit ordering relative to timeline evaluation, stable frame/input snapshots, error reporting, caching/invalidation and deterministic behavior.
- Keep preview evaluation callable independently of playback so the core can be tested without the editor loop.

This audit did not establish that no suitable internal hook could ever be added; it establishes that no public/general hook was identified in the inspected integration path.

### 11.7 Recommended implementation sequence after the audit

1. **Finish the source audit and record host gaps** (this section). No host code changes are included in this Forma Deform audit.
2. **Characterize and consolidate `smart-controller-core`**: tests/fixtures for the existing three-anchor and Slider/Transform directions; select a canonical representation and specify migration before refactoring.
3. **Complete the thin plug-in proof** only for the read-only/manual preview path and verified host queries. Preserve Smart Controller V1 preview-only and Form Deform independence.
4. **Design the AniCraft document schema and command contract** before native UI implementation. Decide controller ownership, IDs, serialization compatibility, command atomicity and target-reference validation.
5. **Implement the first-party native tool in the AniCraft repository**: tool registration, contextual Properties, minimal stage interaction and shared commands. Keep it on a separate reviewable host PR.
6. **Add MCP coverage through the existing generic command surface** and test native/MCP parity against the same command tests.
7. **Design playback evaluation as a separate host milestone** after the native preview and document model are stable.

### 11.8 Verification status and limitations

- Source reviewed at the exact AniCraft commit linked above.
- Repository files inspected: native app/tool/panels/stage, engine registry/session/history, document model/evaluator/player, plug-in host/runtime/manifest/registry, CLI MCP and desktop remote bridge.
- No AniCraft build, test suite, GUI session or live MCP/plug-in smoke test was run as part of this source-only audit.
- This is not a comprehensive security review or a proof that every possible internal extension point is absent.
- Any future implementation must re-check the source commit at the time of coding; the host repository is actively changing.



---

## 12. Smart Controller core characterization and consolidation

**Characterization baseline:** branch `audit/anicraft-native-tool-feasibility`, source commit `ec9c8b4738cb396c78bf53829df142b7be9d63cf`.  
**Scope:** inspect the existing core, migration layer, plug-in adapter and tests before changing controller semantics. The current work fixes two compile-time integration blockers and records the characterization; it does not yet change serialized controller semantics.

### 12.1 Existing model inventory

| Existing type/path | Actual responsibility | Classification |
|---|---|---|
| `legacy::BindingSet`, `legacy::ControllerBinding` | Versioned V1 binding collection; one binding owns a driver ID, three-point input range, easing, target anchors, mode and enabled state. Its evaluator directly calculates symbol-frame or geometry-deform preview values. | Compatibility format and reference behavior; preserve through fixtures and migration/export. |
| `model::Params`, `model::Mode`, `preview::preview_slider`, `preview::preview_transform` | The older plug-in command payload. Slider uses a directed two-endpoint rotation range and frame interval; Transform uses the directed range and a scaled property output. | V1 command/adapter DTO and compatibility preview; not the long-term authored document model. |
| `migration::CanonicalDocument`, `canonical::Controller`, `canonical::ControllerBinding`, `input::InputDefinition` | Versioned document containing shared inputs, controller nodes, dependencies, typed target bindings and explicit composition configuration. | Intended authoritative authored/runtime model. |
| `graph::ControllerGraph`, `evaluator::evaluate` | Validates the controller DAG, resolves immutable input snapshots, checks dependencies and unavailable/stale inputs, then collects typed binding contributions in deterministic topological order. | Canonical orchestration. |
| `composition::compose` | Independent post-pass for Replace, Additive, WeightedBlend and PriorityOverride target contributions. | Canonical composition stage; remains separate from evaluation and from Form Deform. |
| `evaluator::ControllerMapper` | Injected mapping interface. The core evaluator delegates per-controller mapping to this interface. | Test/extension seam, but currently no complete built-in canonical mapper was identified. |

The old V1 formats are not interchangeable with the canonical document. They encode different use cases and identity guarantees. In particular, `Params` has no authored controller/binding IDs; migration assigns deterministic temporary identities and reports when fields are discarded. Do not promote those generated IDs to durable authored identity without an explicit first-save replacement policy.

### 12.2 Duplicated mapping primitives

| Behavior | Existing implementations | Characterization |
|---|---|---|
| Three-anchor normalization | `legacy::InputRange::normalize`; `mapping::range::ThreePointRange::normalize_signed` | The legacy method now delegates to `ThreePointRange`, preserving its V1 error type. Existing `range_mapping_tests.rs` checks numeric parity. |
| Easing | `legacy::EaseCurve`; `mapping::easing::Easing`; `canonical::EasingConfig` | `canonical::EasingConfig` now re-exports the shared `Easing` type; the legacy enum delegates to it while retaining the old serialized names and V1 surface. |
| Directed range normalization | `mapping::normalize_rotation`; `mapping::range::normalize_directed` | The V1 helper is now a thin wrapper over `normalize_directed`, translating `CoreError` to the old string error. Existing tests compare forward and reversed ranges. |
| Frame mapping | `mapping::map_to_frame`; `mapping::frame::map_to_frame` | Existing tests compare endpoint outputs. The newer helper returns `Result` and rejects non-finite values and reversed frame intervals; the V1 helper has a non-fallible signature and a debug-only interval assertion. |
| Three-anchor configuration | `MappingConfig::Range1D`; `MappingConfig::ThreeAnchorRange` | Both carry minimum/neutral/maximum. `ThreeAnchorRange` additionally carries easing and is the form emitted by V1 BindingSet migration. The duplicate needs a version-aware consolidation decision. |
| Two-endpoint configuration | `MappingConfig::DirectedRange1D` | Semantically distinct from a three-anchor range: it maps a directed start/end interval into `[0, 1]`, preserving reversed endpoints. Keep this distinct. |

The existing tests already characterize much of the old/new numeric equivalence. The remaining work is to make the canonical configuration actually select the canonical mapping implementation, rather than maintaining parallel helpers indefinitely.

### 12.3 What is already characterized by tests

The inspected core test suite includes:
- captured V1 BindingSet and Params JSON fixtures, strict deserialization and round trips;
- legacy symbol-frame and geometry-deform endpoint, midpoint, neutral and disabled behavior;
- host-driven and manual Slider preview values;
- reversed rotation ranges and inclusive/exclusive frame endpoints;
- equivalence tests for old/new three-anchor normalization, directed normalization and frame mapping;
- BindingSet and Params migration/export round trips, including explicit loss reporting;
- canonical schema version/unknown-field rejection and controller-output dependency validation;
- deterministic graph ordering, missing dependency and cycle rejection;
- typed contribution composition, ordering, explicit additive bases and invalid composition handling.

These tests are compatibility evidence, not proof that the canonical model has a complete end-to-end mapper. Most evaluator tests inject a closure implementing `ControllerMapper`; that verifies orchestration and composition while leaving the actual canonical mapping algorithm outside the tested production path.

### 12.4 Consolidation target

Adopt these boundaries:

1. **Canonical authored/runtime model:** `CanonicalDocument`, `Controller`, `InputDefinition`, `ControllerBinding`, `MappingConfig`, `TargetMapping` and typed values are the only model used by new core and host features.
2. **Compatibility ingress/egress:** keep `legacy::BindingSet` and `model::Params` only for explicit import, migration, export and the existing V1 command adapter. They must not become a second runtime state model.
3. **One numeric mapping implementation:** make the checked `CoreError` mapping primitives authoritative. Legacy functions become thin compatibility wrappers only where their public signatures must be retained; otherwise deprecate them after all callers move.
4. **One easing type:** use the canonical mapping easing type in the runtime model and map legacy `EaseCurve` values at the compatibility boundary.
5. **One three-anchor variant:** consolidate `Range1D` and `ThreeAnchorRange` into a single canonical three-anchor representation with explicit easing semantics. Retain `DirectedRange1D` as the separate two-endpoint mapping. Because canonical documents are already versioned, define a schema migration/compatibility path before changing serialized variants; do not silently reinterpret schema version 1.
6. **Built-in canonical mapper:** implement mapping behavior in `smart-controller-core`, not AniCraft or the WASM adapter. Keep `ControllerMapper` as a seam for tests/alternate algorithms, and provide a first-party implementation that consumes canonical `MappingConfig`, resolved typed inputs and target bindings. Unsupported mapping kinds must return explicit typed errors/diagnostics rather than fabricate outputs. The first one-dimensional implementation now covers Range1D, ThreeAnchorRange, DirectedRange1D, Discrete and PoseSet1D; JoystickFivePose remains explicitly deferred to a dedicated two-dimensional mapper.
7. **Host adapter stays thin:** it resolves AniCraft identities, gathers attributed/freshness-aware input snapshots, invokes the core, verifies target capabilities and presents preview data. It does not define mapping or composition semantics.

### 12.5 Safe implementation order

1. Preserve and run the existing V1 characterization and migration tests as the behavioral baseline.
2. Add direct canonical mapping tests for every supported mapping kind, including neutral, endpoints, clamping, reversed ranges, easing, typed-value mismatch and numeric overflow.
3. Specify the canonical mapper contract and its behavior for one-input versus multi-input controllers, PoseSet/Joystick/Discrete mappings, and target anchor interpolation. Do not infer multi-input semantics from the current data structures. The initial one-dimensional contract is now recorded in section 12.9; multi-input and JoystickFivePose remain deferred.
4. Decide whether schema version 1 is experimental-only or must remain readable. If it must remain readable, implement an explicit schema migration before removing/renaming `Range1D` or changing its JSON representation.
5. Consolidate the mapping/easing helpers behind the canonical implementation, retaining compatibility wrappers until all legacy callers and fixtures are accounted for.
6. Move V1 DTOs behind a clearly named compatibility module/API and migrate the plug-in adapter to canonical preview orchestration.
7. Only after the core has a tested built-in mapper, stable migration behavior and a clean workspace build should native AniCraft UI work begin.

### 12.6 Form Deform boundary

This consolidation does not change the accepted independence rule: Smart Controller evaluation produces its own typed contributions and target preview results. It does not feed values into Form Deform pin-deformation evaluation, alter the existing pin formula, or introduce multi-source composition. Any such behavior requires a separate FDM v2 decision.

### 12.7 Baseline CI finding and current patch

The source-only audit PR's CI run `44989449828` reached `cargo test --workspace` and failed while compiling `anicraft-smart-controller-plugin`:
- `plugins/smart-controller/src/preview_plan.rs:94`: the geometry response `BTreeMap` lacked explicit key/value types (`E0282`);
- `plugins/smart-controller/src/preview_plan.rs:138`: tests imported `EvaluationResult` through `composition`, but that module did not publicly re-export the evaluator result (`E0603`);
- `composition.rs` also contained unused imports.

The characterization branch fixes these compile-time issues by typing the geometry response cache, publicly re-exporting the shared `EvaluationResult` contract, and moving test-only imports into the test module. The first rerun compiled the workspace and reached tests, then exposed a contradictory plug-in test: one test expected a missing manual sample to make the provider fail immediately, while another expected the provider to return an explicit Missing sample for core fallback handling. The test now asserts the latter contract (missing sample, no fabricated value), consistent with the provider/core responsibility boundary. GitHub Actions CI run #447 on the corrected branch head (`bbf3df5f10af6dd056223612271870262ef7d95a`) passed all steps: Bun install/typecheck/tests/build, `cargo test --workspace`, and all three WASM plug-in release builds. This verifies the current characterization branch at that commit; it does not establish in-app AniCraft behavior or complete canonical mapping semantics.



### 12.8 Consolidation progress

The first behavior-preserving consolidation is now in the core:
- `canonical::EasingConfig` re-exports the shared `mapping::easing::Easing` type; canonical documents retain the same `linear`/`smoothstep` serialized values.
- `legacy::InputRange` delegates validation and normalization to `mapping::range::ThreePointRange`, translating errors back to the V1 `ControllerError` variants.
- The V1 wrapper preserves validation precedence (invalid range before non-finite sample); a regression test pins that ordering.
- `legacy::EaseCurve` delegates to the shared checked easing implementation.
- `mapping::normalize_rotation` delegates to `mapping::range::normalize_directed`, preserving its V1 string-error signature.

Still intentionally not consolidated:
- `mapping::map_to_frame` and `mapping::frame::map_to_frame` have different invalid-input/error contracts. The checked `Result<_, CoreError>` API is the new-code target; retain the V1 wrapper until its callers and invalid-input compatibility are explicitly handled.
- `MappingConfig::Range1D` and `MappingConfig::ThreeAnchorRange` remain duplicate three-anchor representations. Removing one requires an explicit canonical schema compatibility/migration decision.
- A first-party `CanonicalControllerMapper` now implements the supported one-dimensional mapping variants. The evaluator still accepts the `ControllerMapper` abstraction so tests and alternate strategies remain injectable; this does not imply JoystickFivePose, general multi-input mapping, native authoring, persistence, or playback support.
- Pose-set, joystick and discrete mapping semantics remain underspecified by the present pose data model and are not silently implemented as scalar ranges.

No target output, controller schema, preview behavior, input fallback policy, or Form Deform behavior was intentionally changed by these refactors. CI must verify the resulting branch before this consolidation increment is considered complete.


### 12.9 First built-in canonical mapper (FD-M1)

**Implementation:** `crates/smart-controller-core/src/mapping/canonical.rs`  
**Public entry point:** `mapping::CanonicalControllerMapper`  
**Runtime pose lookup:** `mapping::PoseLibrary`  
**Tests:** `crates/smart-controller-core/tests/canonical_mapper_tests.rs`

The evaluator retains its `ControllerMapper` injection boundary. New code can now use the first-party `CanonicalControllerMapper` instead of supplying a closure or a custom implementation. The mapper consumes one declared, already-resolved input and emits uncomposed typed values keyed by the controller's canonical BindingIds.

#### Supported mapping semantics

| Mapping | Driver domain | Output behavior |
|---|---|---|
| `Range1D` | Explicit minimum / neutral / maximum | Clamped signed normalization to [-1, 1], linear interpolation between negative-neutral-positive target anchors |
| `ThreeAnchorRange` | Explicit minimum / neutral / maximum | Same signed normalization with symmetric Linear or Smoothstep easing on either side of neutral |
| `DirectedRange1D` | Directed start / end; reversed endpoints are valid | Normalizes to [0, 1]; anchor-based targets use the corresponding signed value, while FrameRange uses the unit value |
| `Discrete` | Scalar driver clamped to [-1, 1] | Requires exactly one output binding; selects the nearest equally spaced authored value; exact midpoint ties choose the higher index; preserves the selected TypedValue |
| `PoseSet1D` | Scalar driver clamped to [-1, 1] | Resolves ordered pose IDs from PoseLibrary and interpolates adjacent poses evenly across the domain |

Range inputs may be Scalar or Angle. Angle driver values and Angle property outputs are interpreted in radians and preserve the input's wrapped/unwrapped semantics. Discrete and PoseSet1D currently require Scalar input.

#### Target behavior

- SymbolFrame interpolates the negative, neutral and positive frame anchors and rounds to the nearest integer frame.
- FrameRange maps the normalized unit value through the configured inclusive/exclusive endpoint policy; the default remains Exclusive.
- GeometryDeform interpolates each vector component independently. RelativeDelta emits the difference from the neutral anchor so the evaluator's explicit additive base restores the authored absolute target value.
- Property maps the signed normalized value by the authored scale. Scalar inputs produce Scalar outputs; Angle inputs produce Angle outputs.
- PoseSet1D emits a complete pose. Every registered pose must have a compatible binding/value schema, and the resulting binding-ID set must exactly match the controller's declared bindings.

#### Explicit limits

- One controller currently declares exactly one driver input. Multi-input controllers are rejected as a mapping-domain failure rather than selecting an arbitrary input.
- PoseLibrary is runtime-only. It is not serialized into CanonicalDocument schema v1 and does not claim durable host persistence.
- JoystickFivePose returns an explicit mapping-domain failure from this one-dimensional mapper. It is not approximated as a scalar range; a dedicated two-dimensional mapping implementation remains a later milestone.
- Mapping returns uncomposed contributions only. It does not apply target state, mutate AniCraft documents, or feed Form Deform pin evaluation.
- Unsupported or invalid mapping behavior is reported through the evaluator's existing `SC_MAPPING_DOMAIN` diagnostic and marks the controller's outputs unavailable.

The new integration tests exercise range endpoints and clamping, symmetric easing, reversed directed ranges, frame endpoint policy, discrete typed outputs, pose-set interpolation by binding identity, relative geometry deltas, angle semantics, missing pose references, exact pose binding sets, unsupported joystick behavior and ambiguous multi-input rejection. This implementation is a core mapping milestone, not evidence of native AniCraft UI, durable pose storage, continuous playback evaluation or Smart Controller/Form Deform composition.
