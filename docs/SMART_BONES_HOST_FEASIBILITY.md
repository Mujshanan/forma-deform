# AniCraft Smart Bones Host Feasibility

- **Status:** Draft technical feasibility assessment
- **Assessment date:** 2026-11-21
- **Target:** Moho-style Smart Bones in AniCraft
- **Primary codebase:** Mujshanan/AniCraft
- **Reusable engine:** Mujshanan/forma-deform
- **Scope:** Host capability and integration feasibility; no AniCraft implementation is authorized by this document

## 1. Executive conclusion

**A complete Smart Bones feature is not feasible as a plug-in-only implementation under the currently documented AniCraft plug-in contract. It appears feasible as a host-native AniCraft feature backed by a host-independent deformation/action engine.**

The current plug-in API can query bone state on demand, inspect warp pins, and issue undoable document commands. It does not provide a persistent per-frame evaluator, arbitrary document-owned plug-in metadata, or plug-in UI panels. Frame scripts are keyframe-entry scripts and cannot call the bone/warp command APIs.

An MCP/control bridge can support external interactive commands and may be useful for a limited posing prototype. It is not a frame-synchronized playback evaluator and must not be presented as one.

Recommended architecture:
- AniCraft owns the document model, authoring UI, animation action tracks, pose evaluation hook, persistence, undo/redo, and playback/export integration.
- forma-deform owns host-independent action/deformation models, validation, angle-to-progress mapping, deterministic evaluation, and geometry correction calculations.
- AniCraft adapters translate host identities, coordinates, and evaluated pose inputs to/from the reusable engine.

## 2. Sources and evidence baseline

### AniCraft source inspected

- Plug-in guide: https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md
- Generated command reference: https://github.com/Mujshanan/anicraft/blob/main/docs/guide/reference/commands.md
- Guest SDK source pinned by the Forma Deform audit: https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/examples/plugins/sdk/src/lib.rs
- Existing Forma Deform audit: docs/ANICRAFT_SDK_AUDIT.md
- Existing Smart Controller plan: docs/SMART_CONTROLLER_PLAN.md
- Existing host smoke-test guide: plugins/INSTALL.md

The generated command reference on AniCraft main describes the command registry for the version it accompanies. The SDK source link is pinned to the audited upstream commit; do not assume the pinned guest SDK and current host command registry are identical revisions without checking their compatibility.

### Existing Forma Deform decisions retained

- Smart Controller V1 is preview-only.
- Form Deform's explicit pin apply path is a document edit subject to host undo semantics.
- bone.info is a read-only on-demand query; its keyed constrained angle and spring swing are separate values.
- warp.info is a read-only query; warp.pin_move is a document mutation that keys a pin at a frame.
- No continuous Smart Controller playback evaluation or durable plug-in-owned binding storage is established.
- The existing pin formula and controller binding contracts remain compatibility baselines; this assessment does not change them.

## 3. Capability matrix

| Capability | Current documented evidence | Feasibility for Smart Bones |
|---|---|---|
| Query evaluated bone pose | bone.info {frame?, layer?, points?} returns bone angles, limits, spring swing and armature details | Available on demand to command plug-ins |
| Read angle at arbitrary frame | bone.info accepts an explicit frame | Available as a query; no subscription/callback |
| Distinguish keyed angle and spring swing | Command result exposes constrained keyed angle and spring swing separately | Input policy must be decided; cannot silently assume one |
| Read warp geometry | warp.info {element?, frame?} returns mesh topology and pin identities/positions | Available on demand |
| Edit warp pins | warp.pin_move moves/keys a pin at a frame | Available as an explicit document mutation with undo; not a live evaluator |
| Read document context | SDK document() and selected_elements() | Limited context; selected/visible active-layer semantics apply |
| Author ordinary property keys | Command registry includes motion.set_key | Explicit key-writing command, not an action-driven runtime |
| Move global playhead | timeline.goto {frame} | Available as a command; not a nested symbol frame setter |
| Store Smart Bone metadata | Plug-in guide says plug-ins belong to AniCraft, not documents; no arbitrary plug-in metadata API is documented | Not available to plug-ins under the documented contract |
| Persist plug-in-owned files | Sandbox forbids file access | Not available to plug-ins |
| Add native authoring panels | Plug-in guide lists panels/other UI parts as not yet supported | Not available to plug-ins |
| Continuous evaluation during playback/scrubbing | No custom per-frame evaluator hook in the plug-in contract | Not available to command plug-ins |
| Frame-script bone query | Frame scripts cannot call command, document, or elements | Cannot query bone.info from a frame script |
| Frame-script execution model | Runs when playback enters a keyframe that names it; each invocation starts a fresh WASM instance | Not a continuous Smart Bone evaluator |
| Undo/redo | A command plug-in run is one undoable transaction; failed runs roll back | Suitable for explicit authoring commands, not repeated per-frame mutation |
| Export evaluated frames | Export plug-ins receive rendered frame data, not the editable document | Useful for output processing; not a substitute for authoring/runtime integration |

## 4. Important semantic distinctions

### 4.1 AniCraft "actions" are not Moho Smart Bone Actions

The current command reference's actions.set / actions.list concerns frame scripts and instance event handlers (play, stop, goto, set_property, and related actions). It does not establish an animation action timeline that stores arbitrary keyed corrective changes and is driven continuously by a bone angle.

The Smart Bones product therefore needs a distinct host-owned animation-action concept or an explicitly designed extension to AniCraft's existing animation model. Reusing the word "action" must not cause these different concepts to be conflated.

### 4.2 Bone angle input is not yet fully specified

bone.info exposes:
- constrained keyed joint angle;
- spring swing as a separate contribution;
- keyed head/tail coordinates.

If Smart Bones should respond to the visually evaluated spring-adjusted angle, the control input may need to include the swing contribution. If they should respond only to keyed pose, it must not. This is a product/architecture decision and needs tests.

### 4.3 Warp pin mutation is not runtime deformation

warp.pin_move writes a pin key at a requested frame. It is useful for authoring and explicit edits, but repeatedly calling it while posing or playing would mutate the document and create timeline data. That is not equivalent to evaluating an authored correction from immutable document state.

### 4.4 MCP bridge is an interactive control path, not playback integration

The existing AniCraft fork has an authenticated MCP/control bridge for document inspection and command execution. A controller can potentially query a bone and issue a corresponding command in response to external interaction. This may support a manually driven interactive experiment, subject to live validation and latency measurement.

It does not provide a callback synchronized to every animation frame, nor does it prove that Smart Bones work during normal timeline scrubbing, Test Movie, or export.

The prior V4 feasibility scope was limited to MCP-driven interactive slider updates (not continuous/frame-synchronized playback), with a proposed test of at least 10 consecutive updates and median latency below 500 ms, plus a 20+ cycle log. The available project record does not establish that those live measurements were completed. Treat V4 as unverified until its actual test artifacts/results are inspected.

## 5. What can be built without changing AniCraft

A command plug-in can reasonably support:
- explicit Smart Bone configuration/diagnostic commands;
- on-demand bone-angle inspection;
- deterministic angle-to-action-progress calculations;
- preview calculations from explicit user-supplied input;
- explicit undoable pin edits for controlled authoring experiments;
- logging and validation.

This can prove parts of the model and adapter boundary. It cannot deliver the full Smart Bones user experience because it lacks durable document-owned definitions, an action editor, and continuous evaluation.

An MCP-driven prototype can add an external interactive control experiment, but it must remain explicitly separate from native playback.

## 6. Required AniCraft host capabilities

A production Smart Bones feature needs host support for:

1. **Document model and persistence**
   - Versioned Smart Bone definitions.
   - Stable references to control bones and affected targets.
   - Authored action tracks and keys.
   - Save/reopen, migration, validation, copy/duplicate, rename and delete semantics.

2. **Authoring UI**
   - Create/manage Smart Bone controls.
   - Configure angular ranges and action direction.
   - Enter and exit Smart Bone Action editing mode.
   - Edit keys for supported target channels.
   - Preview and diagnose missing references.

3. **Pose/evaluation integration**
   - Evaluate bone control inputs at the current scene/frame.
   - Map inputs to action progress.
   - Evaluate action tracks and compose corrections with the ordinary pose.
   - Run deterministically for interactive posing, scrubbing, playback, and export.
   - Avoid destructive writes to authored keyframes during evaluation.

4. **Undo/redo and document transactions**
   - Treat Smart Bone authoring edits as native document edits.
   - Ensure preview is reversible and committed edits are undoable.
   - Keep evaluation itself read-only with respect to authored document state.

5. **Export/baking integration**
   - Define whether output is evaluated live, baked to ordinary animation keys, or both.
   - Ensure exported frames match viewport/playback evaluation.

6. **Nested symbols and identity**
   - Define scope of control bones and action targets across symbol instances.
   - Keep stable IDs distinct from numeric indices and display names.
   - Define coordinate-space conversion at explicit host/core boundaries.

These are host product capabilities, not merely additions to the current guest SDK.

## 7. Recommended vertical-slice feasibility test

Before implementing the full feature, AniCraft should prove one native evaluation path:

1. A document stores one Smart Bone definition and one authored action.
2. One stable bone ID provides an evaluated control angle at a requested scene/frame.
3. The action contains a neutral key and at least two corrective keys for one supported target channel.
4. The native evaluator maps the bone angle to action progress and evaluates the target correction without modifying authored keys.
5. The stage displays the correction during interactive posing and timeline scrubbing.
6. Test Movie/playback evaluates the same correction at the same frame.
7. Save/reopen preserves the control/action data.
8. Undo/redo correctly restores authoring edits.

If the host cannot expose this path, the blocker must be resolved in AniCraft architecture before expanding the FDM v2 feature model.

## 8. Recommended division of responsibilities

| Responsibility | AniCraft host | forma-deform |
|---|---:|---:|
| Document persistence and migrations | Owns | Adapter-neutral schema/types may be proposed |
| Smart Bone editor and timeline UI | Owns | None |
| Bone/rig evaluation | Owns authoritative pose | Consumes explicit evaluated inputs |
| Angle-to-progress mapping | Calls engine | Owns pure deterministic mapping |
| Action key interpolation | Supplies host semantics or explicit contract | Evaluates only against an agreed contract |
| Deformation math | Supplies geometry/coordinate context | Owns reusable pure calculation |
| Multiple-action composition | Owns final integration point | Implements only approved composition contract |
| Undo/redo | Owns | No host transaction ownership |
| Playback and export | Owns integration | Pure evaluator may be reused |

## 9. Risks and unresolved decisions

- The existing AniCraft plug-in guide does not document custom per-frame evaluation or plug-in UI panels.
- No arbitrary plug-in metadata storage is documented.
- Current actions.set commands are not Smart Bone animation actions.
- Bone keyed angle versus spring-adjusted angle remains undecided.
- Warp pin coordinates and the current deformation core's stage-space assumptions require explicit conversion/ownership decisions.
- Multiple controls affecting overlapping targets require composition semantics.
- Nested symbol evaluation and target identity need host-level definitions.
- The current guest SDK ABI uses unsafe pointer/length interfaces; the separate SDK memory-safety audit remains outstanding.
- The existing V4 MCP interactive test results must be verified before using them as evidence.

## 10. Feasibility verdict and next gate

**Verdict:**
- Plugin-only full Smart Bones: **Not supported by the currently documented plug-in contract.**
- MCP-driven interactive experiment: **Potentially feasible; live update and latency evidence still needs verification.**
- Native AniCraft Smart Bones backed by a reusable FDM engine: **Architecturally plausible, subject to a host-side vertical-slice proof.**

**Next gate:** Review and accept the Smart Bones product requirements, then open a focused AniCraft host architecture investigation for the document/action model and pose-evaluation hook. Only after that should ADM-001 through ADM-007 be reconciled and decided.

This document is an assessment, not authorization to modify AniCraft, merge Forma Deform pull requests, or implement the feature.


## 11. Source-level architecture trace (AniCraft fork, inspected 2026-11-21)

This section records concrete files and current extension points in `Mujshanan/anicraft` at commit `bcfea816478f2a00ab172cefe67f0bd5696b3886` (the fork's `main` at inspection time). Line links are pinned to that commit so the investigation can be repeated. These are observed code locations, not proposed API names.

### 11.1 Document model and persistence

| Concern | Actual source | Observed responsibility | Smart Bones implication |
|---|---|---|---|
| Bone identity and rig definition | [`crates/doc/src/armature.rs:60-62,117-155`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/armature.rs#L60-L155) | `BoneId` is a document-scoped numeric identity; `Bone` stores parent/child element IDs, local head/tail, rest transform, limits, spring and optional shape membership | Existing stable bone identity can be referenced. Do not bind by bone list index or name |
| Armature and skin storage | [`crates/doc/src/armature.rs:203-260`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/armature.rs#L203-L260), [`crates/doc/src/skin.rs:92-108`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/skin.rs#L92-L108) | `Armature` owns bones and shape `Skin` records; skin stores rest outline and per-anchor bone influences | AniCraft already has native bone-driven shape deformation, but not authored corrective actions |
| Pose keyframes | [`crates/doc/src/lib.rs:1238-1268`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/lib.rs#L1238-L1268) | `Keyframe` stores elements, classic/shape tween, motion tween, sound, frame script and armature `Pose` | A natural extension point exists at the document/keyframe model, but a Smart Bone action should not be conflated with `FrameScript` |
| Layer storage | [`crates/doc/src/lib.rs:1315-1367`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/lib.rs#L1315-L1367) | `Layer` owns keyframes and optional armature-related data | Candidate for armature-local control metadata; cross-layer or symbol-wide action ownership needs a separate model decision |
| Timeline and scenes | [`crates/doc/src/lib.rs:1596-1600`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/lib.rs#L1596-L1600), [`crates/doc/src/lib.rs:1886-1895`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696/crates/doc/src/lib.rs#L1886-L1895) | A scene owns a timeline; a timeline owns ordered layers | Symbol timelines are separate timelines, so scope and ownership across nested symbol instances must be explicit |
| Document root | [`crates/doc/src/lib.rs:1912-1946`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/lib.rs#L1912-L1946) | `Document` serializes scenes, library, assets, swatches, settings and next ID | If Smart Bone data is document-global, this is the root insertion point; if local to rigs/actions, a nested owner may be better |
| Format versioning and migration | [`crates/engine/src/format.rs:11-24,27-45,63-81`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/format.rs#L11-L81) | Format version is computed from document features; serialization writes the envelope; loading rejects newer versions and sanitizes decoded documents | Persistent Smart Bone data requires a deliberate format-version and backward-compatibility change, not plugin-side files |

### 11.2 Existing pose and geometry evaluation

| Concern | Actual source | Observed responsibility | Smart Bones implication |
|---|---|---|---|
| Per-frame display-list entry points | [`crates/doc/src/eval.rs:155-207`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/eval.rs#L155-L207) | `evaluate`, `evaluate_scene`, `evaluate_scene_camera`, and `evaluate_player` all build a `DisplayList` through `eval_timeline` | The document evaluator, rather than a renderer or UI-only callback, is the shared semantic center |
| Layer evaluation hook | [`crates/doc/src/eval.rs:675-720`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/eval.rs#L675-L720) | `eval_timeline` walks layers and calls `evaluate_shown(layer, frame, ..., doc.stage.fps)` before drawing their elements | This is the most direct existing hook for extending armature-aware evaluation; a Smart Bone stage must run before the layer's elements are emitted and before nested symbol traversal |
| Keyed bone pose interpolation | [`crates/doc/src/armature.rs:475-539,650-665`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/armature.rs#L475-L539) | `Pose` stores per-`BoneId` angle and slide; `pose_at` applies joint limits and interpolates the keyed pose using the armature keyframe tween | Smart Bone control input can be derived from the evaluated pose at this point, but the angle coordinate and spring policy must be decided |
| Forward kinematics and skin deformation | [`crates/doc/src/armature.rs:572-665`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/armature.rs#L572-L665) | `place` computes child transforms; `apply` applies rigid placement and skin deformation; `pose_at` supplies interpolated pose | This is an existing native deformation pipeline. Corrective deformation should be composed at a defined point around this pipeline, not by repeatedly rewriting keyframes |
| Spring-adjusted pose | [`crates/doc/src/spring.rs:101-128`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/spring.rs#L101-L128) | `sprung_pose` deterministically reconstructs spring swing from prior keyed motion up to the requested frame | The Smart Bone control angle decision must state whether it reads keyed `pose_at` or spring-adjusted `sprung_pose`; this choice affects deterministic evaluation and playback feel |
| Existing shape-warp interpolation | [`crates/doc/src/eval.rs:467-501`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/eval.rs#L467-L501), [`crates/doc/src/warp.rs:1-33`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/warp.rs#L1-L33) | Existing warp pins interpolate between ordinary keyframes and bend the rest outline; commands write authored pin positions into keyframes | This can inform a target channel for an MVP, but current warp data is ordinary frame animation, not a bone-driven corrective overlay |

### 11.3 Authoring commands and UI

| Concern | Actual source | Observed responsibility | Smart Bones implication |
|---|---|---|---|
| Bone authoring and posing commands | [`crates/engine/src/cmds_bone.rs:23-50,574-610`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/cmds_bone.rs#L23-L50) | Registers `bone.pose`, `bone.solve`, limits, spring settings and `bone.info`; `bone.pose` edits the keyed `Pose` | Add Smart Bone authoring as native document commands in this command layer if the existing command architecture is retained |
| Bone diagnostics | [`crates/engine/src/cmds_bone.rs:644-704`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/cmds_bone.rs#L644-L704) | `bone.info` reports keyed angle and separately reports spring swing | Existing diagnostic output is sufficient to inspect inputs, but it is not itself an evaluation hook |
| Motion key authoring | [`crates/engine/src/cmds_motion.rs:30-40,744-751`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/cmds_motion.rs#L30-L40) | Registers per-property key and easing commands for ordinary motion tweens | Useful reference for key/ease editing patterns; Smart Bone action tracks need a distinct target/timeline scope and must not be assumed to fit this existing per-layer motion tween model |
| Bone stage tool | [`src/bone_tool.rs`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/bone_tool.rs) | Draws bones from evaluated armature state and drives bone creation/selection workflows | Candidate UI integration point for identifying a bone as a Smart Bone control and opening its action editor |
| Timeline UI | [`src/timeline.rs`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/timeline.rs) | Renders and edits the existing layer/keyframe grid | Likely needs an action-editing mode or a dedicated action timeline; no Smart Bone action UI currently exists |
| Main application state and command dispatch | [`src/app.rs`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/app.rs) | Owns the UI application state and invokes engine commands | UI mode/selection state belongs here or in a focused panel module, not in the pure document evaluator |

### 11.4 Playback, stage rendering, and export

| Concern | Actual source | Observed responsibility | Smart Bones implication |
|---|---|---|---|
| Stage render jobs | [`crates/engine/src/render_service.rs:85-93,228-240`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/render_service.rs#L85-L93) | A frame job derives its display list from document and frame; render service rasterizes that display list | If Smart Bones are in the canonical document evaluator, stage rendering should inherit them without a separate renderer-only feature |
| Stage tile scheduling | [`src/stage.rs:1028-1055,1090-1100`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/stage.rs#L1028-L1055) | Stage schedules visible frame tiles and looks ahead during playback | Do not implement Smart Bone math in this UI tile scheduler; it should consume already evaluated frame content |
| Test Movie snapshot and player | [`src/test_movie.rs:168-189`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/test_movie.rs#L168-L189), [`crates/engine/src/player.rs:212-255`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/player.rs#L212-L255) | Test Movie snapshots the document; `Movie::step` advances `PlayerState` | Smart Bone definitions must be part of the snapshot document and evaluation must be a function of the snapshot and player state |
| Test Movie rendering | [`src/test_movie.rs:559-607`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/src/test_movie.rs#L559-L607) | It advances the player, calls `evaluate_player`, then renders the returned display list | This is a concrete playback acceptance path; the same evaluator extension must be reached by `evaluate_player` |
| Frame export | [`crates/doc/src/eval.rs:155-160`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/doc/src/eval.rs#L155-L160), [`crates/engine/src/anim.rs:342-360`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/src/anim.rs#L342-L360) | Export loops over frames and renders `anicraft_doc::evaluate(doc, frame)` | A document-level evaluator extension should automatically cover exports using `evaluate`; verify every export path rather than assuming all share this path |

### 11.5 Concrete extension-point conclusion

The source trace narrows the likely implementation seams to:

1. **Persistent model:** `crates/doc/src/armature.rs`, `crates/doc/src/lib.rs`, and `crates/engine/src/format.rs`. Decide whether a Smart Bone control/action is armature-owned, scene/timeline-owned, or document-owned before adding fields.
2. **Pure evaluator:** `crates/doc/src/eval.rs`, specifically the `eval_timeline` → `evaluate_shown` layer path. The current armature path is `pose_at` / `sprung_pose` → `armature::apply`. This is the strongest candidate for applying corrective action data before elements are expanded into the display list.
3. **Action track semantics:** `crates/doc/src/lib.rs` keyframe/timeline types and `crates/doc/src/motion.rs`. Reuse interpolation/easing primitives only after defining a Smart Bone action timeline; do not reuse `FrameScript` or blindly overload `MotionTween`.
4. **Native authoring commands:** `crates/engine/src/cmds_bone.rs` plus a dedicated action command module registered through the engine command registry. Existing commands mutate the document transactionally through `Session`.
5. **Authoring UI:** `src/bone_tool.rs`, `src/timeline.rs`, `src/app.rs`, and likely a new focused action-editor panel module.
6. **Playback parity:** `crates/doc/src/eval.rs::evaluate_player` and `src/test_movie.rs`; this must call the same Smart Bone-aware layer evaluator.
7. **Export parity:** `crates/doc/src/eval.rs::evaluate` and export consumers such as `crates/engine/src/anim.rs`.

### 11.6 Source-level caveats and corrections to the earlier assessment

- AniCraft already has a substantial native skeleton deformation pipeline: armature poses, forward kinematics, shape skinning, and deterministic spring reconstruction. The host is not missing all deformation capability; the missing product concept is bone-driven authored corrective actions and their evaluation.
- The central document evaluator is explicitly documented as pure and deterministic. The recommended extension is therefore a host-native evaluator stage, not an external per-frame plug-in callback.
- `bone.info` exposes the keyed constrained pose angle and separately computes the spring-adjusted angle/swing. The authoring UI's reported angle is diagnostic output only; production evaluation should consume internal typed pose values, not parse command JSON.
- `warp.pin_move` is an editing command that writes keyframe data. It is not the correct mechanism for live Smart Bone evaluation. The pure warp interpolation path is in `crates/doc/src/eval.rs` and `crates/doc/src/warp.rs`.
- This inspection first traced the fork's `main`; the separate `feat/smart-controllers-prototype` branch was then inspected and is documented in §11.7.
- No source files in AniCraft were changed. No API, schema, evaluator, or extension point is approved by this investigation alone.


### 11.7 Existing AniCraft Smart Controller prototype branch

The branch `feat/smart-controllers-prototype` was compared against `main` and inspected at head `b001b70e1444c3b194122f859b22b95cb098a22b`. It is three commits ahead of `main` and adds exactly these files:
- [`crates/controller/Cargo.toml`](https://github.com/Mujshanan/anicraft/blob/b001b70e1444c3b194122f859b22b95cb098a22b/crates/controller/Cargo.toml)
- [`crates/controller/README.md`](https://github.com/Mujshanan/anicraft/blob/b001b70e1444c3b194122f859b22b95cb098a22b/crates/controller/README.md)
- [`crates/controller/src/lib.rs`](https://github.com/Mujshanan/anicraft/blob/b001b70e1444c3b194122f859b22b95cb098a22b/crates/controller/src/lib.rs)

Observed prototype contract:
- `ControllerBinding` contains a stable `BoneId`, nested `InstancePath`, finite scalar `InputRange`, and inclusive `FrameRange`.
- `ControllerBinding::evaluate` clamps the scalar input and linearly maps it to a target symbol frame, including reversed output ranges.
- The crate is pure and deterministic; it does not mutate authored timelines.
- Its own README explicitly lists as unimplemented: durable document storage, resolving the effective bone angle from evaluated armature pose, applying the output to nested-symbol playback/evaluation, authoring/inspection UI, and continuous live playback.

**Critical product distinction:** this prototype is a bone-driven nested-symbol frame selector. It is a useful first Smart Controller mechanism and could support a corrective drawing/action represented as a nested symbol timeline. It is not yet a general Moho-style Smart Bone Action system that authors and evaluates corrective changes to arbitrary target channels/geometry. It also does not itself deform geometry.

Consequences for the current Smart Bones plan:
1. Keep the existing controller prototype as a distinct, narrow capability; do not silently redefine its frame-mapping semantics as deformation composition.
2. Decide whether a nested symbol action is an accepted first MVP representation, or whether the MVP must directly animate a geometry channel such as a warp mesh or shape skin.
3. If nested symbol frame switching is retained, the concrete host application point is the recursive symbol-instance evaluation in `crates/doc/src/eval.rs` (the symbol branch inside `eval_timeline`, where the instance's local frame is selected before recursively calling `eval_timeline` on the symbol timeline). The controller output must affect evaluation state, not rewrite keyframes.
4. If direct corrective geometry is required, the existing armature-aware layer hook at `evaluate_shown` and the geometry-specific evaluation functions are the relevant extension seams. The exact ordering relative to armature skinning and warp interpolation must be specified and tested.
5. The prototype's `InstancePath` is already defined in `crates/doc/src/player.rs` as the chain of element IDs from the main timeline to the nested instance. It is a more precise target locator than a single element ID for nested symbol playback.

This branch is not merged into `main`; this assessment does not authorize merging it or selecting it as the final implementation base.
