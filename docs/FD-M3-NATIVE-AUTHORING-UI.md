# FD-M3 — Native Authoring UI Integration Specification

**Status:** M3 started; first implementation slice uses AniCraft's existing native command-settings dialog.  
**Depends on:** FD-M1 canonical mapper and FD-M2 read-only preview.  
**Ownership:** AniCraft owns the egui shell, panels, stage, selection, commands, undo, document state, and plug-in lifecycle. Forma Deform owns controller/binding semantics, validation, mapping, and diagnostics.

## 1. Verified host UI surface

The AniCraft source audit confirms:
- AniCraft is a native Rust/egui application; there is no webview or Tauri UI.
- Command plug-ins with parameters already receive a native settings dialog. The host renders sliders for numbers, checkboxes for booleans, dropdowns for choices, and colour swatches.
- The dialog previews command runs on the stage; Cancel rolls back and OK commits one undo step.
- The WASM command ABI does not expose arbitrary panel registration, a persistent UI lifecycle, or structured custom widgets.

Therefore M3 must not invent a plug-in panel API or embed a web UI. The first M3 slice will provide a native command-settings authoring form using the host's existing dialog. A docked, continuously present inspector/tool context would require a separately reviewed AniCraft host feature and is not part of this slice.

**Important compatibility finding:** the current Smart Controller manifest declares several parameters with type string. AniCraft's actual manifest parser accepts only number, int, bool, choice, and color. The M2 CI did not validate the manifest against the host parser. Correcting the manifest and testing it against the host schema is the first M3 task; until then, the canonical preview command's installability is not established.

## 2. M3 first-slice workflow

The native settings dialog will expose a constrained, schema-driven single-binding authoring workflow:

1. Choose Legacy Slider preview or Canonical Geometry preview.
2. Choose Manual input or Bone Rotation.
3. For Bone Rotation, provide stable numeric bone ID and explicit numeric AniCraft layer ID. Never infer either from selection order.
4. Configure minimum, neutral, and maximum input values; angle wrapping and spring-swing policy are explicit.
5. Choose Linear or Smoothstep mapping.
6. Identify a GeometryDeform target by numeric element ID and pin ID.
7. Configure negative, neutral, and positive 2D output anchors.
8. Run the on-demand preview. Show calculated target values and diagnostics in the plug-in log.
9. Cancel leaves the document unchanged. No save/persist or apply/bake action is offered.

This first slice is intentionally limited to one input, one controller, and one geometry target. It does not claim a general-purpose multi-controller editor.

## 3. Host dialog constraints

The manifest parser supports at most 32 parameters and these types only:
- number: bounded slider
- int: bounded whole-number field/slider
- bool: checkbox
- choice: dropdown with 1–32 options
- color: colour swatch

There is no free-form string or multiline JSON parameter. Canonical JSON must therefore be assembled from validated typed settings inside the plugin; it must not be requested as a raw JSON text field. Enum-like settings must use choice parameters. All labels must be concise and within the host's 48-character limit.

The settings form must use meaningful defaults and bounds. Numeric host IDs must be bounded to the host's supported positive integer range. Input ranges and output anchors must reject non-finite or semantically invalid configurations through core validation; UI slider clamping is not a substitute for model validation.

## 4. Preview semantics

- Preview remains on-demand and read-only.
- No pin setter, keyframe write, timeline command, or document mutation is called.
- Repeated evaluation with identical model, frame, and input snapshot is deterministic.
- Host query failure remains unavailable; never substitute zero.
- Geometry target identity is verified with read-only warp.info.
- Output is a calculated preview plan, not a rendered stage overlay.
- Cancel must leave the document unchanged; this must be checked in AniCraft.

## 5. State and diagnostics

The dialog's settings are ephemeral draft state. They are not durable bindings and do not survive document close/reopen. Diagnostics must report:
- invalid input range or mapping,
- missing/invalid bone ID or layer mapping,
- failed bone.info response,
- missing element/pin,
- unsupported input/target capability,
- unresolved controller output.

The last preview is stale whenever any setting changes. The plug-in should log the effective input, normalized mapping result, target identity, preview value, and stable diagnostic codes where available.

## 6. Follow-on native inspector

A persistent docked inspector/tool context is a later host-side enhancement, only if the AniCraft owner approves a versioned extension API. It should reuse the existing Properties panel and selection model, not add a second stage, timeline, scene tree, renderer, or undo system. It would need:
- native tool/panel registration,
- stable selection and document-revision context,
- typed command request/response rather than log scraping,
- ephemeral draft lifecycle,
- capability discovery,
- accessible keyboard/focus behavior.

No Forma Deform code may call guessed host APIs. Any new extension point belongs in the AniCraft repository and needs its own reviewed PR.

## 7. Acceptance criteria

M3 first slice is accepted when:
- [ ] The Smart Controller manifest parses with AniCraft's real manifest parser.
- [ ] All declared parameter types are supported and there are at most 32 parameters.
- [ ] The settings dialog presents the intended native form in AniCraft.
- [ ] Legacy Slider behavior remains available.
- [ ] Canonical Geometry preview builds a valid canonical document from typed settings.
- [ ] Manual and Bone Rotation input paths work with explicit stable identities.
- [ ] Geometry target identity is verified through warp.info.
- [ ] Invalid settings fail closed with actionable diagnostics.
- [ ] Preview is deterministic and read-only; Cancel leaves the document unchanged.
- [ ] Keyboard, focus, and accessibility behavior follows AniCraft's existing dialog conventions.

The persistent docked inspector is explicitly not an M3 acceptance requirement until AniCraft exposes a supported extension contract.

## 8. Guardrails

- No React/webview UI in the WASM plug-in.
- No arbitrary string/JSON parameters; use typed settings and build canonical JSON internally.
- No invented SDK functions, panel APIs, document metadata fields, setters, or callbacks.
- No durable persistence (M4), apply/bake (M5), or live evaluator (M6).
- No duplicate timeline, renderer, document, or undo system.
