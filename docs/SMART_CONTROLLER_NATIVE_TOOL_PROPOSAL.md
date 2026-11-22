# Smart Controller — Native AniCraft Tool and MCP Integration Proposal

**Status:** Proposed target architecture; not implemented  
**Project:** `Mujshanan/forma-deform`  
**Host:** [`Mujshanan/anicraft`](https://github.com/Mujshanan/anicraft) (fork of `storytold/anicraft`)  
**Supersedes:** The assumption that the command plug-in dialog is the final Smart Controller UI  
**Scope:** Product architecture, host integration requirements, MCP alignment, and implementation sequence

## 1. Decision

Smart Controller should become a **native AniCraft editor tool**, not remain a command plug-in as its final user experience.

The existing WASM command plug-in is a useful prototype and compatibility path. It demonstrates the host-independent evaluator and on-demand preview, but its manifest-generated parameter dialog is not the right interaction model for an ongoing controller-authoring workflow.

The native tool must follow AniCraft's established editor conventions:

- Tools live in the left-side Tools area.
- Tool-specific settings and contextual information belong in AniCraft's existing Properties experience, following the host's selection and focus conventions.
- Frequently used actions remain reachable through AniCraft's command palette (Ctrl+K), menus, and registered shortcuts where appropriate.
- Use AniCraft's theme tokens, form rows, sections, icons, accessibility naming, localization, and undo/status feedback. Do not introduce a visually independent UI system.
- The tool must use the host's existing document, selection, timeline, command, and evaluation models instead of recreating them.

This proposal does not assume that AniCraft currently exposes a third-party native-tool extension API. The implementation is a **first-party host integration** in `Mujshanan/anicraft`, coordinated with this repository. It must not be faked by presenting a plug-in command dialog as a native tool.

## 2. Why a native tool

A command plug-in is invoked, runs, and finishes. AniCraft's command plug-in ABI creates a fresh WASM instance per run and exposes manifest parameters through a command dialog. It is appropriate for discrete operations and previews.

Smart Controller is an authoring and inspection workflow with persistent identity and repeated interaction. Users need to select a controller, inspect its inputs and bindings, edit mappings, see current values, and preview results while working in the editor. A one-shot command dialog is a poor fit for that lifecycle.

Therefore:

- **Native tool** owns the human interaction and authoring workflow.
- **Smart Controller core** owns controller semantics and deterministic evaluation.
- **AniCraft engine/document model** owns durable controller definitions, undoable edits, host state, and integration with the editor lifecycle.
- **AniCraft command registry** exposes explicit operations to the UI, CLI, scripts, and MCP.
- **AniCraft MCP** exposes those same registered operations to external agents through its existing generic tools.

## 3. User experience aligned with AniCraft

### 3.1 Tool placement

Add Smart Controller as a first-party tool in AniCraft's existing left-side Tools area. It should use the standard tool button size, active-tool accent, tooltip, accessibility name, keyboard focus, and tool-selection behavior.

Do not add a separate Smart Controller window as the primary interface. Do not create a second dock/panel framework. Reuse the host's existing Properties panel for contextual settings and controller details.

### 3.2 Interaction model

The proposed workflow is:

1. Activate Smart Controller from the Tools area.
2. Select an existing controller in the controller list/contextual selector, or create one using the host's standard add action.
3. Inspect and edit its input source, mapping, dependencies, and output bindings in the Properties experience.
4. Select a source or target through AniCraft's existing selection/property mechanisms wherever possible.
5. Preview the evaluation at the current frame.
6. Commit configuration changes through normal AniCraft document commands and undo history.

The exact arrangement of controller list, tool options, and Properties sections must be decided after inspecting the host's current tool and Properties implementations. This proposal intentionally does not prescribe a new panel API or a mock-up that conflicts with AniCraft's actual layout.

### 3.3 Controller authoring sections

The contextual Properties experience should use native collapsible sections and standard form controls:

| Section | Responsibility |
|---|---|
| Controller | Name, stable identity, enabled state, controller type |
| Inputs | Input definitions, source selection, value type, units, fallback and freshness policy |
| Mapping | Scalar/2D mapping, ranges, endpoint behavior, easing and boundary policy |
| Dependencies | Nested controller references and dependency validation |
| Outputs | Typed bindings and target selection |
| Preview | Current input snapshot, normalized values, evaluated outputs, diagnostics and provenance |

Only controls supported by the host UI should be included in the first implementation. Avoid inventing custom widgets before verifying existing host components.

## 4. Shared architecture

The native UI and MCP path must use the same controller model, command contracts, input snapshot semantics, and evaluator.

```text
Human
  │
  ▼
AniCraft Native Smart Controller Tool
  │
  ├── Native tool state / Properties UI
  │
  ▼
AniCraft command registry ◄──────── AniCraft MCP client
  │                                      │
  │                                      └── anicraft_commands
  │                                          anicraft_run
  │                                          anicraft_run_script
  ▼
Smart Controller host integration
  │
  ├── Native input provider
  ├── Explicit external/MCP-supplied input values
  ├── Document-owned controller definitions
  └── Immutable input + host-state snapshots
  │
  ▼
smart-controller-core
  │
  ├── validation
  ├── dependency DAG evaluation
  ├── mapping and composition
  └── typed outputs + diagnostics
  │
  ▼
AniCraft preview / document evaluation / undoable commands
```

The core remains host-independent. It must not depend on AniCraft UI, MCP transport, WASM imports, or document internals.

The host adapter is responsible for resolving host identities and converting AniCraft data into canonical snapshots. The UI and MCP do not call evaluator internals directly.

## 5. MCP alignment with AniCraft

### 5.1 Reuse the existing MCP server

Do not create a Smart Controller MCP server, a second transport, a plugin-specific MCP protocol, or parallel AI-only tool names.

AniCraft already exposes the generic MCP tools:

- `anicraft_commands` — discover registered command IDs, labels, menus, shortcuts, and parameter schemas.
- `anicraft_run` — execute one registered command.
- `anicraft_run_script` — execute an ordered sequence of commands.
- `anicraft_document` — inspect document context.
- `anicraft_render` — inspect the rendered frame.

Smart Controller operations should be registered in AniCraft's existing command registry. They then become discoverable and callable through the existing MCP tools, without adding Smart Controller-specific MCP protocol methods.

### 5.2 Shared command surface

The following are **proposed command responsibilities**, not existing command IDs. Final IDs, parameter schemas, return types, permission rules, and undo behavior must be designed against AniCraft's command conventions in the host repository.

| Operation | Purpose | Expected behavior |
|---|---|---|
| Create controller | Add a controller definition | Document mutation; undoable |
| Update controller | Change controller configuration | Document mutation; undoable |
| Remove controller | Remove a definition and resolve references safely | Document mutation; undoable |
| Get controller | Inspect a definition and its validation state | Read-only |
| List controllers | Discover document-owned controllers | Read-only |
| Set external input | Supply an explicitly configured external input value | Session/input-state operation; persistence policy must be explicit |
| Get input snapshot | Inspect resolved values and provenance | Read-only |
| Preview controller | Evaluate against an explicit frame/snapshot | Preview-only by default |
| Apply output | Commit an explicitly approved supported output | Mutating and undoable; not part of the initial preview-only milestone |

The MCP client should discover these commands using `anicraft_commands` and invoke them using `anicraft_run` or `anicraft_run_script`. The same command contracts must be used by native UI actions.

### 5.3 MCP is an input source, not an evaluator

An external agent may calculate or obtain a value and submit it as an explicitly configured input. AniCraft validates and associates that value with a canonical InputId. The normal Smart Controller evaluator consumes the resulting snapshot.

MCP must not:

- Bypass input validation or controller graph validation.
- Invoke evaluator internals through a separate code path.
- Directly mutate controller outputs or Form Deform state.
- Be required for native controllers to work.
- Imply continuous streaming or playback callbacks that AniCraft does not provide.

The initial external-input path is request/response through AniCraft's existing MCP command execution. A streaming subscription is a separate future host capability, not an assumption of MCP itself.

## 6. State ownership and lifecycle

Separate three kinds of state.

### 6.1 Durable controller definitions

Controller identity, input definitions, mapping configuration, dependency references, and output bindings are document-owned data. They must be serialized with the AniCraft document using an explicit schema/version and migrated safely.

The current command plug-in sandbox does not provide a general arbitrary document-metadata API. This is a host-engine/document-model responsibility, not something to work around with hidden files or plug-in-local storage.

### 6.2 Transient input samples

Native and MCP-supplied live values are evaluation inputs, not automatically durable controller configuration. They belong to an explicit session/input-provider lifecycle.

Each sample should carry:

- Canonical InputId and declared value type.
- Source/provider identity and provenance.
- Value and units.
- Host frame or session sequence where relevant.
- Freshness/age metadata and validation status.

The host must define when samples expire, whether they can be cached, and what happens when a provider disconnects. Do not silently hold the last value or substitute zero.

### 6.3 Evaluation snapshots

Every preview/evaluation consumes an immutable snapshot. The same controller graph, input snapshot, host-state snapshot, and evaluation configuration must produce deterministic results.

A preview should report the input source, resolved value, freshness, mapping result, target assessment, and any diagnostic. A missing required input must fail or produce an explicitly configured fallback with a diagnostic.

## 7. Native and MCP input providers

Use one provider-independent input contract.

Initial provider categories:

1. **Native AniCraft provider:** Read host values through verified engine APIs, such as evaluated bone properties at an explicit frame.
2. **External/MCP-supplied provider:** Accept values submitted through registered AniCraft commands, with configured source identity and validation.
3. **Manual provider:** Explicit user-supplied values for testing and deterministic previews.
4. **Controller-output provider:** Resolve upstream controller outputs through the existing dependency DAG.

No implicit source blending or precedence. One input has one configured source unless the model explicitly defines a multi-source composition rule. Manual override must be explicit and visible.

Native providers and external/MCP providers must produce the same canonical snapshot types. The evaluator must not know which transport delivered a sample.

## 8. Evaluation and mutation boundaries

### Initial native-tool milestone

- Author and inspect controller definitions.
- Resolve native/manual inputs for an explicit frame.
- Accept explicit MCP-supplied values through the same input contract.
- Produce deterministic preview output and diagnostics.
- Do not modify authored keyframes, bones, geometry pins, or Form Deform state as a side effect of preview.

### Later host-evaluation milestone

Continuous playback and scrubbing require a first-party AniCraft evaluation hook with defined scheduling, caching, invalidation, frame lifecycle, and deterministic behavior. Do not simulate a continuous evaluator by repeatedly launching a command plug-in or by asking an MCP client to poll.

Applying controller output to a target must be a separately defined host operation with explicit target identity, supported property types, conflict/composition rules, and undo semantics.

## 9. Form Deform boundary

Smart Controller and Form Deform remain independent systems.

- Smart Controller owns controller definitions, input resolution, mapping, dependency evaluation, and controller outputs.
- Form Deform owns its pin-deformation model, formula, and evaluation.
- Neither engine imports or calls the other's evaluator.
- No Smart Controller output is automatically fed into Form Deform.
- No Form Deform pin output is automatically composed with Smart Controller output.
- Any multi-source composition between them requires a separate, explicit architecture decision and compatibility tests.

## 10. Implementation sequence

### SC-NATIVE-0 — Host feasibility and integration map

**Documentation/research only.**

- Inspect AniCraft's actual tool registry, tool state, tool options, Properties panel, document model, command registry, undo transaction, frame evaluation, and MCP command dispatch.
- Identify the smallest first-party host integration points and exact source files.
- Verify how native tools access selection, timeline frame, document state, and property controls.
- Verify how registered commands are exposed through `anicraft_commands` and `anicraft_run`.
- Produce a host integration contract; do not guess module paths or introduce a generic extension framework.

**Exit:** Source-backed integration map and explicit list of required AniCraft changes.

### SC-NATIVE-1 — Native tool shell

- Add Smart Controller to the native Tools registry.
- Use existing tool activation, focus, shortcut, icon, theme, and accessibility patterns.
- Connect contextual configuration to existing Properties UI.
- Keep all controller logic in `smart-controller-core`.
- No persistence or live playback claim until the next host contracts exist.

**Exit:** Tool activates and renders in the editor; no duplicate UI framework; UI tests and accessibility checks pass.

### SC-NATIVE-2 — Document model and command contracts

- Define a versioned, document-owned controller collection.
- Add typed, undoable commands for create/update/remove and read-only commands for get/list.
- Validate IDs, graph references, mappings, bindings, and migrations.
- Ensure native UI and MCP use exactly the same command schemas.
- Preserve document compatibility and reject unsupported schema versions safely.

**Exit:** Save/reopen round trips, migration tests, command tests, undo/redo tests, and MCP discovery/invocation tests pass.

### SC-NATIVE-3 — Unified input providers and preview

- Define provider-independent input samples and immutable snapshots.
- Implement native, manual, and external/MCP-supplied providers.
- Add provenance, freshness, missing-input, timeout, and error behavior.
- Connect the native tool preview to the same evaluator and command path used by MCP.
- Keep preview read-only.

**Exit:** Identical snapshots produce deterministic outputs regardless of provider; missing/invalid/stale inputs are explicit.

### SC-NATIVE-4 — Host target integration

- Add only verified target adapters and supported properties.
- Define preview versus apply behavior.
- Ensure apply is one explicit undoable operation.
- Add conflict handling and deterministic target ordering.
- Do not introduce Smart Controller/Form Deform composition.

**Exit:** In-app target preview/apply, undo/redo, target identity, and regression tests pass.

### SC-NATIVE-5 — Playback and scrubbing

- Add a first-party frame-evaluation hook only after its host lifecycle contract is approved.
- Define scheduling, cache invalidation, dependency evaluation, input refresh, and error reporting.
- Avoid MCP polling as a playback mechanism.
- Test deterministic rendering across playback, scrubbing, save/reopen, and headless rendering.

**Exit:** Playback/scrubbing behavior is deterministic and does not rewrite authored keyframes.

## 11. Migration from the current command plug-in

The current `plugins/smart-controller` remains useful during the transition:

- Preserve the host-independent core and its tests.
- Preserve current V1 slider preview semantics and endpoint behavior.
- Keep the command plug-in as a temporary compatibility/demo route until the native tool reaches feature parity.
- Do not let the plug-in manifest become the canonical controller schema.
- Do not add durable state to the plug-in instance; AniCraft creates a fresh instance per run.
- Remove or deprecate the command plug-in only after the native tool and MCP command path are verified and the compatibility decision is documented.

## 12. Security and reliability

- External inputs are opt-in and bound to configured InputIds.
- Only explicitly configured MCP-originated input operations are accepted.
- Validate types, finite numeric values, units, ranges, freshness, and source identity at the host boundary.
- Apply bounded payload sizes and timeouts consistent with AniCraft's command/MCP conventions.
- Do not execute arbitrary agent-provided code as part of controller evaluation.
- Do not let MCP access bypass AniCraft command validation, permission checks, or undo.
- Read-only preview must remain read-only.
- External provider failure must not block native controller authoring or native-only evaluation.

## 13. Acceptance criteria

- [ ] Smart Controller is a native AniCraft tool, not merely a command plug-in dialog.
- [ ] Its UI follows AniCraft's existing Tools, Properties, theme, localization, accessibility, keyboard, and status/undo conventions.
- [ ] Controller definitions are document-owned and versioned.
- [ ] Native UI and MCP invoke the same registered command contracts.
- [ ] No separate Smart Controller MCP server or protocol is introduced.
- [ ] MCP inputs are explicit, validated, attributed, and freshness-aware.
- [ ] Native-only controllers work without an MCP server.
- [ ] Evaluation uses immutable snapshots and remains deterministic.
- [ ] Preview does not mutate authored document state.
- [ ] Apply operations, if implemented, are explicit and undoable.
- [ ] Playback support is not claimed until a native evaluation hook exists.
- [ ] Form Deform evaluation and behavior remain independent.

## 14. Evidence and references

Host behavior was checked against the AniCraft fork's current documentation:

- [AniCraft UI guidelines](https://github.com/Mujshanan/anicraft/blob/main/docs/ui-guidelines.md) — left-side Tools, central stage, bottom timeline, right-side Properties/Library/History, command palette, visual tokens, accessibility and keyboard conventions.
- [AniCraft MCP guide](https://github.com/Mujshanan/anicraft/blob/main/docs/mcp.md) — the five generic MCP tools and their use of the existing command system.
- [AniCraft plug-in API](https://github.com/Mujshanan/anicraft/blob/main/docs/plugins.md) — WASM command plug-ins, manifest parameter dialogs, fresh instance per run, and current lack of panel/other UI plug-ins.
- [Forma Deform SDK audit](ANICRAFT_SDK_AUDIT.md) — verified command plug-in boundary and current host capability gaps.

This proposal records the intended architecture. It does not claim that the native tool, document storage, external-input commands, target application, or playback hook have been implemented.
