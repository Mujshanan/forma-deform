# FD-M3 — Native Authoring UI Integration Specification

**Status:** M3 design and host-integration contract started; native UI implementation is gated on verified AniCraft extension points.  
**Depends on:** FD-M1 canonical mapper and FD-M2 read-only command preview.  
**Ownership:** AniCraft owns panels, stage, selection, commands, undo, document state, and plugin lifecycle. Forma Deform owns controller/binding semantics, validation, mapping configuration, and preview diagnostics.

## 1. Goal

Deliver a native AniCraft authoring workflow for Smart Controllers and deformation bindings. The UI must feel like an AniCraft tool context, reuse existing host panels and stage, and never create a second scene tree, timeline, renderer, document, or undo system.

M3 is not a standalone web UI and does not add a custom panel framework to the command-plugin WASM ABI. Until AniCraft exposes a supported UI extension contract, this specification is the implementation contract for the host-side work.

## 2. M3 scope

### Included
- Tool-context entry point and contextual tool options.
- Target inspector for the currently selected compatible target.
- Controller/binding editor for one supported input and explicitly configured target bindings.
- Source selection and neutral/range/mapping/easing controls.
- Validation and diagnostics with actionable messages.
- Explicit on-demand Preview action using the M2 read-only evaluation path.
- Clear distinction between preview output and committed authoring changes.
- Keyboard focus, accessibility labels, selection changes, and cancel behavior consistent with AniCraft conventions.

### Excluded
- Durable binding persistence (M4).
- Apply/bake or target mutation (M5).
- Live playback evaluator or stage overlay (M6).
- Multi-axis/joystick authoring, multi-source composition, and pose-library persistence.
- A duplicate timeline, property system, document model, or undo stack.

## 3. Proposed native layout

Reuse AniCraft's existing dockable-panel and stage infrastructure.

- **Tool context / left tool strip:** Select Target, Bind Controller, Preview. Pin/Handle tools appear only when the selected target supports them; do not expose nonfunctional tools.
- **Stage:** AniCraft remains the sole renderer and selection authority. M3 does not draw a parallel stage or claim a live deformation overlay.
- **Properties / right inspector:**
  1. Target identity and capability summary.
  2. Input source (Manual or supported Bone Rotation).
  3. Neutral, minimum, maximum, and angle semantics.
  4. Mapping type and easing.
  5. Output target and typed anchors/gain.
  6. Validation diagnostics and last preview values.
- **Bottom timeline:** use AniCraft's existing timeline and playhead without adding a second timeline.

The inspector should remain compact and progressive: show only fields relevant to the selected input, mapper, and target type. Unsupported target types must show why they are unavailable rather than offering nonfunctional controls.

## 4. Interaction flows

### 4.1 Create a controller binding
1. User invokes Bind Controller from the AniCraft tool context or selected target context menu.
2. User chooses a source: Manual input or a supported canonical bone rotation.
3. For a bone source, choose the bone by stable numeric host ID and display its name as a label only. Never use list position as identity.
4. Capture/confirm neutral and configure the input range, wrapped/unwrapped angle semantics, and spring-swing policy.
5. Choose a supported target and configure mapper-specific output values.
6. Run validation. Block preview for missing IDs, malformed ranges, incompatible typed values, unsupported targets, or unresolved dependencies.
7. Run Preview explicitly. Show input sample, normalized domain, selected mapping segment/anchor, typed output, target identity, and diagnostics.
8. Save/commit is intentionally not available in M3 until the M4 host-owned persistence contract exists.

### 4.2 Edit an existing binding
M3 may edit an in-memory draft only. It must clearly label the draft as unsaved and must not imply that it survives document close/reopen. Cancel discards the draft. M4 will define durable save/load and migration.

### 4.3 Preview
- Preview is an explicit command, not a continuously running evaluator.
- Preview is read-only and must not move pins, write keyframes, alter selected elements, or mutate document data.
- Repeated evaluation with the same document revision, frame, controller definition, and host input snapshot must return identical output.
- Display stale-preview status when the selection, frame, document revision, or draft changes after the last preview.
- Report host query failures as unavailable input; never substitute zero.

## 5. UI state model

Keep ephemeral UI state separate from authored model data:

- Closed: no Forma Deform inspector context.
- SelectingSource: awaiting a valid source identity.
- EditingDraft: draft controller and binding configuration in memory.
- InvalidDraft: validation errors block preview.
- PreviewReady: validated draft, no preview for current revision.
- Previewed: output and diagnostics correspond to the current draft/revision/frame/input snapshot.
- HostUnavailable: required host UI/command capability is not exposed or a query failed.

State transitions must be explicit. Selection/frame/document changes invalidate preview state but must not silently rewrite the draft. Closing the context prompts only if the host has an unsaved-draft convention; otherwise discard with a clear notice.

## 6. Host integration contract required

The current audited WASM command-plugin ABI exposes command invocation, compact document context, selected-element queries, and logging. It does not expose native panel registration or a persistent UI lifecycle. Therefore the M3 host implementation requires a reviewed AniCraft-side API, not speculative calls from the plugin.

Minimum host-side capabilities to verify or implement:

1. **Tool/panel registration:** register a Forma Deform tool context and inspector using existing dockable panel infrastructure.
2. **Selection context:** read stable selected element, layer, frame, and document revision identifiers.
3. **Command bridge:** invoke the existing M2 preview command with validated parameters and receive structured output, not only unstructured log text.
4. **Draft lifecycle:** hold ephemeral UI draft state in the host UI process; do not serialize it into undocumented document fields.
5. **Capability discovery:** expose target/input capability information so unsupported controls are not shown.
6. **Host validation:** confirm stable IDs and active document/frame at preview invocation to avoid stale-selection races.
7. **Undo boundary:** no undo entry for preview or draft editing. Durable save/apply operations remain future milestones and must use host commands.

Any new AniCraft API must be implemented and reviewed in the AniCraft repository by its owner. This repository must consume only a documented, versioned host contract.

## 7. M2 preview bridge contract

The UI should call a typed host facade rather than assemble arbitrary command JSON in view components. Proposed request fields:

- document revision
- frame
- controller document JSON
- optional manual values JSON
- explicit stable bone ID to layer ID map
- bone angle policy
- angle semantics
- target identity

Proposed response fields:

- status: success or unavailable
- input samples: source ID, typed value, availability, and source diagnostics
- normalized inputs
- mapping results: mapper ID and typed binding outputs
- target previews: stable target identity and computed values
- diagnostics: stable code, severity, message, and optional source/binding/target ID

This is a host facade contract proposal, not a claim that the current command ABI already returns this structure. Before implementation, align field names and serialization with the actual M2 core types and AniCraft command response conventions.

## 8. Accessibility and interaction requirements

- All controls have accessible names and keyboard focus order.
- Source and target selectors expose names plus stable IDs in detail/diagnostic views.
- Validation errors are associated with the field that caused them.
- Keyboard shortcuts must not shadow AniCraft global shortcuts.
- Escape cancels the current picker or draft edit according to host conventions.
- Preview status is conveyed in text, not color alone.
- No modal dialog for routine property editing; use existing inspector patterns.

## 9. Acceptance criteria

M3 is accepted only when all of the following are true:

- [ ] AniCraft provides a documented, versioned native tool/panel extension point.
- [ ] Forma Deform registers inside the existing AniCraft workspace; no separate app/window or duplicate stage/timeline is introduced.
- [ ] Inspector reflects current selection and invalidates stale preview on frame/document/selection changes.
- [ ] Manual and supported Bone Rotation sources can be selected by stable identity.
- [ ] Mapper/target-specific controls are schema-driven and hide unsupported capabilities.
- [ ] Invalid drafts cannot preview and show actionable diagnostics.
- [ ] Preview calls the M2 canonical evaluator and is demonstrably read-only.
- [ ] Repeated preview is deterministic for identical inputs.
- [ ] Draft edits and preview create no document mutation or undo entry.
- [ ] Keyboard, focus, and accessibility checks pass.
- [ ] Native UI integration tests run in AniCraft; Rust/WASM tests alone are insufficient.

## 10. Immediate implementation sequence

1. Review the current AniCraft source for existing native tool/panel extension points and identify the smallest supported integration surface.
2. If a suitable extension point exists, document its exact API/version and add a thin Forma Deform host adapter.
3. If none exists, stop plugin-side UI work and prepare a narrowly scoped AniCraft host proposal for panel registration and structured command responses.
4. Implement the UI shell only against that verified host contract.
5. Connect the inspector to M2 preview after the structured request/response path is tested.
6. Defer durable save/load to M4.

## 11. Non-goals and guardrails

- Do not add React/webview UI inside the WASM command plugin unless AniCraft explicitly supports and documents that architecture.
- Do not invent SDK functions, panel APIs, document metadata fields, setters, or host callbacks.
- Do not present a mockup as a working native integration.
- Do not add a second timeline, renderer, undo stack, or persistence mechanism.
- Do not merge or change AniCraft itself from this repository without a separately reviewed host-side change.
