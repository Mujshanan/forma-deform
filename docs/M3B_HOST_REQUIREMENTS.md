# M3B — AniCraft host extension requirements

**Status:** Requirements / RFC input only. No host API is assumed to exist until implemented and tested in AniCraft.
**Related:** UI_UX_DESIGN.md, ANICRAFT_SDK_AUDIT.md, and the AniCraft plug-in contract.

## 1. Current host boundary (rechecked against AniCraft main)

AniCraft already has a native dock/workspace system and a stage renderer. Those are application-owned extension points today, not public plug-in contributions.

The public WASM plug-in contract is command-oriented:
- Command plug-ins receive manifest-generated parameter dialogs and run atomically as one undo step.
- Plug-ins can query the document, active-layer elements, and verified commands such as bone.info, warp.info, and warp.pin_move.
- Every command run gets a fresh WASM instance. There is no persistent plug-in state.
- The documented plug-in guide explicitly lists panels and other UI plug-ins as not in v1.
- Frame scripts run at keyframe entry, get player state, and queue timeline actions; they cannot query the document or bone/warp commands. They are not a per-frame deformation evaluator.
- The app's dockable panels are implemented in the native UI. There is no documented plug-in panel registration or stage-overlay drawing contract.

Therefore M3B is a coordinated AniCraft host project, not a UI-only change in this repository. Do not call the existing dock API from a WASM plug-in or expose egui objects across the sandbox boundary.

## 2. Goals

1. Make Forma Deform feel native: a dockable inspector/controller panel, stage feedback, and normal AniCraft undo/redo.
2. Keep the AniCraft document authoritative. Controller bindings are document data, not plug-in-local files or transient WASM globals.
3. Keep evaluation deterministic and separate from UI rendering.
4. Make preview read-only and apply/bake explicit, atomic, and undoable.
5. Keep the sandbox narrow: plug-ins submit validated data and commands; they do not receive raw pointers, egui contexts, or unrestricted engine access.

## 3. Non-goals

- A second timeline, renderer, scene tree, document model, or undo stack.
- Running arbitrary guest WASM code for every rendered frame.
- Letting plug-ins mutate document structs directly.
- Exposing native egui APIs or host pointers to a guest module.
- Promising compatibility with arbitrary old document files without versioned migration.
- Supporting all deformation targets in the first host integration. First target: one Asset Warp pin driven by one bone angle.

## 4. Required host capabilities

### H1 — Versioned document-owned binding data

Add a document-owned, versioned collection of controller bindings. The initial schema should be deliberately narrow:

- Stable binding UUID (not array index).
- Source: armature/layer ID, stable bone ID, channel (keyed_angle or angle_plus_spring_swing), units.
- Target: warped element ID and pin ID, target channel (stage_position).
- Mapping: neutral angle, minimum/maximum input, X/Y gains, optional clamp policy.
- Enabled/disabled state and schema version.

Requirements:
- Bindings survive save/open, copy/duplicate semantics are explicitly defined, and unknown future fields/versions fail safely or are preserved according to document policy.
- IDs are validated against the document; stale references become diagnostics, never silently rebound.
- A document migration test covers a file with no binding field.
- Binding edits are represented by engine commands (controller_binding.add, .update, .remove, .list or equivalent) and form one undo step each.
- Commands validate source and target compatibility before commit.
- Document serialization tests prove round-trip stability and deterministic ordering.

### H2 — Evaluator integration

Add a pure engine evaluation stage that consumes authored document data plus the requested scene/frame and returns effective deformation values. It must not rewrite authored keyframes or binding definitions.

Requirements:
- Same document + same frame + same evaluator version produces the same output.
- Evaluate only enabled, valid bindings; report disabled/stale/invalid bindings as diagnostics.
- Resolve the bone's keyed angle and spring contribution explicitly; channel semantics are stored, never inferred.
- Define dependency ordering and reject cycles before evaluation.
- Bound work by document limits; malformed or excessive bindings must not stall playback.
- Evaluate from the same canonical frame path used by scrubbing, playback, export, and headless rendering.
- Add tests for exact keyframes, in-between frames, spring on/off, reversed ranges, missing bones, missing pins, cycles, and multiple bindings targeting one pin.
- Define deterministic conflict composition for multiple bindings to one target (initial implementation may reject duplicates).

### H3 — Stage overlay contribution

Add a host-owned overlay model for inspection and preview. The initial primitives are:
- Line segment / polyline.
- Circle or handle marker with screen-space hit radius.
- Text label / diagnostic marker.
- Ghost pin position and displacement vector.

Requirements:
- Geometry is expressed in stage coordinates; hit radii and label placement specify screen-space behavior.
- Overlay rendering is a view concern and never modifies the document.
- Overlay state is derived from a stable evaluation snapshot and is invalidated when selection, frame, source, target, or binding changes.
- Overlay contributions are clipped to the stage and respect high-DPI scaling, zoom, pan, theme, and accessibility contrast.
- A plug-in cannot draw outside the stage or register arbitrary native callbacks.
- First slice may render built-in Forma Deform overlay data from the engine; do not require guest WASM execution per frame.

### H4 — Dockable panel contribution

Use AniCraft's existing panel/dock/workspace infrastructure, but add a supported contribution boundary.

Preferred first implementation: a host-rendered declarative panel schema (sections, labels, numeric fields, choices, buttons, diagnostics, preview values), not guest-provided egui code.

Requirements:
- Panel contributions have stable IDs, display names, versioned schemas, and declared command/query capabilities.
- Panel controls dispatch validated engine commands or read-only queries; all persistent edits use the command registry.
- Panels participate in Window menu, dock/float/close/reopen, saved workspaces, focus traversal, accessibility naming, keyboard navigation, localization, and high-contrast behavior.
- Missing or incompatible contributions fail closed with a useful message and do not corrupt workspace preferences.
- Panel UI must not be required for document evaluation, export, or CLI operation.
- Panel state that is merely view state remains in preferences; binding data remains in the document.

### H5 — Preview / apply transaction

- Preview evaluates a snapshot and returns values/diagnostics without mutating authored data.
- Preview overlay is visually distinct from committed geometry.
- Apply/bake is a separate explicit command, revalidates source and target, and performs one atomic undoable document edit.
- Cancel leaves document bytes and undo history unchanged.
- A failed apply rolls back all writes.
- Tests compare document hashes and undo/redo outcomes for preview, cancel, success, and injected failure.

## 5. Suggested delivery sequence

| Slice | Host work | Forma Deform work | Exit criteria |
|---|---|---|---|
| M3B-0 contract | Agree on schema, ownership, limits, compatibility and command names | Keep adapter types ready; no speculative UI | RFC accepted in AniCraft |
| M3B-1 binding persistence | Document model, migration, commands, serialization | Add binding CRUD adapter and validation tests | Save/open + undo/redo tests pass |
| M3B-2 deterministic evaluation | Pure evaluator integrated into canonical frame path | Provide mapping/evaluation core and golden fixtures | Scrub/play/export agree on same frame |
| M3B-3 overlays | Stage overlay model and rendering/hit-testing | Supply overlay primitives from evaluated snapshots | Preview overlays work at zoom/DPI; no document mutation |
| M3B-4 dock panel | Panel contribution registry + host-rendered schema UI | Controller editor/diagnostics panel contribution | Dock/workspace/a11y tests pass |
| M3B-5 apply/bake | Atomic host commands and conflict policy | Target adapters and apply UX | One-step undo, rollback, save/reopen verified |

The order is intentional: persist and evaluate the model before building a panel that edits it. Overlay and panel UI must not become the source of truth.

## 6. Acceptance test matrix

- Legacy document without controller data opens unchanged.
- Add/update/remove binding, save, close, reopen, and inspect identical binding identity/settings.
- Undo and redo each binding edit as one step.
- Missing source bone or target pin yields a stable diagnostic and no crash.
- Scrub, playback, export, and CLI evaluate the same binding at the same frame.
- Preview/cancel leaves serialized document and undo history unchanged.
- Apply creates one undo step; undo restores exact prior document; redo reapplies exact result.
- Stage overlay is clipped and remains correctly aligned under zoom, pan, resize, and high-DPI scaling.
- Panel opens, closes, docks, floats, reopens, participates in saved workspaces and Ctrl+F6 focus order.
- Screen-reader names, keyboard-only operation, localization and high-contrast checks pass.
- Malformed schemas, unknown versions, excessive bindings, cycles and duplicate target writes fail safely.

## 7. Decisions required from AniCraft maintainers

1. Should bindings be a first-party document feature or a generic extension-data registry? First-party typed data is preferred for referential integrity and evaluator performance.
2. Should the initial panel be a declarative host-rendered schema or a built-in native panel owned by AniCraft? Declarative schema is preferred only if it meets accessibility and interaction requirements without exposing egui.
3. Which canonical evaluation layer should consume the binding graph so that stage, player, export and CLI remain consistent?
4. What is the supported lifecycle for registering extension panels and stage overlays across desktop and web builds?
5. What is the policy for multiple bindings writing the same pin/property? Rejecting duplicates is the safest initial behavior.

## 8. Out of scope until these contracts exist

- Live playback controller UI.
- Persistent binding claims in the Forma Deform plug-in alone.
- Arbitrary custom drawing callbacks from guest WASM.
- Full rigging graph, IK, soft selection, lattice, multi-axis controllers and pose sets.
- A custom standalone browser UI pretending to be the AniCraft workspace.
