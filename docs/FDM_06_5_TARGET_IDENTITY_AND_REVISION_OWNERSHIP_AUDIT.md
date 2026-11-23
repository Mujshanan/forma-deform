# FDM-06.5 — Target Identity and Revision Ownership Audit

**Status:** Source audit completed; design evidence only  
**Audit target:** `Mujshanan/anicraft`  
**Pinned host commit:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Host baseline:** fork of `storytold/anicraft`  
**FDM design baseline:** FDM-06.4, PR #66, head `4942b81a6d0daa11f5d9adced43b3ad9286eabdf`  
**Scope:** Identity lifecycle, mutation entry points, dependency freshness, timeline evaluation, hierarchy, and native Asset Warp interaction.  
**Out of scope:** AniCraft implementation, FDM runtime implementation, changing native Warp, or changing Smart Controller/Form Deform composition semantics.

---

## 1. Executive result

The FDM-06.4 identity proposal is directionally compatible with the host, but **AniCraft does not currently expose the identity ledger or dependency-revision model needed to implement it safely**.

The audit found:

1. **A document-wide numeric allocator exists.** `Document::alloc_id` allocates IDs shared across elements, layers, and symbols. The allocator is persistent in the document. It is not an incarnation ledger and does not describe deletion/recreation history.
2. **Element IDs are routinely preserved, replaced, and remapped by different command families.** Duplication and paste allocate fresh IDs; ordinary edits preserve IDs; conversions, grouping/ungrouping, imports, and some rewrite operations replace IDs. These semantics need a single identity lifecycle contract.
3. **The host has a coarse session revision, not target/dependency revisions.** `Session::revision` advances for successful mutating command execution and undo/redo, but it is not a geometry, topology, transform, hierarchy, or target-lifecycle revision. It is also not sufficient alone to describe frame-dependent evaluation.
4. **Frame and evaluation context are independent inputs.** The display-list cache keys state, scene, frame, and editing context. `set_frame` updates the playhead without incrementing document revision. FDM freshness therefore needs an explicit evaluation stamp that includes frame/context, not just document revision.
5. **Timeline edits can change which artwork exists at a frame without changing an ElementId.** Keyframe insertion/removal, frame shifts, keyframe copying, tween creation/removal, and motion-key edits alter evaluated geometry and target presence.
6. **Layer hierarchy and rig parenting affect world placement.** Parent links and parent transforms are evaluated over time; changing them invalidates FDM's effective transform even if the target's local path is unchanged.
7. **Native Asset Warp is deeply integrated into Shape state.** It owns a rest path, mesh/pins, and a bent outline stored as the shape path. FDM must not infer that this state is an FDM dependency it can mutate or compose with.
8. **There is no single audited mutation hook covering every dependency class.** Commands are dispatched through a transaction wrapper, but mutation is performed in many command modules and direct helpers. A central host-owned invalidation/identity boundary is required before async FDM evaluation or durable authored extension state is safe.

**Conclusion:** FDM-06.4's proposed `DocumentId + ElementId + IncarnationId` and versioned host extension envelope remain viable as a design direction, but must be treated as proposed—not as capabilities already implemented by AniCraft. A conservative session/evaluation epoch plus canonical dependency fingerprints is the minimum safe first integration. Fine-grained revisions require explicit host-owned mutation coverage.

---

## 2. Pinned source inventory

All source observations below refer to host commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`. Blob SHAs are recorded to make the audit reproducible.

| Host source | Blob SHA | Audit role |
|---|---|---|
| `crates/doc/src/lib.rs` | `59559a0b4599fd0ec34c720004b42ceda3080883` | Document, IDs, Element, Shape, Timeline, Keyframe |
| `crates/doc/src/eval.rs` | `53e3e7489ebbbca76a436211d822168ecab0fb2e` | Frame evaluation and motion application |
| `crates/doc/src/warp.rs` | `46bbf1cd7012100e5ca06844d28214fdb3b99747` | Native Asset Warp state |
| `crates/doc/src/motion.rs` | `0e7178cde9ae1595e5446a722e6ff58bc4e4e3b2` | MotionTween target and animated properties |
| `crates/engine/src/lib.rs` | `de144ad9996bf1c29c52c28135b8f00a359ec338` | Session revision, command transaction, undo/redo, frame |
| `crates/engine/src/history.rs` | `204847cfb538942b495ef58e21a90d62fbfa1ee4` | Persistent snapshots and StateId |
| `crates/engine/src/editing.rs` | `43a6c919e324508db5cf8ccb5d418eeae876cbd5` | Nested symbol/drawing edit context |
| `crates/engine/src/cmds_draw.rs` | `eb279b835e248f97a397b9611ce0b00b1672470d` | Element creation, transforms, duplication, deletion, conversion |
| `crates/engine/src/cmds_path.rs` | `7c7201db127e6ab87d80a456e282cc90c7fc1209` | Path-point geometry edits |
| `crates/engine/src/cmds_raw.rs` | `093cd8d684e1b658869a0ef91741e55ed34f31c0` | Raw path edits |
| `crates/engine/src/cmds_rawpoints.rs` | `fb04262d8d4e16493fe7bb5a5e8a34f0c157d5db` | Raw point add/delete/convert |
| `crates/engine/src/cmds_shape.rs` | `e17a045b22f9b61120afbe27eb7e0214a2589c90` | Shape rewrite and replacement |
| `crates/engine/src/cmds_modify.rs` | `9bf988164ea0789d45222cbd65cf887661a2605a` | Copy/paste and transform |
| `crates/engine/src/cmds_group.rs` | `40fec49ebaf6cf45c8e173162e601e3febd68bdd` | Group/ungroup and ID replacement |
| `crates/engine/src/cmds_timeline.rs` | `7e9191f5f9dcf467faf11e1981a161978840488f` | Layer/keyframe/frame/tween mutations |
| `crates/engine/src/cmds_motion.rs` | `75a52d1a92a0babfb9753c0ab1704b6ede3819cf` | Motion path and property key edits |
| `crates/engine/src/parenting.rs` | `1df1796968874a5a4b583ed5aa2eea02741a1c69` | Rig-parent and layer placement changes |
| `crates/engine/src/cmds_warp.rs` | `3d3d923b9ce1ef2d9243d1f79814f00cbba4ec36` | Native Asset Warp edits |
| `crates/engine/src/cmds_bone.rs` | `24f93d85ded5b99b88a6e9e86af368d5f962c74d` | Armature/skin binding and rest-shape changes |
| `crates/engine/src/cmds_file.rs` | `5b834a0ce1347c5f612a71a0cae56dff6915f7b0` | Import and file operations |
| `crates/engine/src/cmds_scene.rs` | `f48b844e534097fc1a8c3b96c1bcd8221a43f030` | Scene duplication and remapping |
| `crates/engine/src/multi.rs` | `7f795c8be4a83c2cd757d39f817c21e3f9210da0` | Multi-frame element operations |
| `crates/engine/src/merge.rs` | `1d3b0c2763ffc1d17a4d2e0bfc5d6fb1131ce5f9` | Merge/replacement and document normalization |
| `crates/engine/src/transfer.rs` | `7bbee132c5d0b9ae96d4b96d499d550e2bd99bdf` | Cross-document/resource ID remapping |

This inventory is a targeted audit of the host's document/evaluation and principal artwork mutation paths. It is not a claim that every UI action, importer, exporter, or third-party extension has been exhaustively proven to route through these files.

---

## 3. Identity ownership findings

### 3.1 Existing identity primitive

- `ElementId` is a `u64` newtype.
- `Document::next_id` is the next allocator value for layers, elements, and symbols.
- `Document::alloc_id()` returns `max(next_id, 1)` and advances it with saturating addition.
- The allocator is document data; it is not a session-local ID counter.
- Element IDs occur in keyframe artwork and are referenced by selection, motion tweens, shape/armature relationships, and other document structures.

The allocator is useful for generating IDs but cannot by itself prove that a reference still identifies the same lifetime of an element. The FDM-06.4 incarnation token is therefore still necessary.

**Allocator edge case:** because increment uses `saturating_add`, exhaustion at `u64::MAX` does not produce a distinct next ID. FDM must not rely on allocator uniqueness as an unbounded mathematical guarantee. Host identity work should define a fail-closed exhaustion policy before persistent identity is introduced.

### 3.2 Lifecycle matrix

| Operation family | Observed host behavior | FDM identity rule required |
|---|---|---|
| Draw/create shape, text, camera, symbol instance | Allocates a new ElementId | Register a fresh incarnation |
| Duplicate selected artwork | Clones element data, assigns fresh ElementIds | New incarnation for every duplicate |
| Paste artwork / cross-document paste | Allocates fresh element IDs; resource/symbol IDs are remapped as needed | New target incarnations; remap all internal FDM references atomically |
| Ordinary transform/style/path-point edit | Mutates an existing element in place | Preserve incarnation; advance relevant dependency freshness |
| Shape rewrite/conversion | May retain one selected ID or allocate new IDs for generated pieces | Explicitly classify each output as retained identity or new identity; never infer from geometry |
| Group | Creates a group/layer identity and replaces selected top-level artwork with a new group element; children are moved into group content | New group incarnation; define whether moved children retain identity under their new ownership |
| Ungroup | Reconstructs exposed elements with fresh ElementIds in the audited path | Retire old group/child references and register new output identities; do not silently transfer by index |
| Convert to symbol / bitmap | Creates symbol/bitmap and replacement instance IDs; removes/replaces source elements | Explicit source-to-result identity map; old targets become stale unless host explicitly declares continuity |
| Import to stage / SVG import | Allocates fresh element/layer/symbol IDs | Fresh identities; imported extension references require complete-subgraph remapping or quarantine |
| Delete selection / layer removal | Removes artwork or its containing timeline/layer | Retire affected identities; layer deletion must enumerate every target it removes |
| Scene duplication | Allocates new layer and element IDs while copying scene content | New identities scoped to the duplicated scene/document graph |
| Undo/redo | Restores persistent document snapshots | Restore identity ledger with document state; advance runtime epoch independently |
| Open/replace document | Replaces Session document; clears history; clipboard survives | New session/evaluation epoch; DocumentId comes from persisted host envelope, not path/session |

**Important distinction:** an ElementId may appear in multiple keyframes as the same animated element. It is not a unique keyframe-occurrence ID. FDM target identity should attach to the host's element identity and must separately capture the frame/keyframe/evaluation context in the dependency stamp.

### 3.3 Special continuity hazard: motion tween target fallback

`MotionTween` stores an `ElementId` target. Its source documentation explicitly allows the only element in a keyframe to become the tween target when the stored ID is absent, so some existing host motion can survive element replacement.

That is a host animation compatibility behavior, **not a valid FDM target-resolution rule**. FDM resolution must require exact DocumentId + ElementId + incarnation match. It must never use the single-element fallback, element order, name, geometry similarity, or current selection to rebind an FDM target.

---

## 4. Revision and freshness ownership findings

### 4.1 What exists

`Session::revision` is documented as a UI invalidation counter. The command execution wrapper advances it after successful mutating commands; undo and redo also advance it. Gesture edits are coalesced into history snapshots. `StateId` identifies document content/history state and can return to a previous value after undo/redo.

The display-list cache is keyed by document state, scene, frame, and editing context. `Session::set_frame` changes the playhead and repairs selection but does not increment the document revision.

### 4.2 What does not exist

The audited host API does not expose dedicated monotonic revisions for:

- target incarnation/lifecycle;
- source Path2D geometry;
- topology or subpath structure;
- local element transform/pivot;
- layer rig-parent/hierarchy;
- timeline/keyframe/motion sampling;
- native Asset Warp rest mesh/pins/output;
- FDM authored payload.

A single session revision can conservatively invalidate every FDM result after a document mutation, but it is too coarse to support selective cache reuse. StateId alone is unsafe for stale-result rejection because undo/redo can restore historical content identity. Frame and edit context are additional evaluation inputs.

### 4.3 Required freshness model

For the first host integration, use a host-owned monotonic evaluation epoch and an immutable evaluation request stamp. A request/result pair is acceptable only if all captured fields still match at acceptance.

Minimum stamp:

```text
EvaluationStamp {
  document_id,
  session_instance_id,
  evaluation_epoch,
  state_id,
  scene_id_or_index,
  frame,
  edit_context_fingerprint,
  target_ref,
  source_geometry_fingerprint,
  local_transform_fingerprint,
  hierarchy_fingerprint,
  timeline_sampling_fingerprint,
  native_warp_fingerprint_or_explicitly_unsupported,
  fdm_payload_revision,
  fdm_policy_version
}
```

The session epoch must advance on every relevant successful document mutation, undo, redo, document replacement/reopen, target lifecycle change, and any external sampled input that can affect evaluation. Frame changes must produce a different evaluation stamp even if the document epoch is unchanged.

The first implementation may use the host's session-wide revision/epoch plus canonical fingerprints for dependencies. Do not claim fine-grained invalidation until each mutation path is covered and tested.

---

## 5. Mutation-to-dependency audit

| Mutation path | Representative source entry points | Dependency classes affected | Required invalidation / identity behavior |
|---|---|---|---|
| Element create/delete/duplicate | `cmds_draw::add_shape`, `duplicate`, `delete` | Target lifecycle; source set; timeline | Register/retire incarnation atomically; duplicate gets new identity |
| Element transform/pivot | `cmds_draw::move_by`, `set_transform`; `cmds_modify::apply`, `transform`, `remove_transform` | Local transform; evaluated world transform | Preserve incarnation; invalidate transform-dependent FDM output |
| Path point edits | `cmds_path::move_points`, `add_point`, `delete_points`, `convert_point`, `extend`; raw path commands | Geometry/topology | Preserve incarnation for in-place edits; advance geometry/topology freshness |
| Shape rewrites | `cmds_shape::rewrite` and conversion helpers | Geometry; sometimes target lifecycle | Must return an explicit old→new identity outcome for each result |
| Group/ungroup | `cmds_group::group`, `ungroup` | Target lifecycle; containment/hierarchy; transforms | New group identity; explicit child continuity/remapping policy |
| Copy/paste and cross-document transfer | `cmds_modify::paste_layer`, `transfer` | Target lifecycle; resource/symbol references | New element incarnations; remap extension references atomically |
| Timeline/keyframe/frame edits | `cmds_timeline::insert_frame_at`, `remove_frame_at`, keyframe insertion/removal/move, cross-layer frame move/copy | Timeline sampling; target presence; geometry | Invalidate affected frame ranges and target presence; preserve identity only where host semantics preserve it |
| Tween/motion edits | `cmds_timeline::tween_of_kind`; `cmds_motion::set_path`, key/ease operations | Evaluated transform; geometry at frame; timeline | Include frame and tween/key dependencies; stale requests rejected |
| Layer hierarchy/type/visibility | `cmds_timeline::set_type`, add/delete/move layer; `cmds_layer::set_properties` | Hierarchy, visibility, masks/guides, target availability | Invalidate layer-derived dependency graph; deletion retires contained targets |
| Rig parenting | `parenting::link` / set/remove parent; `Layer::place_all` | Hierarchy; animated world transform | Preserve target incarnation; invalidate hierarchy and effective transform |
| Native Asset Warp | `cmds_warp::edit_at`, `ensure`, add/remove; `Shape::set_warp` | Native warp mesh/rest path/bent path | Keep host-owned; FDM integration rejects unsupported composition and fingerprints relevant native state |
| Armature/skin binding | `cmds_bone::bind`, remove, set translation/weight, reshape | Rest geometry; binding; evaluated transform/deformation | Explicitly unsupported for initial FDM Path2D target scope unless host adapter can provide stable immutable source geometry |
| Import/scene duplication | `cmds_file` import paths; `cmds_scene::duplicate_scene` | Identity graph; resource refs; hierarchy | Fresh identities and complete graph remap; no partial cross-document references |
| Document replacement/open | `Session::replace_document` and file open | Document/session identity; all caches | New session epoch; resolve persisted DocumentId and rebuild/validate identity ledger |

The table is a dependency map, not a claim that the current host already emits these invalidation events. The existing command transaction boundary can provide a conservative global mutation signal, but it does not yet classify these dependency classes.

---

## 6. Domain-specific observations

### 6.1 Geometry and topology

Shape geometry is stored in `Shape.path`; native Asset Warp also stores its bent outline there. The path editing commands mutate the path in place through copy-on-write. Some editing commands operate on text-on-path copies or reshape armature skin shapes through a different helper.

For FDM, the immutable source snapshot must identify:
- the exact target and incarnation;
- the authored/source path representation;
- whether the path is a native Warp output or ordinary authored geometry;
- local transform and effective hierarchy transform;
- a canonical geometry fingerprint that includes command structure, not just bounds or sampled points.

The host's ordinary path edits preserve ElementId; they still invalidate the FDM source geometry.

### 6.2 Transform and hierarchy

Local transforms are mutated by draw/modify commands. Rig parenting is layer-level: changing a layer's `rig_parent` can also call `place_all` to rewrite placement across its keyframes. Evaluated element transforms additionally depend on tween/motion state and frame.

Therefore a target's local transform and its evaluated world transform are distinct dependencies. The initial FDM adapter must declare which coordinate space its core expects and construct that snapshot deterministically. A transform-only change must invalidate any cached output whose source coordinates or output placement depend on it.

### 6.3 Timeline and motion

A keyframe contains elements, classic/shape tween state, optional MotionTween, hints, and armature pose. Frame operations can insert/remove/shift keyframes or move/copy them between layers. MotionTween stores target ElementId and property tracks; evaluation applies sampled properties and may rewrite the evaluated element transform.

FDM must not cache by ElementId alone. Its cache identity needs scene, frame, editing context, relevant keyframe/tween sampling data, and target incarnation. Frame-range invalidation can be optimized later; correctness requires frame-specific freshness first.

### 6.4 Native Asset Warp boundary

Native Warp is part of `Shape`, with rest path, mesh/pins and output outline. Host Warp editing may update matching keyframes on the same mesh and reapply the warp. The shape path and Warp object are therefore coupled.

For the first FDM host scope:
- accept ordinary vector Path2D only;
- reject or report unsupported when a target has live native Asset Warp unless an explicit, separately approved interoperability contract exists;
- do not overwrite, clear, or compose native Warp state;
- do not reinterpret native Warp mesh revisions as FDM topology revisions.

This preserves the FDM-06.2/06.3/06.4 boundary and the existing native host feature.

### 6.5 Armature/skin and nested editing

Armature skin reshape and binding operations use separate paths and can alter rest geometry, weights, or element transforms. Editing context can target nested symbol/drawing content rather than the outer document timeline. The display cache already includes edit context; FDM's snapshot must do likewise or explicitly limit initial support to the main scene timeline.

Initial support should remain narrow: vector Path2D on a supported editable timeline target, excluding bitmap/text/symbol/group deformation, armature skin deformation, nested nonlinear propagation, and native Warp composition.

---

## 7. Required host conformance cases

These cases refine the FDM-06.3 test plan. They are acceptance requirements for future host implementation; **they were not executed by this read-only audit**.

| ID | Scenario | Required assertion |
|---|---|---|
| FDM-REV-001 | In-place path point edit | Same incarnation; geometry fingerprint/epoch changes |
| FDM-REV-002 | Add/delete/convert anchor | Same incarnation; topology fingerprint changes |
| FDM-REV-003 | Local transform or pivot edit | Same incarnation; transform dependency changes |
| FDM-REV-004 | Duplicate target | New ElementId and new incarnation; source unchanged |
| FDM-REV-005 | Delete then create replacement | Old TargetRef remains dangling; replacement has new incarnation |
| FDM-REV-006 | Convert shape to symbol/bitmap | Explicit identity transition; old reference never resolves to replacement implicitly |
| FDM-REV-007 | Group/ungroup | Group and generated children receive specified identities; no index-based rebinding |
| FDM-REV-008 | Paste/import into another document | New identities; complete extension graph remap or reject/quarantine |
| FDM-REV-009 | Keyframe insert/remove/shift | Frame-dependent target and sample stamps change |
| FDM-REV-010 | Move/copy frames across layers | Source/destination identity semantics are explicit and validated |
| FDM-REV-011 | Tween key/ease/path edit | Evaluated transform changes invalidate affected frame stamps |
| FDM-REV-012 | Rig-parent change | Effective transform/hierarchy fingerprint changes for descendants |
| FDM-REV-013 | Delete containing layer | Every contained target incarnation is retired |
| FDM-REV-014 | Native Asset Warp edit | FDM either rejects unsupported composition or observes changed unsupported-state fingerprint |
| FDM-REV-015 | Armature rest reshape/binding change | Unsupported targets are rejected; no stale ordinary-path snapshot reused |
| FDM-REV-016 | Nested symbol/drawing edit | Edit-context identity changes; no outer-target cache collision |
| FDM-REV-017 | Undo/redo | Persistent identity ledger restores with document snapshot; monotonic runtime epoch advances |
| FDM-REV-018 | Change frame without document mutation | Evaluation stamp changes even if StateId/session document revision does not |
| FDM-REV-019 | Replace/open document | Session instance/epoch changes; no old asynchronous result accepted |
| FDM-REV-020 | Failed/rolled-back command | Document and identity ledger roll back atomically; no leaked incarnation |
| FDM-REV-021 | ID allocator exhaustion | Allocation fails closed; no duplicate ID at saturation |
| FDM-REV-022 | MotionTween target missing but one element present | FDM TargetRef does not use host's single-element motion fallback |
| FDM-REV-023 | Multiple mutation commands in one gesture | One undo step may contain many changes; freshness advances for every visible mutation |
| FDM-REV-024 | Layer placement changes across keyframes | Hierarchy/effective-transform dependencies invalidate all affected samples |

---

## 8. Ownership contract required before implementation

| Concern | Owner | Required responsibility |
|---|---|---|
| DocumentId generation/persistence | AniCraft | Create, persist, validate, duplicate/remap |
| ElementId allocation | AniCraft | Fail-closed unique allocation |
| Incarnation ledger | AniCraft | Create/retire/restore tokens with document transactions |
| Target resolution | AniCraft adapter | Exact document + element + incarnation validation |
| Source Path2D snapshot | AniCraft adapter | Immutable source geometry and coordinate-space declaration |
| Dependency stamps/fingerprints | AniCraft | Provide authoritative host-owned values for transform, hierarchy, timeline, native Warp state |
| FDM authored model/payload | FDM | Versioned authored deformation data and migrations |
| Deformation evaluation | FDM core | Deterministic output and dependency declaration |
| Scheduling/cancellation/stale suppression | AniCraft | Reject obsolete results at acceptance time |
| Cache lifetime/invalidation | AniCraft | Own cache and key it by complete dependency stamp |
| Persistence/undo/redo | AniCraft | Include host envelope and FDM payload in atomic document snapshots |
| Apply/bake/render | AniCraft | Explicit commands, preflight, atomic rollback, shared evaluated geometry |

No responsibility above is transferred to the plugin SDK or to an installation-level plugin registry.

---

## 9. Blockers and decisions

### Blockers before implementation

1. **No durable DocumentId or incarnation ledger exists in the pinned host source.** FDM-06.4's proposed envelope remains a prerequisite.
2. **No target/dependency revision ownership exists.** Session revision is a coarse invalidation counter only.
3. **Mutation coverage is distributed.** The host needs a centralized identity/freshness mutation boundary or an equivalent audited event system.
4. **Frame/evaluation context must be part of freshness.** Document StateId/session revision alone is insufficient.
5. **Replacement semantics need an explicit operation result.** Grouping, ungrouping, conversions, rewrites, imports, and frame transfers cannot be handled by guessing continuity.
6. **Unsupported host deformation paths must be detectable.** Native Warp and armature/skin must not silently enter the ordinary Path2D pipeline.

### Decisions retained from prior milestones

- Host owns document lifecycle, identity, mutation, scheduling, cache, persistence, undo/redo, and rendering.
- FDM owns its authored model and deterministic deformation output.
- Initial target scope is supported vector Path2D only.
- Native Asset Warp remains host-owned and unchanged.
- Smart Controller binding evaluation and FDM pin evaluation remain independent under CCC-01.5.
- No FDM/native Warp or Smart Controller/FDM composition is introduced here.
- Synchronous evaluation precedes any asynchronous execution.
- No AniCraft or FDM runtime implementation is authorized by this audit.

---

## 10. Recommended next milestone

**FDM-06.6 — Host Mutation Invalidation Contract Design**

Design the concrete host seam that makes the audit enforceable, before writing runtime code:

1. Define one transaction-aware mutation outcome for create, retire, preserve, replace, and remap target identities.
2. Define where all document mutations must report geometry, transform, hierarchy, timeline, and native-Warp dependency changes.
3. Specify how gestures, multi-command operations, imports, frame transfers, undo/redo, and rollback update the identity ledger and monotonic evaluation epoch.
4. Define the immutable target snapshot and `EvaluationStamp` acceptance predicate.
5. Decide whether the first implementation uses conservative session-wide invalidation plus fingerprints, or introduces fine-grained revision counters immediately.
6. Add an exhaustive mutation-path coverage checklist and tests before enabling asynchronous FDM evaluation.

**Do not begin implementation until FDM-06.6 has mapped the mutation seam and its failure/rollback behavior.**

---

## 11. Audit limitations

- This is a source-level audit of a pinned commit, not a runtime instrumentation exercise.
- No AniCraft tests, FDM tests, builds, or CI workflows were run as part of this milestone.
- The audit records principal mutation paths and known direct helpers; future source changes require re-audit against a new pinned host commit.
- No source code in AniCraft was modified.
- No FDM evaluator, plugin adapter, extension envelope, identity ledger, revision counter, or cache integration was implemented.
