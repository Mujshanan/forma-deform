# Forma Deform — UI/UX Design Specification

**Product:** Forma Deform for AniCraft  
**Design status:** Proposed native-tool UX specification  
**Scope:** M3 native authoring-tool design; respects current prototype and host capability limits.

## 1. Product experience

Forma Deform is a focused deformation and rig-control workspace inside AniCraft. It answers: **how should selected artwork respond to a rig input or authored controller?**

It is not a standalone animation editor. It must use AniCraft's stage, timeline, selection, document, command, dockable-panel, and undo/redo infrastructure. Do not introduce a second scene tree, timeline, renderer, document model, or playback runtime.

### UX principles

1. **Stage-first:** the artwork and its deformation affordances remain the visual focus.
2. **Progressive disclosure:** show controls relevant to the selected target and active tool only.
3. **Explicit intent:** binding, capture-neutral, apply, and bake are deliberate actions with clear consequences.
4. **Preview is safe:** preview is deterministic, on demand, and non-mutating.
5. **Inspectable cause and effect:** show input → normalization → mapping → output → target displacement.
6. **Host-native behavior:** selection, shortcuts, panels, timeline scrubbing, and undo follow AniCraft conventions.
7. **Honest capability states:** never imply persistence or live playback evaluation until the host supports and verifies them.

## 2. Information architecture

The tool has four user-facing areas:

- **Stage tools:** target selection, pin/handle editing where supported, soft selection, binding, and preview.
- **Target inspector:** selected target identity, target type, neutral/rest state, and target-specific settings.
- **Controller editor:** source input, neutral/range, normalization, mapping, easing, and output bindings.
- **Diagnostics and preview:** validation, source values, normalized values, mapping result, and displacement.

The existing AniCraft timeline remains below the stage and is the only timeline.

## 3. Desktop workspace layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ AniCraft chrome · document · undo/redo · contextual tool options            │
├─────────────┬───────────────────────────────────────────┬───────────────────┤
│ TOOL RAIL   │ STAGE                                     │ INSPECTOR         │
│             │                                           │                   │
│ Select      │ Artwork / selected target                 │ Target            │
│ Pin*        │                                           │ Controller        │
│ Handle*     │ Mesh / pins / handles / rest markers      │ Mapping           │
│ Soft select*│                                           │ Outputs           │
│ Bind        │ Preview displacement overlay              │ Diagnostics       │
│ Preview     │                                           │ Preview values    │
├─────────────┴───────────────────────────────────────────┴───────────────────┤
│ AniCraft timeline · layers · playhead · frame controls                       │
└─────────────────────────────────────────────────────────────────────────────┘
*Only show tools supported by the active target type.
```

### Layout behavior

- Tool rail: narrow, icon-led, tooltips and accessible labels; active tool has a persistent selected state.
- Stage: takes the largest share of available width. Keep artwork centered and overlays visually distinct.
- Inspector: dockable/resizable using AniCraft conventions. Tabs or collapsible sections prevent excessive scrolling.
- Timeline: host-owned and unchanged. Forma Deform may read the current playhead/frame but must not create a second timeline.
- Narrow layouts: allow inspector collapse into a host panel/tab; never cover the stage with a modal for routine edits.

## 4. Stage visual language

| Visual | Treatment | Meaning |
|---|---|---|
| Selected target outline | Cyan, 1.5–2 px | Current target selection |
| Deformation mesh | Low-contrast neutral lines | Editable topology / warp structure |
| Selected pin or handle | Solid cyan marker with clear hit area | Active point |
| Unselected pin | Small neutral marker | Available point |
| Neutral/rest position | Dashed violet marker or ghost | Authored baseline |
| Preview displacement | Violet ghost / directional vector | Evaluated result, not authored geometry |
| Bound target | Small link glyph / subtle badge | Has a controller binding |
| Invalid target | Amber outline and diagnostic marker | Binding or target requires attention |
| Unsupported capability | Disabled control with explanation | Host or target does not expose the operation |

Avoid showing every overlay at once. Provide stage overlay toggles for Mesh, Pins, Handles, Rest Positions, Bindings, and Preview. Default visibility should depend on the active tool.

## 5. Tool definitions

### Select Target
- Click artwork or a supported target to select it.
- Selection populates the Target inspector.
- Show target type and stable host identity in a compact, copyable details row.
- Do not create a competing layer or object tree.

### Pin
- Available only when the selected target exposes compatible warp pins.
- Select and move pins using AniCraft's supported command/preview transaction.
- Display pin ID and stage-space position in the inspector when selected.
- Distinguish rest position from current evaluated position.
- Do not imply arbitrary mesh editing if only pin movement is supported.

### Handle
- Available only for target types with a verified handle model and host operation.
- If unsupported, omit it rather than presenting a dead tool.

### Soft Selection
- Future/advanced capability. Show only when the corresponding implementation exists.
- When available, provide radius, falloff, and weight visualization; preview weights before committing.

### Bind Controller
- Opens the guided binding flow (section 7).
- Requires a compatible source and target.
- Never silently creates a binding from selection alone.

### Preview
- Runs a single explicit evaluation using a manual value or an on-demand host query.
- Shows a read-only overlay and structured evaluation breakdown.
- Does not change authored geometry, keyframes, document data, or timeline state.

## 6. Inspector structure

Use contextual sections with clear labels and concise helper text.

### Target
- Target name / host identity
- Target type (pin, geometry, vector, property, symbol frame, etc. only where supported)
- Source layer / containing element when returned by the host
- Binding status: Unbound, Valid, Needs attention, or Unsupported
- Rest/neutral reference

### Source input
- Input source: Bone / Manual value
- Bone selector populated from verified AniCraft bone query results
- Bone identity (stable ID) and display name
- Input channel and units (e.g. joint angle in degrees or radians, explicitly labeled)
- Frame source: current AniCraft playhead
- Manual input field for reproducible preview

For bone angle, clearly label whether the value is the constrained keyed angle or includes spring swing. Do not silently combine these values.

### Range and normalization
- Minimum
- Neutral
- Maximum
- Current input
- Normalized output
- Range direction / reversed-range indication
- Validation for non-finite values and zero-length ranges

### Mapping
- Mapping type: Range 1D, Three Anchor, Directed Range, Discrete, or Pose Set 1D, subject to actual core support
- Easing: Linear / Smoothstep where supported
- Endpoint policy for frame ranges: Exclusive / Inclusive where relevant
- Mapping preview graph with labeled anchors and current-input cursor
- Unsupported modes (including joystick/two-dimensional mapping in the one-dimensional mapper) must not appear as selectable options

### Outputs and target gains
- Output binding list with target identity
- Output type and units
- Per-target gain / axis constraint only when implemented for that target
- Clear indication that mapper outputs are contributions and are not implicitly composed with Form Deform pin evaluation

### Diagnostics
- Severity, concise issue title, and actionable explanation
- Examples: missing source, stale target, unsupported target type, invalid range, missing pose, incompatible output type, dependency cycle, unsupported multi-input controller, or host capability unavailable
- Each diagnostic links to the relevant inspector field or target where possible

### Preview values
Show the evaluation chain as a compact read-only stack:

1. Raw input (with source and units)
2. Normalized input
3. Mapping segment / selected keyform
4. Mapped output (with type and units)
5. Target displacement / resulting value

Display “Not evaluated” until the user runs Preview. Show timestamp/frame context only if returned reliably by the host.

## 7. Binding workflow

Use a guided side panel or compact stepper, not a large blocking wizard.

**Step 1 — Choose source**
- Select a verified bone input or enter a manual value for testing.
- Show stable bone ID, name, and input channel.
- Confirm angle semantics and units.

**Step 2 — Choose target**
- Select an eligible deformation target from the stage or a host-provided target list.
- Show target type and stable target identity.
- Reject unsupported or stale targets with a specific reason.

**Step 3 — Set neutral**
- Read the current source value or enter a neutral value.
- “Capture neutral” is an explicit action.
- Show the captured value before confirmation; do not modify authored artwork.

**Step 4 — Configure mapping**
- Set minimum / neutral / maximum.
- Choose a supported mapping and easing.
- Explain reversed ranges and endpoint behavior inline.
- For Pose Set 1D, verify that referenced poses and binding identities are compatible.

**Step 5 — Validate and preview**
- Run validation.
- Show errors before enabling the final action.
- Preview is non-mutating and can be repeated with manual input.

**Step 6 — Confirm**
- Use an explicit Save/Apply action only when the host-backed persistence or command path exists.
- State whether the action creates a persistent binding, performs a one-shot command, or is preview-only.
- Persistent changes must be one coherent AniCraft undo operation.

### Binding states

- Draft: configured locally in the current editing flow, not persisted.
- Previewed: deterministic preview completed; no document mutation.
- Valid: binding is accepted by the available model/host contract.
- Needs attention: missing source, target, or invalid settings.
- Persisted: use only after verified host document persistence exists.
- Unsupported: operation cannot be completed by the current host integration.

Do not label a draft as saved.

## 8. Preview and apply behavior

### Preview
- Explicit user-triggered action.
- Inputs are a stable snapshot for that invocation.
- Same model + same explicit input must produce the same result.
- Show progress only if evaluation is not immediate.
- Preview overlay is visually distinct from authored geometry.
- Clear preview when the target, source, mapping, or frame context changes; alternatively mark it stale until refreshed.
- Never write keyframes, authored pin positions, or document metadata.

### Apply / bake
- Separate from Preview and visually more deliberate.
- Explain exactly what will change and at which target/frame.
- Require a valid target and successful validation.
- Use one host command/transaction and one undo step.
- Confirm success only after the host reports success.
- If apply/bake is not implemented for a target type, do not show the action.

## 9. Diagnostics, empty, loading, and error states

| State | UI response |
|---|---|
| No target selected | Stage guidance: “Select a supported deformation target to inspect it.” |
| Unsupported target | Explain which capability is unavailable; keep unrelated tools usable |
| No controller/source | Show a clear “Choose source” action |
| Host query loading | Inline loading indicator in the relevant field; keep stage interactive |
| Host query failed | Preserve last authored settings; show retry and error details |
| Invalid range | Inline field errors and a blocking validation diagnostic |
| Stale target/source ID | Mark binding Needs attention; never silently rebind by array index |
| Preview unavailable | Explain the missing host capability or invalid model |
| Preview success | Show overlay and evaluation chain |
| Preview stale | Keep result visibly marked stale; offer rerun |
| Apply failed | Report host error; retain authored settings and avoid claiming a commit |
| No persisted-binding API | Label settings as session/draft state; explain that durable storage is not yet available |

## 10. Interaction and accessibility

- Follow AniCraft keyboard conventions; avoid inventing conflicting global shortcuts.
- Every icon-only control has an accessible name and tooltip.
- Use visible focus indicators and keyboard-operable controls.
- Do not rely on color alone: combine color with shape, labels, and iconography.
- Numeric fields support direct entry, keyboard stepping, and clear units.
- Stage hit targets should be generous enough for precision work; allow zoom-aware interaction.
- Provide undo/redo for every persistent edit through AniCraft, not a plugin-local undo stack.
- Destructive actions require clear labeling and, where appropriate, confirmation.
- Respect reduced-motion preferences; overlays should not pulse or animate unnecessarily.

## 11. Suggested visual tokens

These are starting points, not requirements to override AniCraft's theme system.

| Token | Suggested value | Usage |
|---|---|---|
| Workspace background | #17191F | Main dark surface |
| Panel surface | #20232B | Inspector and tool surfaces |
| Raised surface | #292D37 | Menus and nested sections |
| Primary text | #F1F3F7 | Main labels |
| Secondary text | #A3AAB8 | Helper text |
| Accent | #8B7CFF | Primary action / controller identity |
| Selection | #55D6E8 | Selected target and active pin |
| Warning | #E7B45A | Invalid or stale state |
| Error | #EF727B | Blocking failure |
| Rest overlay | #A88CFF | Neutral/rest ghost |
| Preview overlay | #9B7CFF | Evaluated displacement |

Use AniCraft theme tokens if available; do not hard-code a separate theme implementation into the plugin.

## 12. MVP screen inventory

1. **Workspace / no selection** — tool rail, stage guidance, inspector empty state.
2. **Target selected** — target identity, supported tools, target inspector.
3. **Pin editing** — mesh/pin overlays and selected-pin properties.
4. **Controller binding draft** — source, target, neutral, range, mapping, validation.
5. **Preview result** — raw-to-output breakdown and stage displacement overlay.
6. **Diagnostics** — grouped actionable issues with links to relevant fields.
7. **Apply confirmation/result** — only for verified supported commands.

## 13. Implementation sequence

### M3 — Native authoring UI design and implementation
- Build the native tool context using AniCraft's existing panel and command infrastructure.
- Implement selection-aware inspector and supported tool visibility.
- Add the binding editor with source/target identity and validation.
- Add on-demand, non-mutating preview display.
- Add diagnostic states and accessible keyboard/focus behavior.
- Verify host selection, panel docking, command dialog, and undo behavior in AniCraft.

### Follow-on work
- M4: versioned binding persistence through a host-owned document extension. Until then, do not present drafts as durable.
- M5: target adapters and explicit apply/bake commands with stable identity and atomic undo.
- M6: live overlay/playback only after a supported host evaluator is implemented and verified.
- Advanced tools such as lattice, soft selection, and multi-axis joystick mapping should appear only when their models and algorithms are implemented.

## 14. Acceptance criteria

- The workspace is visibly part of AniCraft and reuses its stage and timeline.
- No duplicate document, layer tree, timeline, renderer, undo stack, or playback runtime is introduced.
- Tool availability follows the selected target's verified capabilities.
- A user can inspect a target and understand its stable host identity.
- A user can configure a supported one-dimensional controller mapping with explicit source, neutral, range, and easing.
- Preview clearly exposes raw input, normalized input, mapping result, and output.
- Preview does not mutate authored geometry, keyframes, or document data.
- Invalid and unsupported states are actionable and never silently coerced.
- Persistent edits, when host persistence exists, participate in AniCraft undo/redo.
- The UI does not claim live playback or durable binding storage before host support is verified.


---

## 2026-11-23 implementation status — native M1 foundation

The first native Smart Controller UI foundation has been merged into the AniCraft host repository in [AniCraft PR #2](https://github.com/Mujshanan/anicraft/pull/2) (merge commit `8a437b81f0f59618a16a87b669cfffd4c9e38c4b`). This is an **M1 UI foundation**, not completion of the full authoring workflow.

Implemented in AniCraft:
- A native Smart Controller entry in the existing tool strip.
- Transient Stage handles with nearest-handle selection and a foreground overlay.
- A Properties inspector showing selected handle coordinates and allowing draft-handle removal/clear.
- Explicit draft-only labeling; no unsupported Save, Apply, Bake, or Preview controls.

Still outstanding: drag editing, stable source/target identity, controller binding and mapping UI, validation graph, deterministic non-mutating preview, document persistence/undo integration, target application, live playback, and the manual Windows host smoke test. The merged PR had no reported workflow runs or commit status checks, so this merge is not evidence of a passing native build or CI run.

The Smart Controller remains independent of Form Deform pin evaluation under CCC-01.5. AniCraft continues to own the native UI lifecycle and host document/evaluation boundaries; Forma Deform owns reusable domain logic.
