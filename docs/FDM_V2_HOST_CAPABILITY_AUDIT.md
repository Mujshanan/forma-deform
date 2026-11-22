# FDM v2 — AniCraft Host Capability and Integration Audit (M0)

**Status:** Second source-audit pass in progress  
**Audit date:** 2026-11-22  
**Host repository:** [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft)  
**Architecture authority:** [FDM v2 Architecture](FDM_V2_ARCHITECTURE.md)  
**Related audit:** [Existing AniCraft SDK Capability Audit](ANICRAFT_SDK_AUDIT.md)

> This is an evidence-based first pass, not a completed M0 sign-off. It records capabilities verified in cited source files and separates them from unverified host internals and design gaps.

## 1. Audit method

The current pass inspected AniCraft main-branch source for Asset Warp, warp commands, the plugin guide, and command reference. It reconciled those findings with the existing FDM SDK audit dated 2026-11-19.

Evidence labels:
- **Verified in source:** Directly supported by inspected implementation or host documentation.
- **Documented, implementation not yet audited:** Described by host documentation or the existing SDK audit, but implementation still needs inspection.
- **Not established:** No supporting capability found in the inspected surface. This is not proof that no internal implementation exists.
- **FDM responsibility:** Defined by accepted ADM decisions as FDM-owned.
- **Host extension candidate:** A possible gap requiring separate host-side design and authorization.

## 2. Executive findings

1. AniCraft has a shape-owned native Asset Warp implementation. It stores rest geometry, resolution, pins, and a fingerprint; its regular mesh is derived rather than serialized as authoritative topology.
2. Native Asset Warp uses rigid moving-least-squares (MLS) pin deformation at mesh vertices and piecewise-affine mapping between triangles.
3. Warp pins are stored in shape-local coordinates. Host warp commands accept stage-space coordinates and convert them using the inverse element transform.
4. Native Asset Warp is shape/bitmap scoped. The inspected command implementation rejects non-shape element kinds and directs users to break symbols or text apart first.
5. Warp pins participate in the host timeline: pin positions are keyed at the playhead and tween between keyframes. A move inside a tween keys the frame.
6. AniCraft also exposes a native 4×4 envelope command for selected shapes/drawing objects. The command reference says symbols, text, groups, and bitmaps cannot be bent with that operation. Its implementation and relation to FDM topology require further review.
7. The plugin guide documents command plugins as fresh, sandboxed runs that modify documents through host commands and produce one undo step. The existing SDK audit reports no documented arbitrary plugin metadata persistence or continuous custom playback evaluator.
8. AniCraft's document model has stable numeric ElementId, LayerId, and SymbolId values allocated from a document-local next_id counter. This is a useful host identity foundation, but it is not a cross-cutting geometry revision contract.
9. AniCraft has layer parenting (ADR 0113): child-layer artwork follows the first object on the parent layer through an affine transform at each frame. This is ordinary transform inheritance, not nonlinear parent-deformation propagation.
10. Document evaluation is a pure document/frame-to-DisplayList function. The editor caches display lists by document state, scene, frame, and open editing context. The state identity is useful for host cache invalidation but is not a persisted per-target geometry revision.
11. The current Document and DocumentFile schemas enumerate their fields and expose no generic plugin extension-data collection. The plugin host ABI exposes command, document, elements, and log requests; it has no general authored-data persistence operation.
12. These capabilities create reuse and adapter opportunities, but do not yet establish the versioned geometry snapshot, FDM topology persistence, nested nonlinear deformation, explicit composition graph, or runtime result contract required by ADM-001–007.

## 3. Evidence-linked capability matrix

| Area | Current finding | Classification | Evidence / next verification |
|---|---|---|---|
| Canonical artwork and shape model | Asset Warp is attached to a host Shape; a bitmap on stage is a shape with image fill | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs); inspect document Shape/Element definitions and serialization |
| Native Asset Warp persistence | Warp contains rest path, resolution, pins, and shown fingerprint; ADR 0079 places Shape.warp on the shape | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs); [ADR 0079](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0079-asset-warp.md) |
| Native mesh topology | Grid is derived from rest-outline bounds and resolution; vertices and triangles are generated, not stored as persistent mesh topology | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs) |
| Native deformation algorithm | Rigid MLS deforms grid vertices; triangles provide piecewise-affine mapping | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs) |
| Pin identity | PinId is a document-allocated numeric identity; pins store rest and current positions | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs) |
| Coordinate conversion | Pin data is shape-local; command inputs are stage px and converted through inverse element transform | Existing capability | [cmds_warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs) |
| Geometry staleness | Warp checks the fingerprint of the path it last displayed; stale warp is ignored/rebased by the next warp command | Existing capability | [warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs); [ADR 0079](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0079-asset-warp.md) |
| Native warp target coverage | Warp command resolves a shape and rejects other element kinds; symbols/text require breaking apart | Existing capability / limitation | [cmds_warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs) |
| Warp commands | warp.add, warp.pin_add, warp.pin_move, warp.pin_remove, warp.remove, and read-only warp.info are registered | Existing capability | [cmds_warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs) |
| Warp timeline behavior | Pin edits are keyframe-aware; pins tween between keyframes; edits inside tweens key the frame; a drag can be one undo step | Existing capability | [cmds_warp.rs](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs); verify exact current tests and timeline evaluator |
| Multi-layer/group nonlinear deformation | Native Asset Warp is per shape; inspected source does not establish a general parent/group deformation mapping | Not established | Inspect layer hierarchy, parent transforms, group/drawing-object representation, and evaluation traversal |
| Native envelope deformation | Command reference exposes a 4×4 Bézier envelope for selected shapes/drawing objects; excludes symbols, text, groups, and bitmaps | Existing capability (command surface); implementation review pending | [Command reference](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/reference/commands.md); inspect modify.envelope implementation and tests |
| Plugin execution and mutation | Plugins act through host commands; a command-plugin run is one undoable step and failed runs roll back | Documented, implementation not yet audited in this pass | [Plugin guide](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md); existing [SDK audit](ANICRAFT_SDK_AUDIT.md) |
| Plugin document metadata | Existing SDK audit found no arbitrary plugin-owned document metadata API in documented contract | Not established / likely host gap | Recheck current SDK and document extension APIs; do not infer absence from command guide alone |
| Continuous custom evaluator | Existing SDK audit found no command-plugin callback for continuous playback/scrubbing | Not established / likely host gap | Inspect frame evaluator and plugin runtime lifecycle |
| Timeline and animation | Host timeline, keyframes, tweening, and warp pin tweening exist | Existing capability | [Command reference](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/reference/commands.md); inspect document timeline model and interpolation |
| Host result application | Warp commands mutate host document; a general external FDM result-application contract is not established | Not established | Inspect engine evaluation/render pipeline and command transactions |
| Cache invalidation | Host cache lifecycle relevant to deformation has not been audited | Open audit item | Inspect geometry/render cache owners, revision propagation, and invalidation paths |
| Baking and provenance | Existing native warp edits geometry and retains warp data; FDM non-destructive bake/provenance contract is not established | Open audit item | Inspect native bake/geometry replacement commands and document undo transactions |
| Stable identity/revisions | Numeric document IDs exist for elements and pins; a cross-cutting versioned geometry revision contract for FDM snapshots is not established | Partial capability / open gap | Inspect document ID allocator, shape mutation paths, fingerprints, and cache revisions |

## 4. Native deformation details

### 4.1 Asset Warp

The host implementation establishes these concrete semantics:

- A warp belongs to an individual shape. A bitmap on stage is a shape with image fill.
- Persistent Warp data contains the rest outline, resolution, pins, and fingerprint of the path it last produced.
- The grid is derived from the rest-outline bounds and resolution. It is not persisted as an authoritative mesh.
- The grid is regular, with alternating triangle diagonals.
- Each pin has a stable document-allocated PinId, a rest point, and a current point.
- Both pin positions are stored in the shape's own coordinate space.
- Rigid MLS deforms grid vertices. The resulting mesh maps geometry piecewise-affinely.
- If the shape path no longer matches the warp fingerprint, the warp is stale; the next warp command rebases pins to their currently displayed positions on the new rest outline.

This is consistent with ADM-001: AniCraft owns source geometry and native Asset Warp. FDM must not copy or replace that native representation merely to obtain a lattice-like structure.

### 4.2 Warp command boundary

The inspected command implementation registers:
- warp.add
- warp.pin_add
- warp.pin_move
- warp.pin_remove
- warp.remove
- warp.info

warp.info is read-only. Mutating warp commands operate through AniCraft's document command path. warp.pin_move accepts stage-space coordinates and the implementation obtains the inverse element transform to edit the pin in shape-local space.

Warp data is propagated to keyframes that hold the same artwork; pin positions are edited at the playhead and tween between keyframes. This is a native host behavior that FDM should consume or preserve rather than recreate.

### 4.3 Scope limitations

The inspected warp command resolves a shape and rejects other element kinds. Its error message states that symbols and text must be broken apart first. Native Asset Warp therefore cannot be treated as a general group, layer, or symbol deformer.

The command reference also exposes a separate 4×4 envelope for selected shapes/drawing objects, but explicitly excludes symbols, text, groups, and bitmaps. This is a useful host capability to characterize, not evidence that it implements FDM's accepted general lattice/cage model.

## 5. FDM responsibility and host-extension implications

| Requirement | Current host position | Likely owner/action |
|---|---|---|
| Canonical source geometry and host IDs | Present in AniCraft document model; exact identity/revision APIs still need inspection | AniCraft |
| Native shape Asset Warp | Present and should be reused where sufficient | AniCraft; FDM adapter |
| Additional regular lattice and irregular cage types | Not established as reusable host deformation topology | FDM |
| Explicit source-to-deformation mapping | Not established as a general external contract | FDM defines semantics; adapter supplies host snapshot |
| Immutable, revisioned geometry snapshot | Not established as an FDM-facing public contract | AniCraft adapter/host contract |
| Parent-driven nonlinear deformation propagation | Not established by shape-specific Asset Warp | FDM semantics plus host hierarchy inputs; host integration gap may remain |
| Explicit multi-source composition graph | Not established as a host feature | FDM |
| Deformation animation properties/interpolation | Host timeline exists; FDM-specific property contract is not established | Shared ownership under ADM-005 |
| Retained non-destructive FDM baking and provenance | Not established | AniCraft transaction/persistence; FDM evaluation and provenance requirements |
| Versioned FDM runtime contract | Not established | FDM defines; AniCraft coordinates lifecycle |
| Persistent FDM extension data | Existing SDK audit reports no documented plugin metadata storage API | AniCraft host extension likely required; verify current document extension APIs |
| Continuous FDM evaluator | Existing SDK audit reports no command-plugin callback | AniCraft host extension likely required; inspect current evaluator before final gap classification |

## 6. Remaining M0 audit work

### Document identity and geometry revisions
- Locate ElementId, Shape, and document ID allocation.
- Identify geometry mutations and how revisions/fingerprints propagate.
- Determine whether stable element IDs survive keyframe interpolation, duplication, and symbol/drawing-object transitions.
- Identify existing geometry snapshot or immutable read APIs.

### Hierarchy and transforms
- Inspect layer parent relationships and parent-transform evaluation.
- Inspect groups, drawing objects, symbols, and nested element identity.
- Determine how host transform inheritance differs from nonlinear deformation propagation.
- Verify whether a stable hierarchy snapshot can be supplied to FDM.

### Timeline and animation
- Inspect keyframe, tween, easing, and property sampling structures.
- Determine how custom deformation properties could be stored and sampled.
- Verify transaction behavior for edits spanning frames.
- Define the host time representation exposed to an evaluator.

### Persistence and extension boundary
- Inspect document schema and serialization entry points.
- Inspect plugin manifest/runtime APIs and any extension-data mechanism beyond the documented SDK surface.
- Confirm whether versioned FDM-authored data can be stored in the document without host core changes.

### Evaluation, rendering, and cache lifecycle
- Locate the frame evaluation pipeline and geometry application stages.
- Identify cache owners, keys, and invalidation propagation.
- Determine whether host evaluation can accept an external immutable deformation result.
- Determine cancellation and stale-result handling points.

### Undo, commands, and baking
- Inspect command transaction and rollback implementation.
- Identify native geometry replacement/baking operations and source-retention patterns.
- Determine whether one FDM bake can be committed atomically across a target set and frame range.

### Smart Controller / pin integration
- Verify the current preview-only boundary against implementation and tests.
- Preserve the existing pin formula and behavior.
- Verify that no integration path combines Smart Controller and pin-deformation outputs.
- Do not change this behavior as part of M0.

## 7. Second source-audit pass — document, hierarchy, evaluation, and transactions

This pass inspected AniCraft's document model, pure evaluator, layer-parenting implementation, editor session, plugin command bridge, and undo history.

### 6.1 Document identity and geometry ownership

Verified in the document model:

- ElementId, LayerId, and SymbolId are distinct numeric identity types.
- Document allocates IDs through next_id; the allocator is documented as unique within that document.
- Element contains an ID, name, affine transform, color effect, and ElementKind.
- Shape contains a path and optional native Warp data.
- Shape, Element, Keyframe, Layer, Timeline, and Document use Serde serialization.
- DocumentFile is an explicit deserialization structure. Its fields include stage, scenes/timeline, library, bitmaps, sounds, library tree, document class, publish profile, swatches, aids, eases, and next_id. There is no generic extension-data map in the inspected current document schema.
- Session has a revision counter and a StateId. The StateId identifies document content for the editor's cache/history behavior; it is not a persisted geometry revision attached to each target.

**Architecture consequence:** stable host IDs can anchor adapter references, but ADM-001 still requires an explicit FDM-facing snapshot identity/revision contract. Do not treat Session.revision or StateId as a substitute for per-target geometry/topology revisions.

Evidence:
- [Document model and IDs](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/lib.rs)
- [Document evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)
- [Session and display-list cache](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/lib.rs)

### 6.2 Hierarchy and transform evaluation

AniCraft has two different layer-parent concepts:

- Layer.parent is folder/mask containment.
- Layer.rig_parent is the ADR 0113 parent-layer relationship.

The rig-parent relationship is evaluated from the first (lead) object on the parent layer at the requested frame. Child artwork follows the parent's position, rotation, scale, and skew. Colour effects, filters, and blending are not inherited. The relationship is a pure function of timeline and frame, and the host validates missing/self/cyclic/deep parent chains.

The document evaluator computes layer rig spaces and applies them as affine transforms while building the display list.

**Important distinction for ADM-003:** AniCraft already provides ordinary affine layer transform inheritance. It does not establish the accepted FDM nonlinear deformation propagation contract. FDM must not duplicate the host's affine transform hierarchy, and must not assume that an existing parent transform is equivalent to a nonlinear parent deformation mapping.

Evidence:
- [Layer and rig-parent data model](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/lib.rs)
- [Pure layer-parent evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/parenting.rs)
- [Layer-parent commands and stage/local conversion](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/parenting.rs)
- [Display-list hierarchy evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)

### 6.3 Timeline and animation model

Verified in the document model:

- Timeline owns an ordered stack of layers.
- Layer owns keyframes and optional armature data.
- Keyframe stores frame start/duration, elements, tween, motion tween, hints, label, sound, frame script, and armature pose.
- Element transform is authored on the element and is evaluated by the host.
- Native Asset Warp pins are part of shape-owned Warp data and their keyframe behavior is implemented by the host warp commands.

No generic FDM property-track or plugin-authored animation payload is present in the inspected Keyframe/Document schema.

**Architecture consequence:** ADM-005's shared-ownership model requires a supported host extension or an explicitly designed representation for FDM deformation animation. AniCraft's existing timeline and keyframe evaluator can provide time and sampling infrastructure, but must not be assumed to persist arbitrary FDM property values today.

Evidence:
- [Timeline, Layer, and Keyframe model](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/lib.rs)
- [Timeline commands](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_timeline.rs)
- [Asset Warp keyframe behavior](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs)

### 6.4 Plugin persistence boundary

The inspected SessionHost plugin bridge accepts command, document, and elements requests; the ABI also supports logging. Commands are checked against the host registry and execute as nested commands.

The Session keeps the installed plugin registry outside Document. The current Document and DocumentFile schemas have no generic plugin extension-data collection. The plugin guide also states that plugins belong to AniCraft on the computer, not to individual documents, and that command plugins modify the document only through host commands.

**Finding:** arbitrary durable FDM-authored document data is not supported by the inspected plugin contract or current Document schema. This is a concrete host-extension candidate, not something an FDM plugin can safely work around with local files or session memory.

Evidence:
- [Plugin command bridge](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_plugin.rs)
- [Document schema](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/lib.rs)
- [Plugin guide](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md)

### 6.5 Evaluation and cache lifecycle

AniCraft's document crate exposes pure evaluation functions, including evaluate(Document, frame) -> DisplayList and evaluate_scene/evaluate_timeline variants. The document evaluator does not mutate authored document state during frame evaluation.

The editor Session caches the current DisplayList using document StateId, scene, frame, and open editing context. Session invalidates this cache around mutating commands and restore operations.

**Finding:** AniCraft already has deterministic host frame evaluation and a state-keyed display-list cache. The inspected public surface does not establish an external FDM result injection point, an FDM dependency fingerprint, or asynchronous FDM result acceptance/cancellation contract.

**Architecture consequence:** ADM-007 must integrate with host scheduling and cache lifecycle. FDM must not create a competing authoritative frame evaluator or cache owner. The precise host hook remains an open audit item.

Evidence:
- [Pure document evaluator](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)
- [Session display-list cache and invalidation](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/lib.rs)

### 6.6 Transactions, undo/redo, and baking

Verified in the engine:

- CommandSpec declares whether a command mutates the document.
- Session.execute snapshots the pre-command document for mutating commands, clears relevant display-list cache state, executes the command, restores on failure, and records a successful document change.
- Undo/redo restore persistent Document snapshots through History.
- Plugin runs are one outer undo step; nested plugin commands do not create separate history steps.
- Session revision changes on document changes and undo/redo.
- Asset Warp is a persistent shape-owned deformation model.
- Native Envelope is different: its output is baked into ordinary paths, and its open mesh/history state is session-owned for the editing/undo workflow.

**Finding:** the host already supplies the transactional command and snapshot undo foundation required by ADM-006. The accepted FDM bake still needs a concrete host-native output representation, retained-source/provenance model, and verified frame-range transaction design. Native Envelope's baked editing model is not automatically the correct FDM bake representation.

Evidence:
- [Command execution and rollback](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/lib.rs)
- [Snapshot undo/redo history](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/history.rs)
- [Plugin nested command execution](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_plugin.rs)
- [Native Envelope implementation](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_envelope.rs)

### 6.7 Evaluation, rendering, result application, and cache lifecycle

The current AniCraft rendering path is a staged pipeline:

1. **Authored document and timeline evaluation.** `anicraft_doc::evaluate`, `evaluate_scene`, `evaluate_scene_camera`, and `evaluate_timeline` resolve keyframes, tweens, armatures/springs, symbols, groups, hierarchy transforms, and native Asset Warp into a `DisplayList`. Evaluation is pure: it reads the document and frame and does not mutate authored document state.
2. **Display-list construction.** Each `DisplayItem` contains a resolved path, local-to-stage transform, paint/stroke, color, layer/element identity, clipping/group information, decoded bitmap pixels, and (for a warped bitmap) a prepared native `WarpMesh`. A warped shape's outline is already bent in its evaluated path; a warped bitmap is carried through its mesh by the renderer.
3. **Render preparation and rasterization.** `anicraft_render::render` consumes an immutable `&DisplayList` and `&View`. It culls and prepares geometry/shaders in parallel, then rasterizes ordered operations in parallel horizontal bands. The CPU renderer is the deterministic oracle.
4. **Stage render service.** `RenderService` receives `FrameJob` values containing a cloned, structurally shared `Document`, frame, view, onion/camera context, and cache keys. It evaluates each distinct scene once per batch, then renders the requested tiles in parallel.
5. **Renderer substitution.** An optional `Rasterizer` callback receives only `&DisplayList` and `&View`, and returns pixels or a native GPU image. If it fails, panics, or returns an invalid image size, the CPU oracle draws the frame. This is a renderer replacement/fallback boundary, not a geometry-evaluation or deformation-result application API.

**No external geometry-result injection point was found in the inspected public pipeline.** The render callback cannot mutate the display list, return changed geometry, identify target revisions, or participate in picking/bounds evaluation. It is therefore not a valid FDM geometry integration hook. Post-raster pixel manipulation would also fail to provide coherent geometry for hit testing, bounds, masks, exports, and other display-list consumers.

#### Evaluation consumers and consistency requirement

There is more than one caller of document evaluation:

- `Session::display_list` caches the editing display list by document `StateId`, scene, frame, and open-editing context. The same list is used by hit testing, element bounds, and stage overlays.
- The stage's `FrameJob::display_list` path calls the document evaluator through `display_list_for` / `display_list_inside`, including onion and camera variants; it does not consume the Session's cached list.
- Built-in rendering/export paths evaluate movie frames through the document evaluator. The renderer's `render_frame` helper evaluates a frame then renders it. The exporter-plugin frame host likewise calls `anicraft_doc::evaluate` and serializes the resulting display-list items.
- Player evaluation has its own `evaluate_player` entry point for runtime movie-clip/button state.

**Integration consequence:** a deformation applied only in the stage renderer would make the picture disagree with hit testing, bounds, player output, and exported output. A supported FDM result must enter a shared host-owned evaluated-geometry path (or an explicitly equivalent host contract used by every relevant consumer), while leaving authored source geometry unchanged. The host must also decide how editing-only, camera, onion-skin, symbol, group, and player evaluation contexts resolve FDM data.

#### Cache identity and invalidation

The inspected cache layers are:

- Session display-list cache: `(StateId, scene, frame, open-editing context)`; mutating commands clear it, successful document changes receive a fresh `StateId`, and restore/undo paths clear it.
- Render scene cache identity: `(StateId, frame, onion settings)`.
- Rendered frame/tile key: document `StateId`, frame, view transform and dimensions, background, onion settings, and rasterizer identity.
- Finished images are held in a byte-bounded LRU cache. Stage tiles from previous states/zooms may remain temporarily as visual placeholders; the current key set determines which tiles are drawn as current. Switching documents explicitly forgets the prior document's pictures.

The current keys do not include an FDM dependency fingerprint, target geometry revision, deformation-source revision, animation sample revision, composition revision, or evaluator version as separate dimensions. If FDM authored data is stored inside the host Document and every mutation advances the document `StateId`, whole-document invalidation can remain correct but may be coarse. If runtime deformation inputs or authored data can change outside that state identity, the existing keys are insufficient and stale pictures could be reused.

#### Asynchronous work and stale results

The render worker has an urgent queue and a prefetch queue. New urgent batches replace pending urgent work; new prefetch calls replace the pending prefetch queue. Work already removed from the queue and marked busy is not cancelled. It completes and is inserted into the image cache under the key it was created with. The stage requests repaint and selects tiles by the current job keys, so a completed old-key image is not selected as a current tile, but there is no explicit source-revision validation or stale-result rejection at cache insertion/application time.

This is **latest-pending-work scheduling**, not a general cancellation or stale-result acceptance contract. There is no inspected FDM evaluation task handle, cancellation token, generation check, per-target revision validation, or atomic result-application API. Publish has a separate cancellation control checked by export work; that does not establish cancellation for live deformation evaluation.

#### Ownership and required host integration

| Concern | Existing AniCraft capability | FDM v2 implication |
|---|---|---|
| Pure frame evaluation | Deterministic document/frame evaluators producing DisplayLists | Reuse host time/context resolution; FDM supplies deformation-specific evaluation |
| Evaluated geometry | DisplayItems carry resolved paths/transforms and native warp data | Add a host-owned evaluated-geometry integration contract; do not mutate authored source artwork |
| Render backend | CPU oracle plus optional replaceable GPU Rasterizer | Renderer callback is not the FDM geometry hook |
| Shared consumers | Session display list, stage jobs, player evaluator, built-in and plugin exports | Apply FDM consistently before all geometry consumers diverge |
| Cache identity | Whole-document StateId and frame/view/onion/raster keys | Add explicit FDM dependency/revision identity or guarantee all relevant mutations advance host StateId |
| Background scheduling | Latest pending urgent/prefetch queues; in-flight jobs finish | FDM needs explicit cancellation/generation/freshness semantics if evaluation is asynchronous |
| Result application | No external deformation result acceptance/apply API found | AniCraft host extension required for validation and application of FDM results |
| Failure handling | CPU render fallback for renderer errors; render errors surfaced on FrameImage | FDM invalid/stale/incompatible results need host-level diagnostics and fail-closed handling |

**M0 conclusion:** the host already has a strong pure evaluation and rendering architecture, but not the FDM-facing evaluation/result boundary required by ADM-001 and ADM-007. The likely host-extension surface is before the shared evaluated-geometry representation is consumed by hit testing, stage rendering, player output, and exporters—not inside the rasterizer callback. The exact contract, data flow, and cache-key design remain open decisions; this audit does not authorize implementation or changes to AniCraft.

Evidence:
- [Document evaluation and DisplayList construction](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)
- [Session display-list cache, hit testing, and bounds](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/lib.rs)
- [Session stage-picture and display-list cache lifecycle](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/editing.rs)
- [RenderService, FrameJob, scene/frame keys, and worker queues](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/render_service.rs)
- [Stage tile selection and rendering requests](https://github.com/Mujshanan/anicraft/blob/main/src/stage.rs)
- [CPU renderer and immutable DisplayList input](https://github.com/Mujshanan/anicraft/blob/main/crates/render/src/lib.rs)
- [Native Asset Warp mesh rendering](https://github.com/Mujshanan/anicraft/blob/main/crates/render/src/warp.rs)
- [Exporter-plugin frame evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/plugin_export.rs)
- [Render pipeline ADR 0002](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0002-render-pipeline.md)
- [Tiled stage rendering ADR 0007](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0007-tiled-stage-rendering.md)

### 6.8 CCC-01.5 — Smart Controller and Form Deform separation

**Verification result: source/test contract reconciled with CCC-01.5.** The production paths remain independent, the conflicting integration test now tests only Smart Controller preview output, and the Smart Controller plugin no longer has a Form Deform dev-dependency. Existing Form Deform core tests continue to own pin-position formula coverage. AniCraft pin behavior remains unchanged.

#### Production Smart Controller paths

The V1 plugin `plugins/smart-controller/src/lib.rs`:
- Reads a bone angle through `bone.info` (or accepts a manual value).
- Validates and calculates `preview_slider`.
- Logs the preview.
- Rejects Transform mode in the V1 adapter.
- Does not issue timeline commands or mutate the document.

The canonical preview adapter also remains read-only. Its core evaluates a controller DAG and composes controller contributions; `TargetMapping::GeometryDeform` yields a typed `Vector2` target preview. `build_host_target_preview_plan` may query `warp.info` to verify the exact element/pin identity, but does not issue `warp.pin_move`. The composition module explicitly states that it never combines Smart Controller results with Form Deform pin-deformation evaluation.

The production Smart Controller crate depends on `smart-controller-core` and the AniCraft plugin SDK. `form-deform-core` is listed only as a dev-dependency in `plugins/smart-controller/Cargo.toml`, not as a production dependency.

#### Production Form Deform path

The separate `plugins/form-deform/src/lib.rs` command:
1. Reads the selected bone's keyed angle from `bone.info`; spring swing is opt-in.
2. Reads the requested shape and pin identity through `warp.info`.
3. Calls `form_deform_core::evaluate_pin_position` with its own authored neutral position, neutral angle, and per-degree gain.
4. Defaults to calculation-only mode. Only when `apply=true` does it call `warp.pin_move`.

The pure Form Deform formula is absolute and non-accumulating:

`target = neutral_position + (current_angle - neutral_angle) * gain_per_degree`

Its unit tests cover neutral pose, positive/negative angle results, repeatability, and rejection of non-finite input. Plugin unit tests cover stable bone-ID lookup, exact pin lookup, and malformed host responses. This is a separate command/evaluation path; it does not call the Smart Controller evaluator or consume its outputs.

#### Resolved test discrepancy

The conflicting test `bone_query_input_flows_through_controller_and_form_deform_math` has been replaced in `plugins/smart-controller/tests/plugin_suite.rs` by `bone_query_input_produces_independent_controller_geometry_preview`.

The replacement:
1. Reads the sample keyed bone angle from the same `bone.info`-shaped fixture.
2. Evaluates the Smart Controller `GeometryDeform` binding.
3. Asserts only the typed controller preview offset (`Vector2 { x: 6.0, y: -2.0 }`).
4. Does not construct a Form Deform neutral pin base, call `evaluate_pin_position`, or apply a host pin command.

The unused `form-deform-core` dev-dependency was removed from `plugins/smart-controller/Cargo.toml`. Form Deform's own core tests remain the isolated coverage for its absolute pin-position formula (neutral pose, positive/negative angles, repeatability, and non-finite rejection); its plugin tests remain responsible for host bone/pin lookup and malformed responses.

This removes the test-level executable composition contract without changing either production plugin path, the Smart Controller preview semantics, the Form Deform formula, or AniCraft's pin implementation. No new composition behavior is authorized. The source change was committed on the FDM documentation branch; CI/test execution was not performed as part of this source audit.

#### Existing AniCraft pin behavior — preserved by the inspected host implementation

AniCraft remains authoritative for pin storage, interpolation, and applying pin edits:

- `Warp` stores stable `PinId` values and each pin's rest/current positions in shape-local coordinates.
- `Warp::lerp` requires the same rest artwork and mesh resolution, pairs pins by stable ID, interpolates matching pin positions, and holds a pin at its existing value when no matching partner exists.
- The host warp command converts stage-space command coordinates through the inverse element transform before storing local pin coordinates.
- `Shape::set_warp` derives and stores the bent outline from the warp state.
- The host's `pin_poses_tween_between_keyframes_with_easing` test verifies midpoint interpolation, keyframe easing, stable pin identity across keyframes, and keying when a pin is moved during a tween.
- Other host tests cover unchanged rest pose, bending, undo/redo, bitmap rendering/export, persistence, and stale-artwork rebasing.

The FDM Form Deform core computes a desired absolute pin position; when explicitly applied, it delegates the edit to AniCraft's existing `warp.pin_move` command. It does not replace the host's MLS warp algorithm or pin tweening semantics.

#### CCC-01.5 classification

| Area | Current evidence | Result |
|---|---|---|
| Smart Controller V1 production plugin | Slider preview only; no timeline/document mutation | Separate |
| Canonical Smart Controller preview | Pure controller evaluation/composition; GeometryDeform target is preview data; host pin verification is read-only | Separate |
| Form Deform production plugin | Independent neutral/gain calculation; optional explicit `warp.pin_move` | Separate |
| Production/test dependency graph | Smart Controller depends on its own core and AniCraft SDK; Form Deform is no longer a Smart Controller dev-dependency | Independent
| Smart Controller integration test | Asserts only the independent controller GeometryDeform preview offset; no Form Deform call or dependency | Reconciled
| AniCraft pin algorithm and tween behavior | Stable-ID pin interpolation and host tests remain in place | Preserved in inspected host source |

Evidence:
- [Smart Controller V1 plugin](https://github.com/Mujshanan/forma-deform/blob/main/plugins/smart-controller/src/lib.rs)
- [Smart Controller canonical adapter and read-only host preview](https://github.com/Mujshanan/forma-deform/blob/main/plugins/smart-controller/src/adapter.rs)
- [Smart Controller target preview plan](https://github.com/Mujshanan/forma-deform/blob/main/plugins/smart-controller/src/preview_plan.rs)
- [Smart Controller composition post-pass](https://github.com/Mujshanan/forma-deform/blob/main/crates/smart-controller-core/src/composition.rs)
- [Form Deform plugin](https://github.com/Mujshanan/forma-deform/blob/main/plugins/form-deform/src/lib.rs)
- [Form Deform pure pin-position calculation and unit tests](https://github.com/Mujshanan/forma-deform/blob/main/crates/form-deform-core/src/lib.rs)
- [Smart Controller integration test with independent preview assertion](https://github.com/Mujshanan/forma-deform/blob/docs/fdm-v2-architecture-consolidation/plugins/smart-controller/tests/plugin_suite.rs)
- [Smart Controller plugin dependencies (no Form Deform test dependency)](https://github.com/Mujshanan/forma-deform/blob/docs/fdm-v2-architecture-consolidation/plugins/smart-controller/Cargo.toml)
- [AniCraft Warp model and pin interpolation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs)
- [AniCraft warp commands](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs)
- [AniCraft pin tween/easing and behavior tests](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/tests_warp.rs)

## 8. Updated capability conclusions

| Concern | Verified AniCraft capability | Remaining FDM v2 gap |
|---|---|---|
| Stable identity | Document-scoped numeric element/layer/symbol IDs and allocator | Explicit cross-boundary target identity/version contract |
| Geometry state | Shape path and optional shape-owned Warp; document content StateId | Per-target geometry/topology revision and immutable snapshot contract |
| Hierarchy | Folder/mask containment and separate affine rig-parent layer hierarchy | Nonlinear parent deformation mapping remains FDM semantics |
| Timeline | Timeline/layers/keyframes/tweens/armature pose and native warp pin tweening | FDM property schema, storage, and specialized interpolation contract |
| Persistence | Explicit serialized Document schema; no generic extension-data slot found | Host-owned versioned FDM extension data |
| Evaluation | Pure document/frame-to-DisplayList evaluation; shared geometry feeds picking, bounds, stage, player, and exports through distinct evaluation entry points | External FDM result application hook shared by all consumers |
| Cache | Session display-list cache plus render scene/frame/tile caches keyed by whole-document StateId and view context; pending render work is latest-wins | FDM dependency fingerprint/revision identity; explicit cancellation and stale-result rejection contract |
| Transactions | Atomic mutating commands, rollback, snapshot undo/redo | FDM multi-target/frame-range bake transaction details |
| Plugin access | Command/document/elements/log ABI; plugin commands nested in one undo step; exporter plugins read evaluated frames | Durable authored metadata and continuous evaluator/result application are not exposed by this contract |
| CCC-01.5 | Production paths and integration-test contract are separate; Form Deform formula tests remain isolated; AniCraft pin MLS/tween behavior remains host-owned | No remaining discrepancy identified; test execution still required in CI |

## 8.1 M0 integration gap and ownership matrix

The matrix separates capabilities already supplied by AniCraft, host extensions required for a complete FDM v2 integration, and responsibilities that remain inside Forma Deform. “Host extension required” means the inspected public host surface does not currently expose the needed contract; it is not a claim that the extension has been designed or authorized.

| Integration concern | Existing AniCraft capability | Remaining gap / required contract | Primary owner |
|---|---|---|---|
| Canonical target identity | Document-scoped element/layer/symbol IDs and document allocator | FDM adapter must resolve stable target references; host contract must define validity across deletion/reload and target-kind capability reporting | AniCraft owns identity; FDM owns adapter |
| Source geometry and snapshots | Shape paths, transforms, shape-owned Asset Warp, evaluated DisplayItems | Immutable geometry snapshot API with explicit coordinate space, target identity, source revision/fingerprint, supported geometry kinds | AniCraft exposes snapshots; FDM consumes |
| Per-target revisions | Whole-document StateId and command-driven document changes | Explicit target geometry/topology revision or a guaranteed dependency token suitable for stale-result checks and cache keys | AniCraft |
| Native pin deformation | Shape-specific Asset Warp, MLS pin deformation, stable PinId, pin tweening, undoable warp commands | No replacement needed for compatible targets; FDM should route supported pin edits through host commands and report unsupported targets | AniCraft owns algorithm/storage; FDM adapter reuses it |
| Lattice/cage topology | Native warp mesh is derived from rest outline/resolution; native envelope is a separate baked/session-oriented capability | Persistent FDM topology schemas for structured lattices and irregular cages, stable control-point IDs, connectivity, revisions, validation | FDM defines schema/semantics; AniCraft persists and owns document lifecycle |
| Source-to-deformation mapping | Native warp has its own shape-specific mapping; no general FDM mapping contract | Explicit mapping representation, coordinate spaces, compatibility checks, rebase/migration behavior | FDM |
| Hierarchy and nested deformation | Host owns layer containment and affine rig-parent inheritance | Host supplies hierarchy and transform context; FDM defines nonlinear parent-to-descendant propagation and mapping semantics; do not confuse affine inheritance with nonlinear deformation | AniCraft supplies context; FDM defines nonlinear semantics |
| Multiple-source composition | No general FDM composition contract in host | Explicit FDM composition graph, deterministic ordering, type/space/reference compatibility, cycle and stale-reference diagnostics | FDM |
| Smart Controller / Form Deform | Separate production paths; Smart Controller preview is read-only; Form Deform independently computes an absolute pin target | Preserve CCC-01.5 separation; do not combine their outputs absent a new explicit decision | FDM |
| Timeline and sampling | Host timeline, playhead, keyframes, tweens, armature pose, native pin tweening | Host sampling contract for FDM properties and topology-compatible animation values; FDM defines property semantics and specialized interpolation | AniCraft owns time/tracks/sampling; FDM owns property semantics |
| Persistent authored data | Explicit Document schema; no generic plugin extension-data API found in inspected contract | Versioned host-owned extension storage, save/load, migration, undo/redo integration | AniCraft provides persistence boundary; FDM defines payload schema |
| Shared geometry evaluation | Pure document evaluators produce DisplayLists for editing, stage, player and export consumers | One supported FDM integration point before shared geometry consumers diverge; no renderer-only or stage-only injection | AniCraft |
| Evaluation inputs and outputs | Host resolves document/frame contexts; renderer consumes immutable DisplayLists | Versioned runtime contract: immutable snapshots and sampled inputs in; typed target results with source revisions/coordinate space/compatibility out | FDM defines deformation contract; AniCraft owns host boundary |
| Scheduling and stale-result handling | Render queues and caches exist; in-flight render work is not cancelled; no FDM result acceptance API found | Host scheduling, cancellation/generation, freshness validation, atomic result acceptance, fail-closed diagnostics | AniCraft |
| Cache invalidation | Session/render caches use StateId and frame/view context | FDM declares dependencies/fingerprints; host includes relevant revisions in cache identity and invalidates consistently | FDM declares dependencies; AniCraft owns cache |
| Bake and undo | Host commands, rollback, snapshot undo/redo | Host transaction/command for atomic bake and source retention; FDM supplies evaluated geometry and deformation provenance requirements | AniCraft owns mutation/transaction; FDM owns deformation evaluation/provenance fields |
| Export/player consistency | Separate host evaluation consumers exist | Ensure the same FDM-evaluated geometry is consumed by editing, hit testing, bounds, stage rendering, player, and all export paths | AniCraft integration; FDM evaluator |

### 8.2 Required AniCraft host extensions (separate from FDM work)

These are the host-side capabilities indicated by the audit. They are requirements for a complete native FDM v2 integration, not implementation instructions or accepted designs for AniCraft.

1. **Versioned FDM document extension storage.** Persist FDM-owned payloads through an explicit document extension boundary, including save/load, schema migration, and undo/redo participation. The host retains ownership of the document and persistence lifecycle.
2. **Stable target and geometry snapshot contract.** Resolve host target IDs and supply immutable source geometry snapshots with declared coordinate spaces, target kind/capabilities, and revision/fingerprint information.
3. **Shared evaluated-geometry extension point.** Integrate deformation results before the common evaluated geometry is consumed by hit testing, bounds, stage rendering, player evaluation, and built-in/plugin exports. A rasterizer callback or stage-only overlay is insufficient.
4. **Timeline property sampling boundary.** Store and sample FDM-specific animatable values using AniCraft's playhead/time/tracks, while preserving host ownership of timeline infrastructure.
5. **Scheduling, freshness, and cache contract.** Define dependency-aware invalidation, cache identity, cancellation/generation semantics, stale-result rejection, and atomic publication/application of valid results.
6. **Host transaction/bake boundary.** Provide undoable atomic document mutation for explicit bake/apply operations, including retained-source restoration and deterministic redo.
7. **Capability and diagnostic reporting.** Expose supported target kinds and explicit failures for stale IDs, incompatible geometry, unsupported target types, and rejected results.

The exact API shapes, storage format, cache-key design, and evaluation scheduling policy remain open. No AniCraft code has been changed.

### 8.3 Forma Deform-owned implementation work

1. Define versioned persistent schemas for deformation domains, structured lattices, irregular cages, stable control-point IDs, topology revisions, and source mappings.
2. Implement topology construction/validation, source-to-deformation mapping, rebase/migration rules, and compatibility diagnostics.
3. Define deformation algorithms and their explicit input/output types, coordinate spaces, reference geometry, numerical tolerances, and degenerate-input behavior.
4. Define nonlinear nested-deformation propagation separately from AniCraft's ordinary affine transform inheritance.
5. Define the explicit composition graph and deterministic sequential, parallel, and weighted-blend semantics; preserve the CCC-01.5 Smart Controller/Form Deform separation.
6. Define deformation-specific animatable property schemas, interpolation rules, topology-revision compatibility, and stable animation references.
7. Implement deterministic pure evaluation against immutable host snapshots and sampled inputs; return typed results and dependency descriptors without mutating authored data.
8. Define deformation-specific bake requirements and provenance payloads; AniCraft remains responsible for document mutation, transactions, and undo/redo.
9. Reuse AniCraft Asset Warp for supported shape-pin deformation instead of introducing a competing pin solver without a measured need.

## 9. M0 deliverables and exit criteria


- [x] Native Asset Warp source and command surface documented.
- [x] Existing plugin SDK capability audit reconciled as prior evidence.
- [x] Initial native envelope capability recorded as a candidate, not assumed equivalent.
- [x] Document identity and source geometry paths inspected; per-target revision contract remains an open gap.
- [x] Hierarchy and ordinary transform evaluation paths inspected; nonlinear deformation propagation remains an FDM contract.
- [x] Timeline/keyframe model inspected; FDM-specific property storage remains a host-extension gap.
- [x] Current plugin bridge and Document schema rechecked; no generic plugin extension-data API was found in the inspected contract.
- [x] Pure document evaluation and editor display-list cache path inspected.
- [x] External result application, async cancellation, and full render/cache invalidation paths inspected; no external geometry-result API or FDM stale-result contract was found.
- [x] Command transactions and snapshot undo/redo inspected; FDM bake representation/provenance remains open.
- [x] CCC-01.5 source/test contract reconciled: production paths remain separate; the Smart Controller integration test asserts only its own preview output; the Form Deform dev-dependency was removed; AniCraft pin behavior remains unchanged. CI/test execution is still pending.
- [x] Every identified integration requirement classified with source evidence and an owner in the M0 integration gap and ownership matrix.
- [x] Required AniCraft host extensions listed separately from Forma Deform-owned implementation work.

## 10. Sources

- [AniCraft Asset Warp implementation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs)
- [AniCraft Asset Warp commands](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs)
- [AniCraft document evaluation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/eval.rs)
- [AniCraft render service](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/render_service.rs)
- [AniCraft CPU renderer](https://github.com/Mujshanan/anicraft/blob/main/crates/render/src/lib.rs)
- [AniCraft stage tile rendering](https://github.com/Mujshanan/anicraft/blob/main/src/stage.rs)
- [AniCraft exporter plugin frame host](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/plugin_export.rs)
- [AniCraft plugin guide](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md)
- [AniCraft command reference](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/reference/commands.md)
- [AniCraft ADR 0079 — Asset Warp](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0079-asset-warp.md)
- [Forma Deform SDK capability audit](ANICRAFT_SDK_AUDIT.md)

**M0 status: Audit complete.** The source-based capability audit, CCC-01.5 reconciliation, integration gap/ownership matrix, and separate host-extension list are complete. Implementation-level contracts and API designs remain open; this audit does not authorize changes to AniCraft. The modified Smart Controller test has not yet been executed in CI.
