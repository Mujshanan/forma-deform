# Forma Deform — Development Plan

**Status:** Proposed plan; implementation has not started.  
**Initial milestone:** Rectangle deformation with keyframe interpolation.  
**Primary language:** TypeScript.  
**Renderer:** Browser Canvas 2D.

## 1. Product definition

Forma Deform is a small 2D geometry-deformation and animation library intended to become one building block of Forma Studio. It is not intended to become a full animation editor.

The first vertical slice is intentionally narrow:

1. Draw one rectangle.
2. Select it.
3. Move the whole rectangle or one of its corners.
4. Store its geometry at a timeline frame.
5. Edit it at another frame and store a second keyframe.
6. Evaluate intermediate frames by interpolating corresponding corners.
7. Scrub and play the result in a minimal browser demo.

The library core must remain usable without the demo UI.

## 1A. Reference: AniCraft

Reference repository: [storytold/anicraft](https://github.com/storytold/anicraft).

AniCraft is a full vector-animation application written in Rust. It is useful as a reference for the *shape of the workflow* and for a few architectural principles, but Forma Deform is deliberately a much smaller library. Study its README, roadmap, and user guide when making decisions about the stage, tools, timeline, keyframes, and evaluation model.

### Concepts to borrow

| AniCraft concept | Forma Deform adaptation |
|---|---|
| Stage and drawing tools | A single HTML Canvas with Draw and Select tools |
| Rectangle tool and subselection | Draw a rectangle, then edit its four corner points |
| Frame-based timeline and keyframes | One simple timeline and one rectangle track |
| Tweening between keyframes | Linear interpolation of four corresponding corner points |
| Document as plain data | A small typed rectangle-animation document |
| Pure frame evaluation (evaluate(document, frame)) | A deterministic TypeScript evaluator with no DOM or Canvas dependency |
| Commands as explicit user actions | Keep editor operations explicit and separable; do not build a command bus until undo/redo or automation needs it |
| Milestones with a green build/test state | Every phase ends with type-check, tests, and a runnable demo |

### Concepts intentionally not copied

- AniCraft's pure-Rust/egui application stack. This project starts with TypeScript and browser Canvas 2D.
- AniCraft's complete vector drawing model, layers, symbols, scenes, masks, and file compatibility.
- Classic, motion, and shape tween systems. The first implementation only interpolates four stable corner correspondences.
- GPU/tiled rendering, background workers, CLI/MCP control, and broad import/export.
- AniCraft's document format or internal code. It is a reference, not a dependency.

**Guiding rule:** borrow the separation of document, evaluation, rendering, and interaction—not the size or complexity of the application.

## 2. Scope and constraints

### Included in the first release

- One rectangle per document / scene in the initial editor workflow.
- Four ordered corner points.
- Draw, select, whole-shape move, and corner editing.
- Frame-based timeline and explicit keyframes.
- Linear interpolation.
- Canvas 2D rendering.
- Basic playback and project serialization.
- Unit tests for geometry, timeline, and interpolation.

### Explicitly deferred

- Multiple shapes, layers, groups, and hierarchy.
- Arbitrary polygons and Bézier curves.
- Automatic point matching or topology conversion.
- Bones, rigs, skinning, weights, IK, and mesh deformation.
- Advanced easing curves and motion paths.
- SVG import/export and broad file-format support.
- Collaboration, undo history, autosave, and production asset management.
- Rust/WASM, worker threads, GPU rendering, and framework-specific UI integrations.

These features may be considered later, but must not complicate the rectangle milestone.

## 3. Architecture

Keep four responsibilities separate:

| Area | Responsibility | Must not depend on |
|---|---|---|
| Core geometry | Points, rectangle validation, geometric operations | DOM, Canvas, editor |
| Animation | Keyframes, timeline, frame evaluation, interpolation | DOM, Canvas |
| Canvas adapter | Drawing geometry and converting pointer coordinates | Animation policy |
| Demo editor | Tools, selection state, timeline controls, playback UI | Internal implementation details of geometry |

The public API should expose geometry and animation operations without exposing Canvas contexts or DOM event objects.

### Proposed source layout

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
  index.ts
demo/
tests/
docs/
  DEVELOPMENT_PLAN.md
```

Adjust the layout as implementation clarifies the boundaries. Do not create empty modules merely to match this tree.

## 4. Geometry model

### 4.1 Point

A point is a position in the document's world coordinate system:

```ts
export interface Point {
  x: number;
  y: number;
}
```

Use one consistent coordinate convention throughout the core. Canvas pixel coordinates are an adapter concern; the core should not know about device-pixel ratio or CSS pixels.

### 4.2 Rectangle

Represent the editable rectangle as four ordered corners:

```ts
export interface Rectangle {
  id: string;
  points: [Point, Point, Point, Point];
}
```

Corner order is fixed and documented:

- 0: top-left in the initial axis-aligned rectangle.
- 1: top-right.
- 2: bottom-right.
- 3: bottom-left.

After corner editing, these names describe point identity/order, not necessarily the point's current screen-space position. Do not reorder corners based on their coordinates; stable correspondence is essential for interpolation.

A rectangle created by the draw tool begins axis-aligned. Editing corners may turn it into a general quadrilateral. The data model should not claim that every edited result remains a mathematically axis-aligned rectangle.

### 4.3 Validation

The core should validate:
- Exactly four finite point coordinates are present.
- The rectangle has a valid identifier where required.
- Degenerate geometry is handled consistently.
- Interpolation endpoints have compatible point correspondence.

For the initial editor, zero-width or zero-height draw gestures should not create a usable shape. Self-intersecting or concave quadrilaterals should be treated deliberately: initially, either prevent edits that produce them or document and test the accepted behavior. Do not silently reorder points to repair them.

## 5. Animation model

### 5.1 Time representation

Use integer frame indices as the canonical timeline time. Start with a default display rate of 24 frames per second, but keep frame rate as document/timeline metadata rather than embedding it in geometry.

The first implementation may use one animated rectangle track. Avoid designing a generalized multi-track animation system until there is a concrete second use case.

### 5.2 Keyframes

A keyframe stores a snapshot of the rectangle geometry at a specific frame:

```ts
export interface RectangleKeyframe {
  frame: number;
  points: [Point, Point, Point, Point];
}
```

Keyframe geometry must be copied or otherwise immutable from the editor's perspective. Editing the current rectangle after creating a keyframe must not mutate previously stored keyframes.

Define and test behavior for:
- Inserting a keyframe at an occupied frame (replace that frame's value).
- Removing a keyframe.
- Evaluating before the first keyframe.
- Evaluating after the last keyframe.
- Evaluating exactly on a keyframe.
- Evaluating between two keyframes.

For the first version, hold the nearest endpoint outside the keyed range. Between keyframes, interpolate linearly.

### 5.3 Interpolation

For adjacent keyframes at frames (f_0) and (f_1), evaluate frame (f) using:

```text
t = (f - f0) / (f1 - f0)
P(t) = P0 + (P1 - P0) * t
```

Apply this independently to each of the four corresponding points.

Requirements:
- The operation is pure: it does not mutate either keyframe.
- Input keyframes must have matching point counts and ordering.
- Frame order must be valid and (f_1 > f_0).
- Results must be deterministic.
- Exact keyframe frames should return the stored geometry values.
- No extrapolation outside the keyed range in the initial version.

Do not add automatic point correspondence. The fixed four-corner ordering is what makes the first version straightforward.

## 6. Editor interaction

### 6.1 Canvas and coordinates

- Create a canvas with a clear document coordinate space.
- Convert pointer coordinates from CSS/client space into document space in one tested utility.
- Account for canvas scaling and device-pixel ratio in the renderer/adapter.
- Keep zoom and pan out of the first milestone unless needed to make the canvas usable.
- Render the rectangle, selection outline, and corner handles.

### 6.2 Draw tool

Initial behavior:
1. Pointer down records the starting point.
2. Pointer move updates a temporary rectangle preview.
3. Pointer up finalizes the rectangle.
4. Normalize drag direction so drawing works in all four directions.
5. Reject dimensions below a small documented threshold.
6. Assign a stable unique shape identifier.

The draw tool should create an axis-aligned rectangle with four ordered points.

### 6.3 Select and move

Initial behavior:
- Clicking inside the rectangle selects it.
- Clicking outside clears selection.
- Dragging the selected interior moves all four points by the same delta.
- Dragging a corner handle moves only that corner.
- Hit testing uses document-space coordinates and a configurable tolerance.
- Selection visuals are not stored in the geometry model.

Avoid adding a separate complex tool framework for only two tools. A small explicit tool state is sufficient.

## 7. Timeline and playback

### 7.1 Timeline UI

The demo timeline needs only:
- Current frame display.
- A horizontal frame ruler or simple frame strip.
- Visible keyframe markers.
- A playhead that can be moved by clicking or dragging.
- Add/update keyframe action.
- Remove keyframe action.
- Play and pause controls.

Do not build a sophisticated dope sheet or nonlinear editor.

### 7.2 Playback

- Use a browser animation scheduling mechanism in the demo layer.
- Derive the current frame from elapsed time and configured FPS.
- Evaluate geometry through the same pure timeline evaluator used by scrubbing.
- Stop or pause cleanly when requested.
- Keep playback scheduling out of the core geometry package.
- Ensure the displayed frame is deterministic when the user scrubs manually.

The core should accept a frame number; it should not own a browser clock. Follow AniCraft's useful principle that the displayed result is derived from document data and a requested frame, conceptually evaluate(document, frame). The evaluator must be pure and deterministic.

## 8. Project document and serialization

Introduce serialization only after geometry and timeline behavior are stable.

The first project format should be small and versioned. It should contain:
- Document format version.
- Timeline frame rate.
- Rectangle identity and initial/current geometry as required by the editor.
- Ordered keyframes and their point snapshots.

Requirements:
- JSON serialization and deserialization.
- Validate parsed data rather than trusting arbitrary JSON.
- Reject unsupported document versions with a useful error.
- Do not serialize transient UI state such as hover, selection handles, or active pointer gestures.
- Add round-trip tests.

## 9. Phased implementation plan

### Phase 0 — Repository and tooling foundation

**Work**
- Add and maintain the README and this development plan.
- Confirm package/module format and minimum supported browser/runtime.
- Set up TypeScript compilation and a minimal development server or demo workflow.
- Add formatting/linting only if they provide clear value.
- Add a test runner and a basic CI workflow.
- Keep dependencies minimal and pin versions.

**Acceptance criteria**
- A clean checkout can install dependencies and run the documented scripts.
- TypeScript type-checking and tests run in CI.
- No application framework is required.

### Phase 1 — Geometry core

**Work**
- Implement Point and Rectangle types.
- Implement rectangle creation from two drag endpoints.
- Implement translation and corner replacement operations.
- Implement geometry validation and basic hit-testing helpers.
- Export the public geometry API.

**Acceptance criteria**
- Unit tests cover all four drag directions.
- Translation preserves relative corner offsets.
- Moving one corner leaves the other three unchanged.
- Invalid/non-finite coordinates are rejected.
- Core modules run without DOM or Canvas globals.

### Phase 2 — Canvas renderer and drawing

**Work**
- Create a minimal HTML demo page.
- Implement document-to-canvas coordinate mapping.
- Render the rectangle and a temporary draw preview.
- Implement pointer capture and draw gesture lifecycle.
- Handle canvas resizing and device-pixel ratio.

**Acceptance criteria**
- User can draw a rectangle by dragging in any direction.
- Preview follows the pointer accurately.
- The finalized geometry is represented by four ordered points.
- Resizing the browser does not corrupt document geometry.

### Phase 3 — Selection and corner editing

**Work**
- Implement point/handle hit testing and rectangle interior hit testing.
- Add selected-state visuals and corner handles.
- Implement whole-shape dragging.
- Implement individual-corner dragging.
- Keep editor interaction state outside the core geometry objects.

**Acceptance criteria**
- User can select and deselect the rectangle.
- User can move the rectangle without changing its shape.
- User can move each corner independently.
- Hit testing remains correct under canvas scaling.

### Phase 4 — Keyframe and timeline core

**Work**
- Define keyframe data and timeline storage.
- Add insert, replace, remove, sort, and lookup operations.
- Snapshot geometry when a keyframe is created.
- Define endpoint-hold behavior and invalid-frame handling.

**Acceptance criteria**
- Keyframes remain unchanged when the live edited geometry changes.
- Duplicate-frame insertion follows the documented replace behavior.
- Keyframe lookup is correct at, before, between, and after keyed frames.
- Timeline operations are covered by unit tests.

### Phase 5 — Linear interpolation

**Work**
- Implement pure point interpolation.
- Implement four-corner rectangle interpolation.
- Implement timeline evaluation between adjacent keyframes.
- Handle exact keyframe and out-of-range frame evaluation.

**Acceptance criteria**
- At the midpoint, each corner is exactly the midpoint of its endpoint positions (within floating-point tolerance).
- Evaluating a keyframe returns its stored geometry.
- Inputs are not mutated.
- Incompatible or invalid keyframes fail explicitly.
- Tests cover different frame gaps and all four corners.

### Phase 6 — Timeline UI and scrubbing

**Work**
- Add frame ruler, playhead, current-frame readout, and keyframe markers.
- Add create/update and delete keyframe controls.
- On scrub, evaluate the timeline and render the resulting geometry.
- When editing at a frame, make keyframe creation/update explicit; do not silently modify another keyframe.

**Acceptance criteria**
- User can create a first keyframe, move to a later frame, edit the rectangle, and create a second keyframe.
- Scrubbing between them displays interpolated geometry.
- Scrubbing outside the keyed range holds the endpoint geometry.
- Editing does not silently rewrite an existing keyframe.

### Phase 7 — Playback

**Work**
- Add play/pause controls.
- Advance frames at the configured frame rate.
- Use the same evaluator as timeline scrubbing.
- Handle pause, resume, and playback restart predictably.

**Acceptance criteria**
- Playback visibly animates between two rectangle keyframes.
- Pause freezes the displayed frame.
- Scrubbing while paused updates immediately.
- Playback does not mutate keyframe data.

### Phase 8 — Serialization and first usable release

**Work**
- Define a versioned JSON project format.
- Implement save/export and load/import in the demo.
- Validate loaded documents.
- Add round-trip and malformed-input tests.
- Document the public API and demo workflow.

**Acceptance criteria**
- A project with two or more keyframes survives save/load without geometry changes.
- Invalid files produce useful errors and do not crash the demo.
- The README accurately documents current implemented features.
- A clean checkout can run the complete demo and test suite.

## 10. Testing strategy

### Unit tests
- Point and rectangle construction.
- Drag-direction normalization.
- Geometry translation and corner replacement.
- Geometry validation and hit testing.
- Keyframe insertion, replacement, removal, and lookup.
- Interpolation math and immutability.
- Timeline endpoint behavior.
- Serialization round trips and invalid input.

### Integration tests
- Draw rectangle → create first keyframe.
- Move to later frame → edit corners → create second keyframe.
- Scrub intermediate frames → verify evaluated corner coordinates.
- Play → pause → scrub → resume.

### Manual checks
- Pointer behavior at different browser zoom levels.
- Canvas resizing and high-DPI displays.
- Fast pointer movement and pointer release outside the canvas.
- Very small rectangles and invalid geometry edge cases.

## 11. Public API direction

Keep the initial public API small. Exact names should be settled during implementation, but the conceptual surface should resemble:

```ts
createRectangle(start, end)
translateRectangle(rectangle, delta)
setRectangleCorner(rectangle, cornerIndex, point)
interpolateRectangle(from, to, t)
createTimeline(fps)
setKeyframe(timeline, frame, rectangle)
evaluateTimeline(timeline, frame)
```

Prefer pure functions and explicit inputs/outputs. Avoid exposing internal editor state or Canvas objects through the library entry point.

## 12. Future extension path

Only after the rectangle workflow is stable should the project consider:

1. General polygon geometry with stable vertex correspondence.
2. Bézier paths and curve-point interpolation.
3. Multiple shapes and independent animation tracks.
4. Additional interpolation/easing modes.
5. Shape deformation abstractions shared by multiple geometry types.
6. Separate Forma Studio libraries for bones, rigging, and other deformation systems.
7. Rust/WASM acceleration, if profiling identifies a computational bottleneck.

Each extension should preserve the separation between the reusable core and the demo editor.

## 13. Decision log

| Decision | Initial choice | Reason |
|---|---|---|
| First geometry | Four-corner rectangle | Smallest useful editable shape |
| Geometry storage | Four ordered points | Enables corner deformation |
| Initial language | TypeScript | Fast iteration and direct browser integration |
| Rendering | Canvas 2D | Native, lightweight, sufficient for the first demo |
| UI framework | None initially | Avoid unnecessary dependencies |
| Timeline unit | Integer frames | Simple deterministic keyframe model |
| Initial frame rate | 24 FPS default | Familiar animation baseline; configurable |
| Interpolation | Linear | Easy to implement and verify |
| Acceleration | No Rust/WASM initially | Optimize only after profiling |
| Scope | One rectangle | Keep the first vertical slice achievable |

## 14. Definition of done for the first release

The first release is complete when a user can:

1. Open the demo.
2. Draw a rectangle.
3. Select it and move it.
4. Edit its four corners.
5. Create a keyframe at frame 0.
6. Move to frame 20 and create a differently shaped keyframe.
7. Scrub frames 1–19 and see smooth linear corner interpolation.
8. Play and pause the animation.
9. Save and reload the project.
10. Run the core tests independently of the browser demo.


## 15. Implementation progress

Implementation is being developed on `feat/rectangle-deformation-mvp`; this is intentionally separate from `main` until reviewed.

| Area | Current branch status |
|---|---|
| Bun + TypeScript setup | Added |
| Pure four-point geometry | Initial implementation added |
| Rectangle translation and corner editing | Added |
| Timeline keyframes and linear evaluation | Initial implementation added |
| Canvas drawing and selection demo | Added |
| Timeline scrubber and playback controls | Added |
| JSON project serialization | Added |
| Automated geometry/timeline/document tests | Added; CI verification pending |
| Browser interaction and edge-case validation | Pending |
| PR review and merge | Not done |

### First demo workflow

1. Run `bun install` and `bun run dev`.
2. Open `http://localhost:3000`.
3. Draw a rectangle on the stage. This creates the initial keyframe.
4. Move the playhead to frame 20.
5. Select and drag the shape or one of its corner handles.
6. Click **Set keyframe** to store the edited geometry at frame 20.
7. Scrub frames 1–19 or press **Play** to see linear interpolation.
8. Use **Save project** / **Open project** to round-trip the JSON document.

The current code is a first vertical slice, not yet a release. Browser interaction, pointer cancellation, geometry validity for concave/self-intersecting quads, playback timing, and CI results still need review before the milestone is considered complete.
