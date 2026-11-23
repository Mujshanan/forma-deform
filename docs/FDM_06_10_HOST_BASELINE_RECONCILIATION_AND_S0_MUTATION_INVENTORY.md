# FDM-06.10 — Host Baseline Reconciliation and S0 Mutation Inventory

**Status:** Read-only host audit and S0 inventory; implementation gate remains blocked  
**Parent plan:** FDM-06.9, FDM repo commit `da15372367a77af3f8a209b36c94ced4743ef110`  
**AniCraft repository:** `Mujshanan/anicraft`  
**Verified AniCraft base:** `main` at `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Git tree:** `80071fb3f714944e49608668871d4cf53a6159a5`  
**Scope:** Reconcile FDM-06.9 against the current host source; inventory mutation entry points, public mutable APIs, file/recovery/transfer paths, command/plugin paths, and runtime ownership evidence.  
**Change policy:** No AniCraft runtime changes, no FDM runtime changes, no test execution, no merges, and no implementation authorization.

---

## 1. Executive result

The previously pinned AniCraft baseline is still current. The branch endpoint for `main` and the recent commit listing both resolve to:

```text
2df7775907d1458ce653e7e6314ec8a5d7d565f5
Merge pull request #525 from storytold/docs/principal-review
```

The recursive Git tree is complete (not truncated) and contains 413 Rust files. The engine command registry aggregates **52 command-spec sources**.

The FDM-06.8/06.9 architecture remains directionally valid, but the source audit identifies several implementation details that need to be made explicit before any code work:

1. `Session::execute` is not an enforceable mutation boundary. Public `Session.doc`, public mutable `Document` APIs, and public `CommandSpec.run` callbacks permit bypass.
2. `execute` performs cut/floating/rig/motion/group settlement around and after the main command body. Some settlement occurs outside the point where change detection, StateId, revision, and history are updated.
3. The non-mutating command branch still invokes settlement functions capable of changing document state, but does not take a rollback snapshot or record a normal mutation history step.
4. Persistent state is currently only `Document`. File serialization, autosave/recovery, dirty-state tracking, and history all omit any future extension envelope/identity ledger.
5. The document decoder's private `DocumentFile` has no extension catch-all. Serde's default unknown-field behavior means unknown fields are not retained for lossless round trips.
6. Session ownership is serialized by current call patterns, not enforced by a thread/owner capability. The GUI remote bridge dispatches commands through the app's polling path; CLI/control handlers borrow `&mut Session` sequentially; autosave workers receive cloned `Document` values, not a shared mutable Session.
7. Frame/scene/editing context is mutable independently of document content. `set_frame` changes the playhead without incrementing document revision; scene/editing/frame fields are public.
8. Identity is currently a document-wide numeric allocator, not a target-lifetime identity system. Allocation saturates; transfer and duplication have different remapping rules.

**S0 disposition:** The host baseline and major mutation families are now source-pinned and inventoried. The audit is sufficient to prevent an unsafe implementation start, but it is **not a green light for S1–S9**. A machine-checkable command/helper registry and the owner-thread/dispatch contract remain required S0 closure items.

---

## 2. Reconciled source baseline

### 2.1 Repository and branch

| Item | Verified value |
|---|---|
| Repository | `Mujshanan/anicraft` |
| Default branch | `main` |
| Branch protection | API reported protection disabled |
| Main commit | `2df7775907d1458ce653e7e6314ec8a5d7d565f5` |
| Commit date | 2026-11-21 |
| Commit message | Merge PR #525, principal review of AniCraft's own decisions |
| Tree SHA | `80071fb3f714944e49608668871d4cf53a6159a5` |
| Recursive tree | Complete; `truncated=false` |
| Rust files in tree | 413 |
| Engine command-spec sources | 52 |

The previous FDM-06.1–06.9 host pin is therefore still valid. No intervening AniCraft commit needs reconciliation at this audit point.

### 2.2 Source blobs used for the critical findings

The following blob SHAs were fetched at the pinned commit. These are exact source references, not inferred file versions.

| File | Blob SHA |
|---|---|
| `crates/engine/src/lib.rs` | `de144ad9996bf1c29c52c28135b8f00a359ec338` |
| `crates/doc/src/lib.rs` | `59559a0b4599fd0ec34c720004b42ceda3080883` |
| `crates/engine/src/history.rs` | `204847cfb538942b495ef58e21a90d62fbfa1ee4` |
| `crates/engine/src/format.rs` | `2120a515ccdddfa7a5404294be884f4290783466` |
| `crates/engine/src/recovery.rs` | `5053f4a32de241cb536f46e6b8d3c73cb75cd88c` |
| `crates/engine/src/cmds_file.rs` | `5b834a0ce1347c5f612a71a0cae56dff6915f7b0` |
| `crates/engine/src/cmds_recovery.rs` | `c1a7f8058a92d90c3f53729b7809a6d613fdd63e` |
| `crates/engine/src/transfer.rs` | `7bbee132c5d0b9ae96d4b96d499d550e2bd99bdf` |
| `crates/engine/src/editing.rs` | `43a6c919e324508db5cf8ccb5d418eeae876cbd5` |
| `crates/engine/src/parenting.rs` | `1df1796968874a5a4b583ed5aa2eea02741a1c69` |
| `crates/engine/src/cmds_motion.rs` | `75a52d1a92a0babfb9753c0ab1704b6ede3819cf` |
| `crates/engine/src/cmds_scene.rs` | `f48b844e534097fc1a8c3b96c1bcd8221a43f030` |
| `crates/engine/src/cmds_group.rs` | `40fec49ebaf6cf45c8e173162e601e3febd68bdd` |
| `crates/engine/src/cmds_modify.rs` | `9bf988164ea0789d45222cbd65cf887661a2605a` |
| `crates/engine/src/cmds_timeline.rs` | `7e9191f5f9dcf467faf11e1981a161978840488f` |
| `crates/engine/src/cmds_frames.rs` | `30f4bffc596f1b86bb6f0a8135bb01fe99ca5789` |
| `crates/engine/src/cmds_draw.rs` | `eb279b835e248f97a397b9611ce0b00b1672470d` |
| `crates/engine/src/cmds_library.rs` | `9d9b85671d24646774f79926d73ba495fd78ab63` |
| `crates/engine/src/cmds_data.rs` | `8ab9aca1089024af13241cd528c35e1399d1d177` |
| `crates/engine/src/cmds_sound.rs` | `ce0d4703f7258c194924212c8d27e048fa62d16c` |
| `crates/engine/src/cmds_video.rs` | `df83241095e2da7b5b5edf921b20adf7314c4093` |
| `crates/engine/src/cmds_bone.rs` | `24f93d85ded5b99b88a6e9e86af368d5f962c74d` |
| `crates/engine/src/cmds_warp.rs` | `3d3d923b9ce1ef2d9243d1f79814f00cbba4ec36` |
| `crates/engine/src/cmds_plugin.rs` | `48cea8e7a94d01f7e9733deac6a1ddb1ef7f8440` |
| `crates/engine/src/plugin_export.rs` | `2604e904f85ca7a05c5499f787eb9c2a4401f214` |
| `crates/engine/src/render_service.rs` | `abf5c53b83a5d87ef6d6eb1811fb4b07dbabe676` |
| `src/app.rs` | `687524a801286094d1a62c69b47f415f7c4151e3` |
| `src/remote.rs` | `83554d9ac559d440edaabbf1230d79f48e691829` |
| `src/autosave.rs` | `7253613f4bf429c62312cdde58dc1608224418e7` |
| `src/tabs.rs` | `9986017f309c47dcbeaad1b9d5b2e09afb0626d2` |
| `src/stage.rs` | `704c8ca8e065dbafdb2be32ec69753c6d3e06233` |
| `crates/cli/src/lib.rs` | `6c67d3e06795b02bb9c98307d1e911e596e827a6` |
| `crates/cli/src/mcp.rs` | `13aa801b89ce939d40ddba8ad891b18b474f22b5` |
| `.github/workflows/ci.yml` | `0a4733afb72f12bfd6a2b0eb81967a37b954c104` |

Other command-module sources were fetched from the same pinned tree for the mutation-family scan. The command registry source list is recorded in section 11.

---

## 3. Mutation authority audit

### 3.1 Public Session escape hatches

At `crates/engine/src/lib.rs:391–430`, `Session` is public and exposes numerous public fields. Most importantly:

- `pub doc: Document`;
- `pub scene`, `pub editing`, `pub frame`, `pub layer`, `pub selection`;
- `pub revision`, `pub state`, `pub path`;
- public editor style, aids, text style, clipboard, frame clipboard, ease clipboard, plugin registry, and hit-area state.

Consequences:

- Any downstream crate can mutate `Session.doc` directly without `execute`, history, dirty-state update, cache invalidation, or future epoch receipt.
- Context can be changed directly without a context-generation increment.
- Public `state` and `revision` can be overwritten by downstream code.
- Making only `doc` private is not sufficient; the context and history-relevant public surface must be audited and split into immutable accessors and controlled methods.

### 3.2 Public command callback bypass

At `crates/engine/src/lib.rs:90–120`:

- `RunFn` is public and is a function pointer taking `&mut Session`.
- `CommandSpec` is public.
- `CommandSpec.run` is public.
- `commands()` returns the command specs.

A caller can invoke a command callback directly rather than through `Session::execute`. The current public API therefore has a second mutation route even if `Session.doc` were made private. The command callback type and field must become private/internal or be replaced with a constrained dispatch interface. The public command-description surface should not expose a callable mutable callback.

### 3.3 Public Document and nested mutation APIs

At `crates/doc/src/lib.rs:1968–2016`, `Document` derives serialization and has public persistent fields, including stage, scenes, library, bitmaps, sounds, library tree, publish settings, swatches, aids, eases, data, and next_id.

Additional mutable APIs include:

- `Document::alloc_id` at ~2103;
- `Document::timeline_mut` at ~2123;
- public mutable `Layer::keyframe_mut`;
- public mutable `Timeline::layer_mut`;
- public mutable `Keyframe::elements_mut_where`;
- public fields on `Scene`, `Layer`, `Timeline`, `Keyframe`, `Element`, and related document nodes.

Although `editing::edited_mut` is `pub(crate)` rather than externally public, downstream crates can mutate the public Document tree directly using public fields and nested mutable methods. `Document` is re-exported from the engine as `engine::doc`.

**Conclusion:** mutation authority must be enforced across the Document crate's public API, not just inside Session.

### 3.4 Allocator and state-counter risks

- `Document::alloc_id` uses `saturating_add(1)`; at exhaustion it can return the same maximum ID repeatedly.
- Transfer helpers advance `next_id` with saturating arithmetic.
- `history::fresh_state` uses atomic `fetch_add` without an exhaustion contract.
- `Session::revision` is incremented with ordinary `+= 1` in several mutation paths.
- No target-incarnation ledger exists.

Checked allocation and checked runtime counters must be prepared before publication; exhaustion must return an error without publishing a partial transaction.

---

## 4. Command execution and settlement audit

### 4.1 Main execution path

`Session::execute` is at `crates/engine/src/lib.rs:753` and currently:

1. resolves a `CommandSpec`;
2. checks automation file parameters and confirmation;
3. checks group restrictions;
4. calls `settle_cut` before the command;
5. branches on `spec.mutates`;
6. for mutating commands, snapshots state and clears/sets transient state;
7. starts rig tracking;
8. runs the command callback;
9. settles rig changes;
10. finishes motion auto-key if the command succeeded;
11. ends rig tracking;
12. compares Document equality and updates StateId/revision/history if changed;
13. fixes selection and holds group frame;
14. settles floating paste after the change comparison/history work.

The command metadata `mutates` is therefore the gateway to history/rollback, but it does not own all mutation effects.

### 4.2 Non-mutating command branch is not mutation-free

For `spec.mutates == false`, the current path still calls:

- `settle_cut` before the callback;
- `settle_cut` after the callback;
- `hold_group_frame`;
- `settle_floating`.

That branch does not take the same persistent rollback snapshot, perform the ordinary Document comparison, update StateId/revision, or record history. It is unsafe to treat `mutates=false` as proof that no persistent state can change.

### 4.3 Post-detection settlement gap

For mutating commands, the main Document comparison and StateId/revision/history update happen before `settle_floating`. The latter can merge floating pasted shapes into the document and update revision independently. The transaction boundary must move to after all command-owned settlement and validation.

### 4.4 Other mutation routes outside execute

- `settle_cut` / `heal_cut`: provisional cut restoration can replace `self.doc` and update revision outside a normal command transaction.
- `settle_floating`: merges pending floating artwork after the main command body.
- `begin_gesture` / `end_gesture` / `revert_gesture`: gestures maintain snapshots and coalesce history; cancellation restores document state and updates revision separately.
- `undo` / `redo` / `travel_to`: restore snapshots directly.
- `replace_document`: replaces the whole Session through `Session::new`, preserving selected editor-owned state.
- Public `Session::layer_mut` is currently `pub(crate)`, but command helpers and internal modules use it to mutate persistent layer/keyframe data.
- Direct public Document mutation can happen outside all these paths.

### 4.5 Nested plugin execution

`cmds_plugin.rs` implements `SessionHost`; plugin command requests call `execute_nested`. Nested commands currently use their own snapshot/restore logic but do not record an independent undo step. The outer plugin command records one undo step.

This is a useful compatibility behavior to preserve, but it must be re-expressed as a nested transaction sharing one outer draft and effect journal. Plugin ABI v1 should not gain direct access to the persistent bundle or mutable Document.

---

## 5. Persistent state, history, and file audit

### 5.1 History stores only Document

At `crates/engine/src/history.rs`:

- `Snapshot` contains `doc: Document`, StateId, scene, editing stack, frame, layer, selection, floating paste, command recording, and memory estimate.
- `History::record`, `undo`, and `redo` compare/account memory using Document only.
- `Session::snapshot` and `restore` copy/restore Document and editor context.

A future identity ledger and extension envelope must become part of the same snapshot unit. Runtime session identity and evaluation epoch must remain outside history.

### 5.2 Document serialization and unknown fields

At `crates/doc/src/lib.rs:1968–2053`:

- `Document` derives `Serialize` and `Deserialize` with `#[serde(from = "DocumentFile")]`.
- `DocumentFile` is private and enumerates known document fields.
- There is no flattened unknown-field map or extension envelope field in this structure.
- The conversion constructs a Document from only the declared fields.

Therefore unknown extension keys are not retained by this decoder. The FDM-06.9 unknown-payload requirement is not satisfied by the existing serializer. A host-owned versioned envelope and an explicit lossless compatibility strategy are required before authored FDM state can be saved safely.

### 5.3 Format save/load

At `crates/engine/src/format.rs`:

- `to_json` serializes the Document through the current format wrapper.
- `from_json` decodes through `DocumentDecoder` into Document.
- `load` reads the file and decodes Document.
- `save` writes a temporary file adjacent to the target and renames it into place; the code comment explicitly protects the old file from a crash mid-write.

The atomic file-replacement mechanism is a useful base to preserve. It currently serializes Document only and does not atomically save a wider persistent bundle.

### 5.4 Save and open command semantics

At `crates/engine/src/cmds_file.rs`:

- `open` loads a Document then calls `Session::replace_document`.
- `save` accepts a path or current path, calls `format::save(&s.doc, &path)`, updates `s.path`, and calls `mark_saved`.
- A separate explicit Save a Copy operation was not identified in the inspected file command path.
- Stage settings and import-to-stage mutate Document fields directly inside command callbacks.
- Import paths allocate IDs and create elements/layers/library assets within the existing Session command execution.

The FDM Save As/Save Copy identity distinction is a new explicit host contract; it is not implemented by the existing command path.

### 5.5 Dirty-state weakness for future extensions

`Session::is_dirty` compares `saved_state` with `state`. A direct mutation that bypasses execute can change Document without advancing StateId. Future envelope/ledger changes would also be invisible unless the persistent bundle becomes the dirty-state authority.

### 5.6 Autosave and recovery

- `src/autosave.rs` hands `s.doc.clone()` to the autosave worker.
- Autosave tracks `s.state` and `s.revision`, not a persistent bundle revision.
- `crates/engine/src/recovery.rs` writes and loads Document.
- `cmds_recovery.rs` restores via `replace_document`.

Autosave and recovery are separate persistence paths that must migrate alongside normal file save/load. A solution that only updates `format.rs` is incomplete.

---

## 6. Identity and transfer audit

### 6.1 Current allocator scope

The single `Document::next_id` allocator is shared across many numeric ID types, including elements, layers, symbols, and additional library/rig/warp entities. This provides document-local numeric uniqueness only where all allocation paths cooperate. It does not encode target lifetime or document identity.

### 6.2 Scene duplication

`cmds_scene::reid` creates new layer and element IDs for duplicated scene content, rewrites layer-parent links, and remaps motion tween target references. It is a local identity-remapping implementation, not a central target-identity service.

The audit must verify all ID-bearing nested data, including armature/bone, warp pin, group member, and other embedded references, against the exact duplication semantics. The observed helper is not proof that every future target type is remapped.

### 6.3 Clipboard and cross-document transfer

`transfer.rs` uses a session-origin token to distinguish same-origin from cross-origin carried data:

- same-origin paste can restore deleted Library items with their old numeric IDs;
- cross-origin paste allocates new IDs for brought library assets;
- some duplicate asset reuse is based on name/content equality;
- next_id is advanced using saturating arithmetic;
- the actual pasted artwork receives further per-element ID handling in command helpers.

This behavior is useful for ordinary copy/paste compatibility but is not equivalent to DocumentId + IncarnationId semantics. FDM target references must be remapped explicitly and may never cross DocumentId boundaries.

### 6.4 Group, conversion, merge/split, and delete

Source paths include group construction, shape conversion, raw geometry merge/split, Library duplicate/delete, and scene duplication. They allocate or preserve numeric IDs under operation-specific rules. There is no shared target transition journal. A future identity ledger cannot infer semantics from the resulting numeric ID or geometry; each operation needs an explicit transition map.

---

## 7. Runtime context and owner-thread audit

### 7.1 Context state

- `Session::set_frame` changes `frame` and fixes selection but does not advance document revision.
- `scene`, `editing`, and `frame` are public fields.
- Scene selection and symbol/group edit context are changed by commands and editor helpers.
- There is no EvaluationContextGeneration.
- Current display-list cache keys include StateId, scene, frame, and open-symbol/edit context; this is render/evaluation cache behavior, not a complete FDM target freshness stamp.

The FDM context stamp must be independent of document StateId and must be updated for frame/scene/editing-context changes even when persistent artwork is unchanged.

### 7.2 Observed dispatch patterns

| Path | Observed behavior | What it proves / does not prove |
|---|---|---|
| GUI command path | App calls `Session::execute` from the application event/update path | Normal UI mutation is routed through one Session owner in this path |
| GUI remote/MCP bridge | Remote client communicates with app; app polling path invokes `Live::execute` on `App.s` | Remote request does not directly mutate Session from the client thread |
| CLI/control | Request handlers receive `&mut Session`; JSON-lines loop processes requests sequentially | Borrowed access is sequential within this loop |
| Autosave | Worker receives cloned Document and path; it does not receive `&mut Session` | Background persistence is snapshot-based, not shared Session mutation |
| Render service | Render jobs receive cloned Document/display inputs | Render worker is not the Session mutation owner |

### 7.3 Owner-thread conclusion

The current application architecture uses a single mutable Session in the UI/CLI call paths, and remote app requests are marshalled through the app. However:

- Session has no owner-thread token or runtime assertion;
- no transaction API checks thread/dispatch ownership;
- public Session/Document mutation APIs do not enforce serialization;
- CLI and other embeddings can own a Session in their own execution context.

Therefore the precise contract is **single-owner by current call pattern, not formally enforced by the Session API**. FDM-06.9's owner-thread transaction design remains appropriate, but S0 must specify the concrete dispatch/owner invariant and decide whether to add debug assertions, a non-Send owner token, or another host-compatible enforcement mechanism. Do not claim the existing host already guarantees this invariant.

---

## 8. Mutation entry-point inventory

This is the source-level inventory of observed persistent mutation families. The categories intentionally include non-command helpers because a command-only list would miss important writes.

| Mutation family | Source owners / examples | Current route | Required effect/identity classification |
|---|---|---|---|
| Core artwork create/edit/delete | `cmds_draw`, `cmds_modify`, `cmds_shape`, `cmds_erase`, `cmds_paint`, `cmds_brush` | Direct Document fields and editable keyframes inside command callbacks | Target create/preserve/retire/replace; geometry/topology |
| Raw path and point editing | `cmds_path`, `cmds_raw`, `cmds_rawpoints`, `cmds_rawsub`, `merge` | `editable_keyframe`, raw helpers, keyframe element mutation | Geometry/topology; explicit split/merge replacement |
| Layer/timeline structure | `cmds_timeline`, `cmds_layer`, `cmds_frames`, `cmds_retime` | `edited_mut`, `layer_mut`, timeline vectors/keyframes | Layer identity transitions, hierarchy, timeline structure/sample |
| Scene lifecycle | `cmds_scene` | Scene vector insert/remove/swap and scene selection | Scene identity/context; duplicate target remap |
| Group/ungroup | `cmds_group`, `multi` | Group layer/element creation and keyframe rewrites | Wrapper identity; member preserve/replace rules |
| Symbol/library | `cmds_draw`, `cmds_library`, `editing` | Library/symbol/folder mutations, duplicate and delete | Symbol identity; internal reference remap |
| Bitmap/sound/video assets | `cmds_file`, `cmds_draw`, `cmds_sound`, `cmds_video`, `transfer` | Asset vector mutation and reference rewriting | Asset identity and import mapping |
| Data tables/bindings | `cmds_data` | `Arc::make_mut` on DataSet and binding changes | FDM payload/dependency invalidation where bindings affect evaluated properties |
| Stage/document settings | `cmds_file`, `cmds_view`, `publish`, `cmds_swatch`, `cmds_ease`, `cmds_script` | Direct persistent field replacement or Arc mutation | Document-level dependency or payload change |
| Rig/armature | `parenting`, `cmds_bone`, `cmds_camera`, `cmds_motion` | Rig settlement, bone/layer/keyframe mutations | Transform/hierarchy/armature state; explicitly unsupported FDM dependency marker |
| Native Asset Warp | `cmds_warp`, `cmds_draw`, `cmds_shape` | Pin/mesh/warp mutation | NativeWarpStateChanged; no composition with FDM |
| Motion/auto-key | `cmds_motion`, `cmds_actions`, `cmds_audio_keys`, `cmds_timeline` | Motion target edits, keyframe writes, finish_edit settlement | Timeline sample/structure and transform dependency |
| Filters/eases/paint/style | `cmds_filter`, `cmds_ease`, `cmds_stroke`, `cmds_width`, `cmds_swatch` | Keyframe or document property mutation | Geometry/style dependency; precise classification required per command |
| Text and shape conversion | `cmds_text`, `cmds_shape`, `cmds_modify` | Element/keyframe rewrites | Target preserve vs replace must be explicit |
| Cut/floating/gesture | `Session::settle_cut`, `heal_cut`, `settle_floating`, `begin_gesture`, `end_gesture`, `revert_gesture` | Session lifecycle helpers outside a single command body | Transaction boundary, history and identity rollback |
| Undo/redo/history travel | `Session::undo`, `redo`, `travel_to`, `History` | Snapshot restore | Restore persistent identity/envelope; advance runtime epoch |
| Open/replace/recovery | `cmds_file`, `cmds_recovery`, `recovery`, `Session::replace_document` | Decode Document then replace Session | Document replacement and new SessionInstanceId |
| Clipboard/import | `cmds_file::import_to_stage`, `transfer::bring`, `cmds_modify::paste`, `cmds_frames` | Multiple per-operation alloc/remap paths | Destination identities, internal reference remap, no cross-doc references |
| Plugin nested commands | `cmds_plugin::SessionHost`, `execute_nested` | Nested mutable Session command dispatch | Same outer transaction and shared effect journal |
| External file side effects | `cmds_file`, `publish`, `cmds_template`, `cmds_recent` | VFS write/rename/delete and export paths | Explicit preflight/commit; not rollbackable as ordinary in-memory mutation |
| Autosave/recovery persistence | `src/autosave.rs`, `recovery.rs` | Cloned Document sent to saver worker | Persistent bundle snapshot and matching dirty marker |
| Editor context changes | `set_frame`, scene select, symbol/group edit, active layer/selection | Public Session fields and context commands | Context generation; not necessarily persistent dirty state |
| Render/cache invalidation | `render_service`, Session display-list cache | StateId/frame/scene/view job keys | Keep render cache separate; FDM freshness predicate remains authoritative |

### 8.1 High-confidence direct write examples

The scan found direct writes including:

- `cmds_file.rs`: stage width/height/fps/background assignment; import-created element/layer/bitmap/symbol insertion.
- `cmds_scene.rs`: scene insert/remove/swap, scene duplicate and per-scene ID remapping.
- `cmds_draw.rs`: symbol/library/bitmap insertion and mutable symbol edits.
- `cmds_library.rs`: folder/library/data/symbol/bitmap/sound deletion and mutation.
- `cmds_data.rs`: DataSet table and binding mutation.
- `cmds_sound.rs`: sound/bitmap insertion, sound references rewritten across scenes/symbols.
- `cmds_video.rs`: video, sound, bitmap, layer and element creation.
- `cmds_ease.rs`: persistent saved-ease list replacement.
- `cmds_swatch.rs`: persistent swatch list replacement.
- `cmds_view.rs`: persistent aids and scene guide changes.
- `cmds_script.rs`: document class and symbol linkage/script changes.
- `publish.rs`: publish profile replacement.
- `cmds_erase.rs` and `cmds_brush.rs`: local snapshots and direct Document replacement on command error paths.

### 8.2 Command registry coverage

The engine's `commands()` function registers these 52 spec sources:

`cmds_file`, `plugin_export`, `cmds_envelope`, `publish`, `cmds_recent`, `cmds_desktop`, `cmds_drop`, `cmds_recovery`, `cmds_timeline`, `cmds_layer`, `cmds_camera`, `parenting`, `cmds_bone`, `cmds_warp`, `cmds_motion`, `cmds_retime`, `cmds_ease`, `cmds_hints`, `cmds_scene`, `cmds_frames`, `onion`, `multi`, `cmds_draw`, `cmds_view`, `cmds_group`, `cmds_library`, `merge`, `cmds_modify`, `cmds_path`, `cmds_raw`, `cmds_rawsub`, `cmds_rawpoints`, `editing`, `find`, `explorer`, `cmds_brush`, `cmds_filter`, `cmds_erase`, `cmds_paint`, `cmds_swatch`, `cmds_data`, `cmds_text`, `cmds_width`, `cmds_stroke`, `cmds_shape`, `cmds_plugin`, `cmds_sound`, `cmds_audio_keys`, `cmds_video`, `cmds_actions`, `cmds_script`, `cmds_macro`.

The registry's `CommandSpec.mutates` flag is currently a runtime behavior selector, not a verified effect manifest. S0 has not yet generated a canonical per-command ID → mutation owner/effect/test table. That must be produced before the coverage gate can be called machine-checkable.

---

## 9. External consumers and compatibility inventory

### 9.1 GUI/app shell

`src/app.rs` reads `Session.doc` broadly for stage dimensions, scenes, layers, symbols, sounds, swatches, and UI state. The main UI uses `Session::execute` for commands and `begin_gesture/end_gesture` for drag/edit interactions. No direct app-shell persistent Document mutation was found in the scanned call sites, but direct read compatibility is extensive.

Other UI consumers include `src/stage.rs`, `src/timeline.rs`, `src/convert_symbol.rs`, `src/motion_editor.rs`, `src/xform.rs`, and related panels. Most observed document use is read-only; they rely on public Session fields and therefore require immutable accessor migration.

### 9.2 Remote/MCP

`src/remote.rs` implements a `Live` host borrowing `&mut App`; remote command requests call `App.s.execute`. The client thread in the inspected test communicates through the port while the app poll loop processes the request. This is the evidence for the GUI's current serialized dispatch pattern.

`crates/cli/src/mcp.rs` implements a Session host whose command path executes against `&mut Session`. `crates/cli/src/lib.rs` exposes mutable Session request handlers and a sequential JSON-lines serve loop.

### 9.3 Autosave

`src/autosave.rs` clones Document for the worker and tracks StateId/revision. It must be migrated to a single immutable PersistentDocumentState snapshot and a saved/persisted bundle version. The worker should not receive a mutable Session capability.

### 9.4 Plugins

`cmds_plugin.rs` exposes document/elements read views and a constrained request protocol at the ABI boundary, but the internal `SessionHost` currently borrows the full mutable Session to route nested commands. Keep ABI v1 behavior stable while replacing this internal capability.

### 9.5 Public API migration risk

The largest compatibility cost is not just changing `Session.doc`. It is:

- `Session` public fields and downstream reads;
- `RunFn` and `CommandSpec.run` callable public callback;
- public Document fields and nested mutable methods;
- public engine re-export of `anicraft_doc`;
- tests and internal app panels that rely on direct access;
- autosave, CLI, MCP, export, recovery and plugin host adapters.

The first implementation branch must use compiler errors to enumerate all in-repository consumers and add an explicit migration inventory for downstream public API consumers.

---

## 10. S0 implementation blockers and required decisions

| ID | Blocker | Required S0 closure evidence |
|---|---|---|
| S0-B01 | No machine-readable per-command effect registry | Enumerate all command IDs from all 52 sources; record mutates flag, direct/indirect writes, side effects, identity transitions, tests |
| S0-B02 | Public mutable Session/Document/CommandSpec APIs | Consumer inventory across workspace and public API break plan; prove no callback/direct field mutation bypass remains |
| S0-B03 | Owner-thread contract not enforced | Document GUI, remote, CLI, plugin and worker dispatch; choose enforceable Session-owner strategy |
| S0-B04 | Settlement after command change detection | Map all cut/floating/rig/motion/group settlement call paths and define transaction ordering |
| S0-B05 | Non-mutating command path can settle persistent state | Audit all `mutates=false` specs and classify every side effect; split context commands from document transactions |
| S0-B06 | File decoder drops unknown fields | Prototype or specify lossless opaque extension envelope at the real file wrapper boundary |
| S0-B07 | Autosave/recovery stores Document only | Map exact snapshot, dirty marker, recovery metadata and restore ownership for full persistent bundle |
| S0-B08 | ID allocator saturates; no incarnation ledger | Determine every ID-bearing type and all nested reference remap paths; choose checked allocator/token facility |
| S0-B09 | StateId/revision are not runtime freshness | Enumerate all revision/state consumers; specify checked counters and context updates |
| S0-B10 | Save As/Copy and import modes not distinct in current commands | Map UX/command operations and decide explicit semantics without changing existing Save behavior unexpectedly |
| S0-B11 | History memory accounting only knows Document | Determine bundle ownership/sharing and exact accounting strategy |
| S0-B12 | External side effects mixed with command callbacks | Separate filesystem/network/process side effects into preflight/commit or post-commit protocols |

### 10.1 S0 owner-thread decision

Recommended direction, pending host-owner confirmation:

- The Session's mutation authority is the thread/event loop that owns the live Session.
- All GUI, remote, MCP, and automation commands are dispatched to that owner.
- Worker threads receive immutable snapshots and return immutable results/messages.
- Session transaction entry asserts owner identity in debug/test builds.
- Avoid a blanket `Send` requirement if it makes the Session transferable; avoid relying on `RefCell` or current call patterns as a formal ownership guarantee.
- CLI Session ownership is established by the CLI caller; the same invariant applies within its execution loop.

Do not introduce async FDM evaluation in this milestone.

---

## 11. Existing verification baseline

The inspected `.github/workflows/ci.yml` runs:

```sh
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

The workflow also runs the repository's cargo-deny license, ban, and source checks.

**No host tests, cargo commands, benchmarks, or CI runs were executed by this read-only audit.** The workflow file establishes the configured CI commands, not a passing result for the pinned commit. No baseline pass is claimed.

Before implementation, S0 must record:
- exact host commit;
- CI run URL and conclusion for that exact commit, or an explicit statement that no run exists;
- platform/toolchain details;
- test/clippy/cargo-deny outcomes;
- any existing failures classified as baseline failures.

---

## 12. Updated implementation sequence

The FDM-06.9 S0–S10 order remains, but S0 is now clarified:

- **S0-A — Source baseline and workspace inventory:** complete in this audit (commit/tree/source pins, 52 command-spec sources, module-level mutation families).
- **S0-B — Per-command mutation registry:** still required; enumerate IDs and effect ownership for every command and non-command mutator.
- **S0-C — Owner-thread and dispatch contract:** observed patterns documented; enforceable contract still required.
- **S0-D — Existing test/CI baseline:** CI commands identified; exact baseline execution evidence still required before implementation.
- **S0-E — Public API consumer migration inventory:** major GUI/CLI/MCP/autosave/plugin surfaces identified; complete workspace compile-consumer inventory remains required.

Only after S0-B through S0-E are resolved should the host implementation plan be converted into an authorized S1 branch.

---

## 13. Scope boundaries preserved

This audit does not:

- change AniCraft or FDM runtime code;
- add DocumentId, IncarnationId, an extension envelope, a transaction API, or evaluation epoch;
- change the plugin ABI or retire ABI v1;
- change Native Asset Warp;
- combine Native Asset Warp with FDM;
- combine Smart Controller and Form Deform;
- add asynchronous evaluation;
- add bitmap/text/symbol/group-wrapper/nested nonlinear FDM deformation;
- implement apply/bake;
- merge any PR or authorize runtime implementation.

CCC-01.5 remains binding: Smart Controller binding evaluation and Form Deform pin evaluation remain independent.

---

## 14. Recommended next milestone

**FDM-06.11 — S0 Registry and Ownership Contract Closure.**

It should finish the remaining S0-B through S0-E work:

1. generate the complete command-ID registry from the 52 spec sources;
2. trace every command's direct and indirect persistent writes, including `mutates=false` settlement paths;
3. enumerate non-command mutation helpers and public Document mutation APIs;
4. map all workspace consumers of `Session.doc`, `CommandSpec.run`, and mutable Document methods;
5. document the GUI remote dispatch and CLI ownership contract in enforceable terms;
6. record exact CI baseline evidence for the pinned AniCraft commit;
7. produce a final S0 gate matrix with no unclassified mutating entry points.

**Implementation gate:** FDM-06.10 is an audit deliverable, not authorization to begin S1. The unresolved blockers above must be closed and implementation must be separately authorized.
