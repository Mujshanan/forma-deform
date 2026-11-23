# UI-02 — Standalone Studio Source and UI Architecture Audit

**Status:** Audit complete; initial standalone shell subsequently implemented in UI-03.1  
**Branch:** `feat/ui-01-deformation-authoring-workspace`  
**Audit scope:** Forma Deform source tree, current browser demo, existing styling, project documentation, and the AniCraft 360 Studio repository.

## 1. Executive findings

1. Forma Deform currently has a browser-based rectangle/timeline prototype, not a production standalone desktop Studio.
2. The prototype's TypeScript core is a rectangle geometry/document/timeline model. It is not the deformation authoring UI described in UI-01.
3. The existing CSS provides a useful visual starting point: dark neutral surfaces, warm amber accent, compact toolbar controls, central stage, and inspector-friendly spacing. It is not yet a reusable design system.
4. The AniCraft 360 Studio repository contains only `README.md` and `DEVELOPMENT.md`. It is a proposal, not an implemented UI kit or application shell.
5. No separate AniCraft application repository was found among the connected repositories available for this audit. Therefore no actual AniCraft component APIs, host extension points, or source styles can be claimed as verified.
6. The correct next implementation is a standalone Forma Deform Studio shell. It must not import the AniCraft app or depend on unverified AniCraft host APIs.

## 2. Evidence from Forma Deform

### Project and runtime

- `package.json` identifies a private Bun/TypeScript project.
- The current scripts run a Bun development server, bundle `src/demo/main.ts`, run Bun tests, and typecheck TypeScript.
- `src/demo/server.ts` serves `index.html`, `style.css`, and a browser bundle.
- The root `index.html` identifies the current page as **Forma Deform — Rectangle Tween**.
- There is no desktop runtime/bootstrap in the inspected project tree. The current application is a browser demo.

### Current UI

The current page contains:
- A top bar with Save/Open JSON controls.
- A two-button Draw/Select tool rail.
- A single canvas stage.
- A bottom timeline with frame scrubbing, playback, and keyframe controls.

The UI has no deformation inspector, target list, pin tools, offset drivers, point-array/binding panel, cage editor, Bézier editor, or tessellation controls.

### Current TypeScript model

- `src/core/geometry.ts` models a four-point rectangle and basic rectangle operations.
- `src/core/timeline.ts` models rectangle keyframes and linear interpolation.
- `src/core/document.ts` persists a version-1 rectangle/timeline JSON document.
- `src/index.ts` exports those demo-oriented modules.

This model must not be promoted into the authoritative deformation document model. The deformation UI should consume the existing deformation core contracts through explicit adapters.

### Existing visual language

`style.css` already establishes:
- Dark blue/charcoal workspace surfaces.
- Amber/orange primary accent and active tool state.
- Compact toolbar buttons and a narrow left tool rail.
- Central canvas with a subtle grid.
- Clear separation between workspace and bottom panel.
- Responsive adjustments for narrower browser widths.

Reuse the visual direction and refine it into tokens and small reusable UI primitives. Do not copy the rectangle demo's timeline into the production deformation workspace.

## 3. AniCraft / 360 Studio findings

The inspected `Mujshanan/anicraft-360-studio` repository currently contains only:
- `README.md`
- `DEVELOPMENT.md`

Its documentation describes a proposed character-focused workspace, not a shipped component library or application shell. It does not provide actual toolbar, dock, inspector, viewport, command, or theme source code that Forma Deform can import.

No other AniCraft application repository was present in the connected repository listing used for this audit. The audit therefore cannot verify an existing AniCraft design-token package or component API.

**Decision:** Treat “same as AniCraft” as a visual and interaction compatibility goal, not as a dependency on an unavailable application package. The shared design kit boundary must remain independent and reusable by both applications when the AniCraft implementation exists.

## 4. Architecture recommendation

### Standalone Studio layers

1. **Forma Deform Core** — pure geometry/deformation algorithms, validation, bindings, and evaluation contracts.
2. **Studio application shell** — standalone project lifecycle, workspace layout, selection, viewport navigation, command dispatch, and undo/redo.
3. **Deformation UI modules** — Pin & Offset Drivers; Point Arrays & Binding; Cage/Bézier/Tessellation tools.
4. **Rendering/interaction adapters** — convert domain data into viewport overlays and pointer/keyboard gestures without owning deformation semantics.
5. **Shared visual system boundary** — theme tokens and reusable controls/panels, independent from the AniCraft application.
6. **Future AniCraft adapter** — optional integration boundary, added only after actual AniCraft APIs are available and verified.

### Suggested source organization

This is a target structure, not a claim that these files already exist:

```text
src/
  app/
    bootstrap/
    shell/
    commands/
    project/
    selection/
  ui/
    tokens/
    primitives/
    panels/
    toolbar/
  viewport/
    renderer/
    navigation/
    overlays/
    interaction/
  features/
    pins/
    point-arrays/
    bindings/
    cages/
    bezier/
    tessellation/
  adapters/
    core/
    project-format/
    anicraft/       # optional; no runtime requirement
  demo/             # temporary prototype only
  core/             # current rectangle demo model; do not treat as domain core
```

Do not create all directories as empty scaffolding. Introduce modules only with the first concrete implementation that needs them.

## 5. UI shell decisions

| Region | Forma Deform Studio responsibility |
|---|---|
| Top bar | Studio identity, project actions, undo/redo, commands, viewport/workspace options |
| Left rail | Select, Pin, Point Array, Bind, Cage, Bézier, Tessellate |
| Center viewport | Target geometry, handles, pins, cages, paths, arrays, overlays |
| Right inspector | Contextual target/tool properties, validation, driver/binding state |
| Bottom status panel | Selection summary, diagnostics, coordinate information, operation status |

There is no general animation timeline in the standalone Studio. If a future feature needs time-varying inputs for preview, it must be a focused preview control, not a general-purpose animation timeline or keyframe editor.

## 6. Preserve these domain boundaries

- Bone and cage deformation remain separate capabilities.
- Smart Controller evaluation and pin deformation remain independent.
- No implicit composition or summing of independent deformation outputs.
- Preview must be deterministic and non-mutating.
- Persistent edits must be explicit commands with undo/redo.
- Stable IDs, topology validation, binding freshness, and authored-versus-derived geometry distinctions remain core-owned contracts.
- The UI must not replace core validation with presentation-layer checks.

## 7. Migration plan

| Step | Work | Result |
|---|---|---|
| UI-02.1 | Record source and architecture audit | This document |
| UI-02.1 | Record source and architecture audit | Complete |
| UI-03.1 | Replace default rectangle-demo page with standalone shell | Implemented; verification pending |
| UI-03.2 | Harden viewport and app boundaries | Viewport behavior, responsive layout, and entrypoint contract reviewed |
| UI-04 | Extract theme tokens and reusable primitives | Shared visual language without AniCraft runtime dependency |
| UI-05A | Pin & Offset Drivers UI and core adapter | Real pin/driver data and non-mutating preview |
| UI-05B | Point Arrays & Binding UI and core adapter | Validated binding lifecycle and stale diagnostics |
| UI-05C | Cage, Bézier, and tessellation editors | Topology-safe editing and non-destructive sampling |
| UI-06 | Project persistence, undo/redo, and UX regression coverage | Reliable authoring workflow |

The rectangle demo remains as legacy source and tests under `src/demo`, `src/core`, and `tests`. The default server and build entry now target `src/studio/main.ts`. The old rectangle/timeline model is not imported by the Studio shell and must not become its persistent domain model.

## 8. Documentation alignment

The README and DEVELOPMENT product boundary now describe Forma Deform as a standalone Studio with an optional future AniCraft adapter. The workspace contract and implementation status are linked from those documents.

## 9. Audit completion

**UI-02 audit: complete.** The audit itself changed no engine code. The subsequent UI-03.1 implementation changed only the application entry HTML, Studio CSS/TypeScript shell, development server entrypoint, and TypeScript build entrypoint. No Rust crate, plugin, deformation contract, or engine source was changed. Tests/builds were not run for that shell commit.

**Next:** UI-03.2 viewport and application-boundary hardening, then UI-04 reusable theme tokens and primitives.
