# FDM v2 — AniCraft Host Capability and Integration Audit (M0)

**Status:** Initial source audit in progress  
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
8. These capabilities create reuse and adapter opportunities, but do not yet establish the versioned geometry snapshot, FDM topology persistence, nested nonlinear deformation, explicit composition graph, or runtime result contract required by ADM-001–007.

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

## 7. M0 deliverables and exit criteria

- [x] Native Asset Warp source and command surface documented.
- [x] Existing plugin SDK capability audit reconciled as prior evidence.
- [x] Initial native envelope capability recorded as a candidate, not assumed equivalent.
- [ ] Document identity, source geometry, and revision paths inspected.
- [ ] Hierarchy and transform evaluation paths inspected.
- [ ] Timeline sampling and animation storage paths inspected.
- [ ] Plugin persistence/extension APIs rechecked against current host source.
- [ ] Evaluation/rendering and cache invalidation paths inspected.
- [ ] Command transactions, undo/redo, and baking paths inspected.
- [ ] Smart Controller/pin separation verified against current tests and source.
- [ ] Every integration requirement classified with evidence and an owner.
- [ ] Required AniCraft host extensions listed separately from FDM implementation work.

## 8. Sources

- [AniCraft Asset Warp implementation](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs)
- [AniCraft Asset Warp commands](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs)
- [AniCraft plugin guide](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/plugins.md)
- [AniCraft command reference](https://github.com/Mujshanan/anicraft/blob/main/docs/guide/reference/commands.md)
- [AniCraft ADR 0079 — Asset Warp](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0079-asset-warp.md)
- [Forma Deform SDK capability audit](ANICRAFT_SDK_AUDIT.md)

**M0 status: In progress.** This first pass does not authorize changes to AniCraft. Any host extension requires its own design review and explicit authorization.
