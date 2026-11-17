# Forma Deform

**Forma Deform** is a small, reusable 2D shape-deformation and keyframe-interpolation library. It is being developed as a foundational component of the wider Forma Studio ecosystem.

The project starts deliberately small: draw a rectangle, edit its corners at different points on a timeline, and interpolate the corner positions to produce an animation. The geometry and animation core should remain independent of the demo editor so it can later be embedded in Forma Studio or reused by other applications.

> **Current status:** Planning / initial implementation not yet started.

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
- **Build and tests:** Keep the toolchain minimal; choose and pin the concrete tooling during the setup phase.
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

## Development plan

See [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md) for the phased implementation plan, acceptance criteria, testing strategy, and deferred features.

## Design principles

- **Core first:** geometry and interpolation must not depend on Canvas or editor UI.
- **Small steps:** each phase should produce a runnable, testable result.
- **Explicit geometry:** preserve point ordering and validate geometry before interpolation.
- **Deterministic evaluation:** the same project state and frame must produce the same geometry.
- **No silent topology changes:** incompatible keyframes must be rejected or handled by an explicit future conversion operation.
- **Measure before optimizing:** do not add Rust/WASM or complex data structures without evidence.
- **Reusable boundaries:** Forma Studio should be able to consume the core without depending on the demo editor.

## Status and contribution

The project is at its planning stage. The development plan is the source of truth for the intended sequence and acceptance criteria. Implementation decisions that change the scope or data model should be documented before they are introduced.
