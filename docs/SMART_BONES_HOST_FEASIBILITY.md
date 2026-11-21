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
