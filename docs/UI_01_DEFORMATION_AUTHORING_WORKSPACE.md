# UI-01 — Forma Deform Studio Workspace

**Status:** Product and UI architecture contract  
**Scope:** Standalone desktop authoring workspace for Pin & Offset Drivers, Point Arrays & Binding, and Cages, Bézier Splines & Vector Tessellation.  
**Product boundary:** Forma Deform Studio is independently launchable. Forma Deform Core owns deformation algorithms and domain contracts. AniCraft is a future host/integration target, not a runtime dependency.

## 1. Product decision

Forma Deform Studio uses the same visual language and interaction conventions intended for AniCraft: consistent typography, spacing, colors, iconography, tool affordances, selection states, inspector patterns, viewport navigation, command presentation, and panel behavior.

This means a familiar AniCraft-style workspace, **not** a copy of AniCraft's complete editor. Forma Deform contains only deformation authoring tools. It does not include general drawing/painting, character rig authoring, a general animation timeline, audio/video, or unrelated asset-management features.

The 360 Studio repository currently describes a proposed workspace and contains no implemented UI component system to reuse. Its intended workspace conventions may inform the design, but it is not treated as an existing UI implementation.

### Shared UI architecture

- Define a small, host-independent **AniCraft UI design kit**: design tokens, foundational controls, panel primitives, toolbars, icons, typography, selection/hover/focus states, and interaction conventions.
- The design kit must be independently consumable. Forma Deform Studio must not import or launch the AniCraft application to render its UI.
- AniCraft and Forma Deform Studio may both consume the same kit when the AniCraft application is implemented.
- Until a real shared package exists, keep the Forma Deform implementation behind clean UI boundaries so the kit can be extracted/reused without coupling the deformation domain to a host.
- Do not invent or depend on AniCraft host APIs that have not been inspected and verified.

## 2. Standalone workspace layout

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Forma Deform Studio · File · Edit · View · Tools · Help       Commands      │
├────────────────┬──────────────────────────────────────┬─────────────────────┤
│ Deform tools   │ Viewport                             │ Inspector           │
│                │                                      │                     │
│ Select         │ Artwork / selected target            │ Target & capability │
│ Pin            │ Pins / offsets / handles             │ Pin / driver        │
│ Point Array    │ Array points and binding overlays     │ Array & binding     │
│ Bind           │ Cage vertices / edges / faces         │ Cage / topology     │
│ Cage           │ Bézier paths and handles              │ Curve              │
│ Curve          │ Tessellation preview overlay          │ Tessellation        │
│ Tessellate     │ Validation / stale-state overlays     │ Diagnostics         │
├────────────────┴──────────────────────────────────────┴─────────────────────┤
│ Contextual status · selection details · preview/validation messages         │
└─────────────────────────────────────────────────────────────────────────────┘
```

The shell is standalone and focused. It may provide project open/save, viewport navigation, selection, command history, and undo/redo appropriate to deformation authoring. It must not grow a general animation timeline, general scene editor, or duplicate general-purpose compositor.

## 3. Tool group A — Pin & Offset Drivers

### Viewport interactions

- Select, add, move, and remove pins on compatible targets.
- Display authored/rest positions and evaluated preview positions distinctly.
- Edit offset vectors numerically or by dragging an offset handle.
- Show target bounds and active coordinate-space information.
- Support multi-selection only when stable target and pin identities are available.

### Inspector

- Pin and target identity.
- Neutral/rest position and offset vector.
- Driver source and source identity.
- Input axis/component, neutral input, range, normalization, mapping, easing, and gain.
- Axis constraints and validation diagnostics.
- Preview input, normalized value, mapped output, and resulting displacement.

### Workflow

1. Select a compatible target and choose Pin.
2. Add/select a pin and set or capture its neutral position.
3. Choose a driver source and configure input/range/mapping.
4. Run preview explicitly; preview must not mutate authored geometry or create keyframes.
5. Save authored settings through Forma Deform Studio's project/document boundary.
6. Apply/bake only through an explicit undoable command.

Smart Controller evaluation and pin deformation remain independent. The UI must not imply automatic summing, chaining, or multi-source composition.

## 4. Tool group B — Point Arrays & Binding

### Viewport interactions

- Create/edit point arrays with explicit count, spacing, orientation, and coordinate-space settings.
- Select individual points, ranges, or the complete array.
- Display source points, bound cage features, and binding status distinctly.
- Provide a guided source-to-cage binding operation.
- Distinguish stale, missing, unbound, clamped, and extrapolated states.

### Inspector

- Array identity, count, spacing, orientation, and source coordinate descriptor.
- Source target/feature identity and revision.
- Cage identity, topology revision, and selected face/feature.
- Binding policy and outside-cage policy.
- Binding status, validation result, and freshness diagnostics.
- Rebind/unbind actions and read-only mapping summary.

### Workflow

1. Create or select a point array.
2. Select source features and a validated cage.
3. Run binding and inspect per-point results.
4. Review outside-cage handling and diagnostics.
5. Confirm and persist the binding through the Studio project/document contract.
6. Revalidate when source identity/revisions/fingerprint or cage topology changes.

A successful JSON parse is not a valid binding. The UI may show a binding as active only after core validation succeeds against the current source snapshot and cage certificate. A stale binding must never be silently used.

## 5. Tool group C — Cages, Bézier Splines & Vector Tessellation

### Cage editing

- Create regular lattice cages and supported irregular cages.
- Add, remove, and move control points using stable IDs.
- Select vertices, edges, and faces; expose topology-aware editing commands.
- Display invalid topology/embedding diagnostics before binding/evaluation.
- Distinguish rest cage geometry from deformed control state.

### Bézier editing

- Preserve authored Path2D commands and Bézier handles.
- Display anchors and incoming/outgoing handles.
- Support direct manipulation only where the canonical path-editing contract permits it.
- Never replace authored curves with tessellated polylines during editing or persistence.

### Vector tessellation

- Offer a non-destructive preview overlay for sampled curve segments.
- Expose source-space deviation tolerance, maximum subdivision depth, and sample budgets.
- Show sample count, capped/budget-exceeded status, and diagnostics.
- Keep tessellation output derived and disposable; it is never the authoritative authored path.
- Label the sampling tolerance as source-space curve-to-chord deviation, not a guarantee of deformed-output-space error.

## 6. Shared interaction and state rules

- Use stable target, source, pin, point, cage, and feature identities; never use viewport indices as persistent identities.
- Keep authored/rest geometry, evaluated preview geometry, and tessellation overlays semantically separate.
- Preview is read-only and deterministic. It must not write keyframes or mutate authored project data.
- Persistent changes go through explicit commands and participate in the Studio undo/redo model.
- Revalidate bindings when source/cage freshness dependencies change.
- Surface unsupported capabilities as explicit diagnostics rather than silently falling back.
- Provide keyboard-accessible tool selection and numeric property editing; dragging must not be the only authoring method.
- Keep UI state and rendering adapters separate from deformation-domain logic.
- Do not create a second production deformation engine or introduce implicit composition between independent engines.

## 7. Implementation sequence

| Step | Deliverable | Acceptance gate |
|---|---|---|
| UI-01 | This product and workspace contract | Standalone boundary and shared visual language agreed |
| UI-02 | Inspect current Forma Deform app structure and AniCraft design references | Existing UI stack, demo limitations, and reusable design conventions verified; no guessed host APIs |
| UI-03 | Standalone Studio shell and viewport | Independently launches; focused panels; no AniCraft runtime dependency |
| UI-04 | Shared UI-kit boundary and theme tokens | Components/tokens are reusable without importing the AniCraft application |
| UI-05A | Pin & Offset Drivers UI | Uses existing contracts; preview is non-mutating; no implicit composition |
| UI-05B | Point Arrays & Binding UI | Validated bind/unbind/rebind lifecycle and stale diagnostics |
| UI-05C | Cage, Bézier, and tessellation tools | Topology-safe editing and non-destructive curve/tessellation display |
| UI-06 | Core integration and UX regression suite | Undo/redo, persistence, stale-state, capability, and preview non-mutation tests pass |
| UI-07 | Optional AniCraft adapter | Integration is isolated behind an adapter; Studio remains independently usable |

Feature panels may be developed independently after UI-03, but must use the same workspace conventions and command/inspector patterns.

## 8. Out of scope

- Requiring AniCraft to launch or use Forma Deform Studio.
- A general-purpose drawing/painting application or animation editor.
- A general animation timeline, audio/video tools, or unrelated asset library.
- Replacing AniCraft's native bone/rig UI.
- Implicitly combining Smart Controller and pin-deformation outputs.
- Treating sampled/tessellated geometry as the source of truth.
- Claiming a host integration before the actual AniCraft repository and extension APIs are identified and verified.

## 9. Current implementation and next task

**UI-02 source/UI audit: complete. UI-03.1 standalone shell: implemented as an initial browser-hosted workspace shell.** The root application now presents the deformation-focused tool rail, viewport, contextual inspector, status area, and basic viewport pan/zoom. The previous rectangle/timeline implementation remains under `src/demo` and is no longer the default application entry point.

The current shell is navigation and viewport scaffolding only. It does not yet load geometry, create deformation data, call the Rust engines, persist Studio projects, or provide functional feature editors. These capabilities must be integrated incrementally through explicit adapters without changing engine semantics.

**Next:** UI-03.2 — viewport and application-boundary hardening, followed by UI-04 — reusable theme tokens and UI primitives. Keep the rectangle demo and its tests untouched until a separately reviewed retirement decision.
