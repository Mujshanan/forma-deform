# Smart Controller development plan

**Status:** Native-tool architecture proposed; host integration not yet implemented  
**Host:** [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft)  
**Current implementation:** Host-independent Rust evaluator plus a WASM command-plug-in prototype  
**Target product:** First-party native AniCraft editor tool, aligned with AniCraft's UI and command/MCP architecture

## 1. Product direction

Smart Controller's final user-facing form is a **native AniCraft tool**, not a command plug-in dialog.

It should appear in AniCraft's existing Tools area and use the existing Properties experience for contextual controller configuration. It must follow AniCraft's tool activation, selection, theme, keyboard, accessibility, localization, status, and undo conventions. Do not create a parallel panel framework or a visually independent application.

The current `plugins/smart-controller` WASM command plug-in remains a prototype/compatibility path while the native host integration is designed and implemented. Its manifest is not the canonical controller model.

See [Native Tool and MCP Integration Proposal](SMART_CONTROLLER_NATIVE_TOOL_PROPOSAL.md) for the full architecture, host dependencies, command surface, phases, and acceptance criteria.

## 2. Architecture boundary

- `crates/smart-controller-core` owns the host-independent controller model, validation, mapping, dependency evaluation, composition, typed outputs, and diagnostics.
- AniCraft's native tool owns user interaction and authoring workflow, but not evaluation semantics.
- AniCraft's document model owns durable, versioned controller definitions.
- AniCraft's command registry owns validated, undoable document operations and read-only inspection operations.
- Native and external/MCP input providers resolve data into the same immutable, provider-independent input snapshot.
- AniCraft MCP exposes registered controller operations through its existing generic tools. Do not create a separate Smart Controller MCP server or protocol.
- Host-specific target adapters and frame-evaluation integration belong in AniCraft, not in the host-independent core.

## 3. MCP alignment

AniCraft already provides `anicraft_commands`, `anicraft_run`, `anicraft_run_script`, `anicraft_document`, and `anicraft_render`.

Smart Controller commands must be registered in AniCraft's existing command registry so they are available to the native UI, CLI/scripts, and MCP through the same contracts. MCP is an external input/control path, not a second evaluator and not a continuous playback subscription.

An MCP client may submit an explicitly configured input value through a registered AniCraft command. The host validates the source, InputId, type, units, freshness, and value before constructing the input snapshot.

## 4. Current V1 behavior to preserve

- Slider mapping with deterministic on-demand preview.
- Read evaluated driver angle through verified `bone.info` query when manual input is absent.
- Explicit `manual_value` for deterministic preview and tests.
- Finite rotation endpoints; reversed endpoints remain valid.
- Explicit Exclusive `[start, end)` and Inclusive `[start, end]` frame endpoint policies.
- Missing/malformed host input remains an error; never silently replace it with zero.
- Preview does not write timeline, bones, geometry pins, or document state.

The existing V1 command plug-in may remain available during migration. Do not remove it before native-tool parity and compatibility are verified.

## 5. State ownership

- **Durable:** Controller definitions, mappings, dependency references, and output bindings are versioned document data owned by AniCraft.
- **Transient:** Native, manual, and MCP-supplied input samples belong to an explicit provider/session lifecycle and carry provenance and freshness.
- **Per invocation:** Evaluation consumes immutable input and host-state snapshots. The same snapshots and configuration must produce deterministic results.

The current command plug-in ABI does not provide arbitrary document metadata or persistent per-instance state. Do not work around this with hidden files or plug-in-local storage.

## 6. Implementation sequence

1. **SC-NATIVE-0 — Host feasibility:** inspect actual AniCraft tool registry, Properties UI, document model, command registry, undo, frame evaluation, and MCP dispatch. Record exact source-backed integration points.
2. **SC-NATIVE-1 — Native tool shell:** register Smart Controller in the native Tools area and connect contextual settings to existing Properties UI.
3. **SC-NATIVE-2 — Document model and commands:** add versioned document-owned definitions and shared read/write command contracts with migration and undo tests.
4. **SC-NATIVE-3 — Providers and preview:** unify native, manual, and MCP-supplied inputs through immutable snapshots and the existing evaluator.
5. **SC-NATIVE-4 — Target integration:** implement only verified target operations with explicit preview/apply and undo semantics.
6. **SC-NATIVE-5 — Playback:** add continuous evaluation only after a first-party AniCraft frame-evaluation hook and lifecycle contract are approved.

The detailed phase exit criteria are in [SMART_CONTROLLER_NATIVE_TOOL_PROPOSAL.md](SMART_CONTROLLER_NATIVE_TOOL_PROPOSAL.md).

## 7. Non-negotiable boundaries

- Smart Controller and Form Deform evaluation remain independent.
- Do not automatically feed Smart Controller outputs into Form Deform or compose their outputs.
- Any cross-engine composition requires a separate explicit architecture decision.
- The core has no AniCraft, UI, MCP transport, WASM, or document dependency.
- Native UI and MCP use the same registered command contracts.
- No invented host command IDs, guessed document paths, or assumed streaming MCP API.
- No playback/scrubbing claims until the host evaluator integration exists.
- No source changes to AniCraft are implied by this repository plan; host-side implementation must be separately scoped and reviewed.
