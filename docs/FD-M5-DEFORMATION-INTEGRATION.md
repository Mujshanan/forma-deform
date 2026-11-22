# FD-M5 — Deformation Integration

**Status:** single- and multi-target current-frame apply implemented through AniCraft's `plugin.run` transaction; GUI smoke test still required. Timeline-range bake remains outstanding.

## 1. Single-target current-playhead apply

Forma Deform's Smart Controller command exposes an explicit `target_action`: `preview` (default, read-only) or `apply_current_frame` (one geometry pin at the current playhead).

### Host contract verified from AniCraft source

- `warp.info {element, frame?}` returns `element`, `layer`, `warped`, and `pins: [{pin, at, rest}]`. Pin identity is `pins[].pin`, not `pins[].id`.
- `warp.pin_move {element, pin, x, y, frame?}` accepts stage-space coordinates. Omitting `frame` uses the current playhead.
- A command plug-in runs inside `plugin.run`. Nested document commands execute through the same session. AniCraft documents that one plug-in run is one undo step and any failed nested command restores the document.
- Plug-ins are allowed to call the `warp.*` command namespace.

Sources inspected in `Mujshanan/anicraft`: `crates/engine/src/cmds_warp.rs`, `crates/engine/src/cmds_plugin.rs`, `crates/engine/src/plugin_tests.rs`, and `docs/plugins.md`.

### Apply sequence

1. Evaluate the canonical controller document and build the composed output preview.
2. Require exactly one resolved output and no unresolved targets.
3. Query `warp.info` and verify the exact positive numeric element ID and `pins[].pin` ID.
4. Re-query `warp.info` immediately before mutation to reject stale target identity.
5. Validate a finite Vector2 output and build the absolute stage-space `warp.pin_move` payload.
6. Issue exactly one `warp.pin_move` call. With no `frame` argument, AniCraft uses its current playhead. The enclosing `plugin.run` is the undo boundary.

### Verification status

- [x] Source-level contract checked against the actual AniCraft command implementation.
- [x] Adapter parses the actual `pins[].pin` response shape; tests use AniCraft's exact response shape and reject the incorrect `id` alias.
- [ ] Run the plug-in in the AniCraft GUI: apply one pin, inspect the resulting keyframe, Undo, Redo, and verify the pin returns exactly.
- [ ] Verify Cancel from the native plug-in preview dialog leaves the document unchanged.

A live GUI run has not been performed from this repository session. CI and source checks are not a substitute for that smoke test.

## 2. Multi-target current-frame apply (implemented in Forma Deform)

The native `target_action` choice now includes `apply_all_current_frame`; `preview` remains the default.

### Two-phase algorithm

1. Reject unresolved outputs and require 1–512 resolved targets.
2. Require every item to be host-verified and mapped to `GeometryDeform`.
3. Reject duplicate `(element_id, pin_id)` identities.
4. **Preflight phase:** freshly query `warp.info` for every target, verify its exact `pins[].pin` identity, and build every finite Vector2 `warp.pin_move` payload. No write is issued during this phase.
5. **Commit phase:** issue the prepared `warp.pin_move` commands in deterministic preview-plan order.
6. Rely on AniCraft's enclosing `plugin.run` transaction: a nested command failure aborts the plug-in run and restores the document; a successful run is one undo step.

This is a plug-in-side batch over existing host commands, not a new `warp.pin_move_batch` host API. The dedicated host batch command remains a possible optimization and reusable native API.

### Guarantees and limits

- A preflight failure issues zero mutations.
- A commit-time failure must propagate; do not catch it and continue. AniCraft's outer transaction is responsible for rollback.
- The initial maximum is 512 target writes per invocation.
- All writes use the playhead captured by the enclosing command invocation; the adapter omits `frame` so AniCraft applies its current-frame semantics.
- GUI verification is still required to confirm multi-target apply, one-step undo, redo and rollback in a running AniCraft build.

## 2. Full M5 bake: host batch operation design

### Existing atomicity primitive

AniCraft already provides the transaction boundary for a command plug-in: all nested commands in one `plugin.run` form one undo step; any failure restores the document. Forma Deform can therefore preflight a bake and then issue multiple `warp.pin_move` calls inside one `plugin.run`.

For a first plug-in implementation, that is sufficient and avoids a plugin-side fake transaction. A host-owned batch command is still the preferred stable API for persisted bindings, native authoring, non-plugin callers, and lower host-call overhead.

### Proposed host command: `warp.pin_move_batch`

Proposed payload (all coordinates are stage-space pixels):

    {
      "frame": 12,
      "moves": [
        {"element": 41, "pin": 8, "x": 120.0, "y": 90.0},
        {"element": 41, "pin": 9, "x": 180.0, "y": 110.0},
        {"element": 52, "pin": 3, "x": 240.0, "y": 140.0}
      ]
    }

- `frame` is optional; if omitted, snapshot the current playhead once at command start. Every move uses that same zero-based frame.
- `moves` contains 1–512 entries. Each requires positive numeric `element` and `pin` IDs plus finite stage-space `x` and `y`.
- Reject duplicate `(element, pin)` pairs. Multiple different pins on one element are allowed.
- Never resolve targets by selection order or array index.

### Preflight — no document mutation

1. Resolve every element at the single snapshotted frame and locate its layer.
2. Require each target to be a live Asset Warp shape at that frame.
3. Verify every requested pin exists by stable `PinId`.
4. Validate stage coordinates against AniCraft's supported extent and convert through the element transform's inverse.
5. Reject motion-tween frames and non-invertible transforms using the same rules as `warp.pin_move`.
6. Complete all checks for the entire batch before changing any keyframe.

### Commit — one host command

1. Apply all prevalidated pin positions to their owning keyframes.
2. Preserve `warp.pin_move` semantics: editing inside a classic/shape tween keys the requested frame; motion tweens remain refused.
3. Return `{frame, moved: [{element, pin, at}], count}`.
4. Let the engine's normal command transaction create one history entry. When nested in `plugin.run`, it remains part of that plug-in's single undo step.

### Atomicity and failure

- Any preflight error rejects the whole batch before the first write.
- Any unexpected commit-time error must propagate; the enclosing engine command / `plugin.run` transaction restores the original document.
- No best-effort partial success, no per-pin undo entries, and no silent skipping.
- The batch command must not call `edit.undo` or manipulate history directly.

### Limits and deterministic behaviour

- Initial hard cap: 512 moves per command; re-evaluate against actual document size and plugin fuel/call limits before shipping.
- Sort the commit plan by `(element, pin)` for deterministic application and reports.
- Capture one frame at entry and never allow the target frame to drift during the batch.
- Reject non-finite values, out-of-range coordinates, duplicate pairs, missing pins, stale warps, and invalid IDs.

### Required AniCraft tests

1. A valid batch moves several pins across one or more elements at the specified frame.
2. Undo restores all moved pins and redo reapplies all of them as one history step.
3. A missing pin in the final item rejects the batch and leaves every earlier pin untouched.
4. An invalid coordinate, stale warp, motion-tween frame, or singular transform rejects the whole batch.
5. Duplicate `(element, pin)` pairs are rejected before mutation.
6. A batch invoked from a command plug-in still produces one outer `plugin.run` undo step.
7. The result reports the exact requested frame and stable element/pin IDs.

## 3. Scope boundary

- Single-target current-playhead apply is implemented in Forma Deform after the response-shape correction.
- Multi-target current-frame apply is implemented through `plugin.run`; timeline-range bake is not implemented.
- The existing `plugin.run` transaction is enough for multi-command plug-in writes; timeline-range bake still needs frame sampling, full-range preflight, resource limits and acceptance tests.
- `warp.pin_move_batch` is a proposed AniCraft host API, not an existing command. It is the recommended follow-up for native host-owned authoring and a single preflight/commit boundary.
- M4 durable save/load and in-AniCraft GUI verification remain prerequisites before calling full M5 complete.