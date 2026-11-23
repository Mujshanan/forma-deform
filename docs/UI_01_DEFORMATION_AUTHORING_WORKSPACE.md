# UI-01 — Deformation Authoring Workspace

**Status:** Design contract  
**Scope:** Native AniCraft UI for Pin & Offset Drivers, Point Arrays & Binding, and Cages, Bézier Splines & Vector Tessellation.  
**Repository role:** Forma Deform defines tool behavior and deformation-domain contracts. AniCraft owns the actual workspace shell, stage, timeline, document lifecycle, rendering, and undo/redo.

## 1. Product decision

The production interface is a native AniCraft tool context. The existing TypeScript rectangle/timeline demo is an isolated prototype; it is not the production UI and must not be extended into a competing editor.

The three feature groups are authoring surfaces over existing or planned core capabilities. They are not three new deformation engines.

- **Pin & Offset Drivers** expose pin placement, offsets, neutral capture, driver configuration, and deterministic preview.
- **Point Arrays & Binding** expose point-array authoring, source-to-cage binding, binding inspection, and stale-binding repair.
- **Cages, Bézier Splines & Vector Tessellation** expose cage construction/editing, curve editing, and non-destructive tessellation controls/overlays.

Bone identity, transforms, and the authoritative rig remain AniCraft-owned. Smart Controller evaluation and Form Deform pin evaluation remain independent. The UI must not imply automatic composition between them.

## 2. Workspace layout

Use AniCraft's existing dockable workspace and stage:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ AniCraft menus · tool context · command palette                              │
├────────────────┬──────────────────────────────────────┬─────────────────────┤
│ Deform tools   │ Existing AniCraft stage              │ Properties          │
│                │                                      │                     │
│ Select         │ Artwork / selected target            │ Target & capability │
│ Pin            │ Pins / offsets / handles             │ Pin / driver        │
│ Point Array    │ Cage vertices / edges / faces        │ Array & binding     │
│ Bind           │ Bézier paths and handles             │ Cage / curve        │
│ Cage           │ Tessellation preview overlay         │ Tessellation        │
│ Curve          │ Validation and stale-state overlays  │ Diagnostics         │
│ Tessellate     │                                      │ Preview values      │
├────────────────┴──────────────────────────────────────┴─────────────────────┤
│ Existing AniCraft timeline, layers, and playhead                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

The tool strip is capability-aware: hide or disable tools that the selected target or host adapter cannot support. The Properties panel changes context with the active selection/tool. Do not add a second scene tree, timeline, renderer, or document panel.

## 3. Tool group A — Pin & Offset Drivers

### Stage interactions

- Select, add, move, and remove pins on compatible targets.
- Display each pin's authored/rest position and current preview position distinctly.
- Edit offset vectors numerically or by dragging a visible offset handle.
- Show target bounds and the active coordinate-space indicator.
- Support multi-selection only where the host exposes stable target/pin identities.

### Properties

- Pin identity and target identity (read-only stable IDs where appropriate).
- Neutral/rest position and offset vector.
- Driver source and source identity.
- Input axis/component, neutral input, range, normalization, mapping, easing, and gain.
- Axis constraints and validation diagnostics.
- Preview input, normalized value, mapped output, and resulting displacement.

### Workflow

1. Select a compatible target and choose Pin.
2. Add/select a pin and set or capture its neutral position.
3. Choose a driver source and configure input/range/mapping.
4. Preview explicitly; preview must not mutate authored geometry or create keyframes.
5. Save authored settings through the host's supported document boundary.
6. Apply/bake only through an explicit undoable AniCraft command.

The UI must label Smart Controller preview and pin deformation as separate evaluations. No implicit summing, chaining, or multi-source composition is permitted without a separately approved composition contract.

## 4. Tool group B — Point Arrays & Binding

### Stage interactions

- Create/edit point arrays using explicit count, spacing, orientation, and coordinate-space settings.
- Select individual points, ranges, or the complete array.
- Display source points, bound cage features, and binding status with distinct visual states.
- Provide a guided source-to-cage binding operation.
- Make stale, missing, unbound, clamped, and extrapolated states visually distinguishable and textually diagnosable.

### Properties

- Array identity, count, spacing, orientation, and source coordinate descriptor.
- Source target/feature identity and revision.
- Cage identity, topology revision, and selected face/feature.
- Binding policy and outside-cage policy.
- Binding status, validation result, and freshness diagnostics.
- Rebind/unbind actions and a read-only mapping summary.

### Workflow

1. Create or select a point array.
2. Select source features and a validated cage.
3. Run binding and inspect per-point results.
4. Review outside-cage handling and any diagnostics.
5. Confirm and persist the binding through AniCraft's supported host storage contract.
6. Revalidate when source identity/revisions/fingerprint or cage topology changes.

A successful JSON parse is not a valid binding. The UI may show a binding as active only after the core validation contract succeeds against the current source snapshot and cage certificate. A stale binding must never be silently used.

## 5. Tool group C — Cages, Bézier Splines & Vector Tessellation

### Cage editing

- Create regular lattice cages and supported irregular cages.
- Add, remove, and move control points using stable IDs.
- Select vertices, edges, and faces; expose topology-aware editing commands.
- Display invalid topology/embedding diagnostics before allowing binding/evaluation.
- Distinguish rest cage geometry from deformed control state.

### Bézier editing

- Preserve authored Path2D commands and Bézier handles.
- Display anchors and incoming/outgoing handles.
- Support selection and direct manipulation only where the host's canonical path editor contract permits it.
- Do not replace authored curves with tessellated polylines during editing or persistence.

### Vector tessellation

- Offer a non-destructive preview overlay for sampled curve segments.
- Expose source-space deviation tolerance, maximum subdivision depth, and sample budgets.
- Show sample count, capped/budget-exceeded status, and validation diagnostics.
- Keep tessellation output derived and disposable; never make it the authoritative authored path.
- Clearly label the current sampling tolerance as source-space curve-to-chord deviation, not a guarantee of deformed-output-space error.

## 6. Shared interaction and state rules

- Use stable host target, source, pin, point, cage, and feature identities; never use viewport indices as persistent identities.
- Keep rest/authored geometry, evaluated preview geometry, and tessellation overlays visually and semantically separate.
- Preview is read-only and deterministic. It must not write keyframes or mutate the authored document.
- Persistent changes go through AniCraft commands and participate in its undo/redo model.
- Use the existing AniCraft timeline and playhead. Do not simulate native playback by repeatedly issuing plugin commands.
- Revalidate bindings when their source/cage freshness dependencies change.
- Surface unsupported host capabilities as explicit diagnostics rather than silently falling back.
- Provide keyboard-accessible tool selection and numeric property editing; dragging must not be the only way to author values.

## 7. Implementation sequence

| Step | Deliverable | Acceptance gate |
|---|---|---|
| UI-01 | This workspace and interaction contract | Product boundaries and workflows reviewed |
| UI-02 | AniCraft host UI capability audit | Verified stage/tool registration, selection, property panel, command, undo, storage, and overlay APIs |
| UI-03 | Native workspace shell and capability-aware tool registration | Runs inside AniCraft; no duplicate app/document/timeline/renderer |
| UI-04A | Pin & Offset Drivers panel and stage tools | Pin edits and driver preview use existing contracts; no implicit composition |
| UI-04B | Point Arrays & Binding panel | Validated bind/unbind/rebind lifecycle and stale diagnostics |
| UI-04C | Cage, Bézier, and tessellation tools | Topology-safe editing and non-destructive curve/tessellation display |
| UI-05 | Host integration and UX regression suite | Undo/redo, persistence, stale-state, target capability, and preview non-mutation tests pass |

UI-04A/B/C may be developed independently after UI-03, but each must use the same workspace conventions and host command/property APIs.

## 8. Out of scope

- A standalone Forma Deform desktop editor.
- A second document model, timeline, renderer, layer tree, cache, or undo stack.
- Replacing AniCraft's native bone/rig UI or Asset Warp UI.
- Implicitly combining Smart Controller and pin-deformation outputs.
- Treating sampled/tessellated geometry as the source of truth.
- Claiming live playback evaluation until AniCraft exposes and verifies a native evaluator hook.

## 9. Immediate next task

**UI-02 — AniCraft Host UI Capability Audit.** Inspect the actual AniCraft host repository and verify the extension points needed for native tool registration, stage overlays/input, selection, dockable properties, commands, undo/redo, and durable plugin-owned data. Do not implement UI against guessed APIs. If a required host capability is absent, document the smallest host-side contract needed before proceeding.
