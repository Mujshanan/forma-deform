# Forma Deform

**Forma Deform** is a small, reusable 2D shape-deformation and keyframe-interpolation library. It is being developed as a foundational component of the wider Forma Studio ecosystem.

The project starts deliberately small: draw a rectangle, edit its corners at different points on a timeline, and interpolate the corner positions to produce an animation. The geometry and animation core should remain independent of the demo editor so it can later be embedded in Forma Studio or reused by other applications.

> **Current status:** The TypeScript rectangle/timeline demo is implemented on `feat/rectangle-deformation-mvp`. A separate Smart Controller v1 workstream is now being developed on `feat/smart-controller-v1`.

> **Project direction:** The existing TypeScript demo is retained as a standalone geometry experiment. The Smart Controller workstream now has an initial native Rust core and versioned binding-set JSON model; a thin WASM adapter follows the verified AniCraft SDK contract. Native test execution is pending. See [the Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md) and the [AniCraft SDK capability audit](docs/ANICRAFT_SDK_AUDIT.md).

## Goals

- Provide a lightweight TypeScript-first 2D geometry and deformation core.
- Establish a clean separation between geometry, animation, rendering, and editor interaction.
- Support rectangle creation and corner editing as the first end-to-end workflow.
- Store editable geometry in keyframes and interpolate between them.
- Render through the browser's native Canvas 2D API.
- Keep the initial dependency footprint very small.
- Leave room for future shape types and deformation systems without building them prematurely.

## Non-goals for the first milestone

The initial milestone is not a complete animation application or a general-purpose vector editor. It does not include Bézier paths, arbitrary polygons, topology changes, bones, skinning, inverse kinematics, mesh deformation, layers, audio, effects, or a production-grade asset pipeline.

Rust and WebAssembly are not part of the initial implementation. They may be evaluated later if profiling demonstrates a real need.

## Initial feature scope

1. A single rectangle on an HTML Canvas.
2. Draw tool: click and drag to create the rectangle.
3. Select tool: select and move the rectangle.
4. Corner editing: select and move any of the four corners.
5. A basic frame-based timeline with keyframes.
6. Store rectangle geometry at each keyframe.
7. Linear interpolation of corresponding corner positions.
8. Timeline scrubbing and basic playback.
9. Save and load a small, versioned project document.

The first geometry representation uses four ordered points rather than only position, width, and height. This allows a rectangle to become a general quadrilateral when its corners are edited independently.

## Technology direction

- **Language:** TypeScript.
- **Rendering:** Native HTML Canvas 2D.
- **UI:** Minimal HTML and CSS; no UI framework initially.
- **Runtime / tooling:** Bun and TypeScript, with Bun's built-in test runner and bundler.
- **Build and tests:** `bun install`, `bun run dev`, `bun test`, `bun run typecheck`, and `bun run build`.
- **Future native acceleration:** Rust/WASM only after measurement justifies it.

## Repository layout (target)

```text
src/
  core/
    point.ts
    rectangle.ts
    interpolate.ts
  animation/
    keyframe.ts
    timeline.ts
    playback.ts
  canvas/
    renderer.ts
    coordinates.ts
    pointer-events.ts
  editor/
    tools/
      draw.ts
      select.ts
    main.ts
demo/
tests/
docs/
  DEVELOPMENT_PLAN.md
```

This is a proposed starting structure, not a requirement to create every file at once. Keep modules small and introduce abstractions only when a concrete feature needs them.

## Reference project: AniCraft

The project uses [AniCraft](https://github.com/storytold/anicraft) as a workflow and architecture reference—not as a code dependency or a feature checklist. AniCraft is a broader Rust vector-animation studio with a stage, drawing tools, keyframes, timeline, and tweening. Its documentation and implementation provide useful examples of separating a document model, deterministic frame evaluation, editor commands, rendering, and timeline interaction.

For Forma Deform, borrow only the principles that fit this smaller library:

- Treat the document as data and make frame evaluation a deterministic operation.
- Keep geometry and animation evaluation independent of the editor and renderer.
- Use a stage/canvas plus a frame-based timeline as the basic authoring workflow.
- Make drawing and editing operations explicit, so they can later support undo/redo or automation.
- Build and test in small vertical slices.

Do not copy AniCraft's Rust/egui application architecture, full editor feature set, file format, or shape-tween implementation. Forma Deform remains TypeScript-first, Canvas 2D, rectangle-only for its first milestone.

## Development plans

- [Smart Controller v1](docs/SMART_CONTROLLER_PLAN.md) — authoritative plan for the AniCraft plugin workstream.
- [Rectangle deformation demo](docs/DEVELOPMENT_PLAN.md) — original TypeScript/Canvas prototype plan and implementation history.

## Design principles

- **Core first:** geometry and interpolation must not depend on Canvas or editor UI.
- **Small steps:** each phase should produce a runnable, testable result.
- **Explicit geometry:** preserve point ordering and validate geometry before interpolation.
- **Deterministic evaluation:** the same project state and frame must produce the same geometry.
- **No silent topology changes:** incompatible keyframes must be rejected or handled by an explicit future conversion operation.
- **Measure before optimizing:** do not add Rust/WASM or complex data structures without evidence.
- **Reusable boundaries:** Forma Studio should be able to consume the core without depending on the demo editor.

## Status and contribution

The repository contains an implemented TypeScript rectangle/timeline prototype and an active Smart Controller v1 workstream. The Smart Controller plan is authoritative for plugin scope, SDK capability gates, and native Rust implementation. Changes to the host integration contract must be based on verified AniCraft APIs, not assumptions.


The Smart Controller branch also vendors the audited AniCraft guest SDK under
`crates/anicraft-plugin-sdk`, pinned to an upstream commit. This allows the adapter
to build independently without adding the full AniCraft repository as a submodule.
