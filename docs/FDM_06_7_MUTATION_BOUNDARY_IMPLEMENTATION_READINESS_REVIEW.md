# FDM-06.7 — Mutation Boundary Implementation Readiness Review

**Status:** Readiness review; implementation authorization not granted  
**Host:** `Mujshanan/anicraft`  
**Pinned host source:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**FDM-06.6 contract:** PR #69, branch `design/fdm-06-6-mutation-invalidation-contract`  
**Scope:** Verify the proposed mutation boundary against concrete host mutation, identity, history, timeline, import, and rendering paths.  
**Decision:** **Not ready for runtime implementation.** The architectural direction remains valid, but the current host exposes bypasses and lifecycle ambiguities that must be resolved first.

---

## 1. Executive decision

FDM-06.6 correctly selects a host-owned transaction boundary, explicit target identity transitions, a monotonic session evaluation epoch, immutable snapshots, and atomic stale-result acceptance. The pinned AniCraft source confirms that `Session::execute` is the central command entry point and already provides rollback/history behavior for registered mutating commands.

That central entry point is not yet a complete mutation boundary.

The source audit found the following implementation blockers:

1. **Public mutable state bypasses the command boundary.** `Session::doc` is public, and `Document` exposes public mutable fields and mutation helpers such as `alloc_id` and `timeline_mut`. A caller can change authored content without a command receipt or epoch update.
2. **Mutation side effects exist outside the main command commit.** `settle_cut` / `heal_cut`, `settle_floating`, gesture cancellation, and document replacement can change persistent state and revision through separate paths. These need explicit transaction/receipt semantics.
3. **The existing revision is not a safe evaluation epoch.** It is a UI invalidation counter, is incremented with unchecked `+= 1`, and is not a dedicated, overflow-checked session freshness token.
4. **Identity is numeric and distributed.** `Document::alloc_id` saturates at the integer limit, and many command modules allocate IDs directly. No incarnation ledger exists. Group/ungroup and scene duplication have materially different identity behavior.
5. **History snapshots contain the Document but no separate identity ledger or extension envelope.** The proposed persistent identity state cannot be bolted on later without changing snapshot, equality, serialization, migration, and history-memory accounting.
6. **Timeline and nested edit context affect evaluation independently of document revision.** Frame and scene/editing context are mutable session state. Motion tween sampling, frame transfer, and rig parenting are dependency-changing paths.
7. **The host's renderer/cache is not an FDM acceptance gate.** Existing display-list and render caches use their own keys and clearing rules. They do not provide the complete FDM snapshot stamp or atomic result acceptance contract.

The review therefore recommends **FDM-06.8 — Mutation Boundary and State Ownership Specification** before runtime implementation. FDM-06.8 should resolve API encapsulation, transaction ownership, post-commit side effects, session concurrency, persistent identity storage, and exact target transition semantics. Implementation should remain separately authorized.

---

## 2. Evidence baseline and source inventory

All source observations below refer to AniCraft commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`; the audit does not assume the branch has remained unchanged since that commit.

| Source | Pinned blob SHA | Relevance |
|---|---|---|
| `crates/engine/src/lib.rs` | `de144ad9996bf1c29c52c28135b8f00a359ec338` | Session fields, command registry, execute/nested execution, gestures, replacement, undo/redo |
| `crates/engine/src/history.rs` | `204847cfb538942b495ef58e21a90d62fbfa1ee4` | Snapshot and undo/redo storage |
| `crates/doc/src/lib.rs` | `59559a0b4599fd0ec34c720004b42ceda3080883` | Document/DocumentFile, ID allocator, persistent schema |
| `crates/doc/src/eval.rs` | `53e3e7489ebbbca76a436211d822168ecab0fb2e` | Frame-dependent evaluated output and timeline sampling |
| `crates/engine/src/cmds_timeline.rs` | `7e9191f5f9dcf467faf11e1981a161978840488f` | Frame/keyframe/tween/layer mutations |
| `crates/engine/src/cmds_modify.rs` | `<escape>not captured in this review's pinned source manifest</escape>` | Clipboard, paste, transforms; reviewed at pinned commit |
| `crates/engine/src/cmds_draw.rs` | `eb279b835e248f97a397b9611ce0b00b1672470d` | Create/delete/duplicate/convert and geometry/style edits |
| `crates/engine/src/cmds_group.rs` | `40fec49ebaf6cf45c8e173162e601e3febd68bdd` | Group/ungroup identity behavior |
| `crates/engine/src/cmds_scene.rs` | `f48b844e534097fc1a8c3b96c1bcd8221a43f030` | Scene duplication and ID remapping |
| `crates/engine/src/cmds_file.rs` | `5b834a0ce1347c5f612a71a0cae56dff6915f7b0` | Open/save/import/replacement |
| `crates/engine/src/transfer.rs` | `7bbee132c5d0b9ae96d4b96d499d550e2bd99bdf` | Cross-document library transfer and remapping |
| `crates/engine/src/cmds_motion.rs` | `<escape>not captured in this review's pinned source manifest</escape>` | Motion tween and property sampling; reviewed at pinned commit |
| `crates/engine/src/cmds_warp.rs` | `3d3d923b9ce1ef2d9243d1f79814f00cbba4ec36` | Native Asset Warp lifecycle |
| `crates/engine/src/editing.rs` | `<escape>not captured in this review's pinned source manifest</escape>` | Nested symbol/group/drawing edit context; reviewed at pinned commit |
| `crates/engine/src/render_service.rs` | `abf5c53b83a5d87ef6d6eb1811fb4b07dbabe676` | Render queue, cache, in-flight work, document picture clearing |
| `crates/engine/src/cmds_plugin.rs` | `48cea8e7a94d01f7e9733deac6a1ddb1ef7f8440` | Plugin command execution and nested host mutations |

The `<escape>not captured...</escape>` entries are a manifest limitation, not claims that the files were absent. Before implementation, regenerate a complete machine-readable source manifest from the exact host base commit and pin every audited file SHA.

---

## 3. Mutation boundary: what exists and what is missing

### 3.1 Central command execution exists

The engine module documents that user-visible actions are `CommandSpec` entries and that the editor shell, CLI, JSON-lines channel, and future MCP server call `Session::execute`. The command registry aggregates the major mutation namespaces: file, timeline, layer, parenting, bone, warp, motion, scene, frames, draw, group, modify, path, raw-point, plugin, macro, and related commands.

For a registered command marked `mutates`, `Session::execute` captures a snapshot, runs the command, settles rig/motion edits, compares document state, assigns a fresh `StateId`, increments `revision`, records command history, and restores the snapshot on command error.

This is a useful foundation. It is not yet a semantic mutation receipt:
- `CommandSpec::mutates` is a coarse boolean, not a typed dependency or identity-transition declaration.
- A command's display ID does not prove which dependencies it changed.
- A successful command can perform multiple internal mutations.
- Additional document mutations can happen during cut/floating settlement or gesture lifecycle.
- The document can be mutated directly without entering `Session::execute`.

### 3.2 Direct mutation bypass is a hard blocker

At the pinned source:
- `Session` exposes `pub doc: Document`.
- `Document` exposes public fields including `scenes`, `library`, `bitmaps`, `sounds`, `data`, and `next_id`.
- `Document::alloc_id` is public.
- `Document::timeline_mut` and nested mutable timeline/layer/keyframe helpers are part of the document API.

Consequently, callers with mutable access can mutate the persistent tree without the command transaction wrapper. A boundary implemented only inside `Session::execute` would be advisory rather than enforceable.

**Required resolution:** define an API ownership migration. The session's persistent document must become inaccessible for untracked external mutation. All mutable access must be scoped to a transaction/draft capability; public read-only access may remain. Lower-level crates should expose immutable views and transaction-scoped mutation APIs rather than unrestricted `&mut Document` or mutable collection access.

This is a compatibility and crate-boundary design decision, not a small instrumentation patch.

---

## 4. Existing mutation paths and required boundary mapping

| Mutation family | Verified source owners / paths | Readiness finding | Required receipt coverage |
|---|---|---|---|
| Create/draw/delete/duplicate | `cmds_draw.rs`, `cmds_path.rs`, `cmds_rawpoints.rs` | IDs allocated in individual command helpers; duplicates allocate new numeric IDs | Create/retire/replace target transitions; geometry/topology dependencies |
| Group/ungroup | `cmds_group.rs`, `editing.rs` | Group creates group layer and element IDs; ungroup can retain member IDs but allocates fresh IDs on clashes | Explicit mapping for group wrapper, every member, and collision remap |
| Transform/pivot/style | `cmds_draw.rs`, `cmds_modify.rs`, `cmds_shape.rs`, paint/stroke/filter commands | Many in-place edits preserve numeric element IDs | Preserve target identity; report transform, geometry, style, visibility/evaluation dependencies as applicable |
| Path geometry | `cmds_path.rs`, `cmds_rawpoints.rs`, `cmds_raw.rs`, `cmds_rawsub.rs` | Point and command topology can change without replacing the element | Separate source-geometry and source-topology effects |
| Timeline/keyframes | `cmds_timeline.rs`, `cmds_frames.rs`, `cmds_retime.rs`, `cmds_ease.rs`, `cmds_hints.rs` | Frame structure and sampled content are edited in-place; frame is not document revision | Timeline structure/sample dependencies; explicit frame stamp |
| Motion tween | `cmds_motion.rs`, `cmds_timeline.rs` | Auto-key temporarily substitutes evaluated target state and then writes properties/path data | Report final authored dependencies, not temporary intermediate state |
| Layer/hierarchy | `cmds_timeline.rs`, `cmds_layer.rs`, `parenting.rs` | Layer parenting and rig-parent transforms affect effective transforms | Hierarchy and effective-transform invalidation |
| Scene | `cmds_scene.rs` | Scene duplication re-IDs layers and elements; scene selection changes frame/context | Fresh target identities for copied scene; scene context stamped separately |
| Paste/cross-document transfer | `cmds_modify.rs`, `transfer.rs`, `cmds_frames.rs` | Clipboard survives document changes; carried library objects are remapped; same-document paste may restore original IDs | Destination target identities always fresh for new artwork; complete internal FDM subgraph remapping or reject |
| File open/import/recovery | `cmds_file.rs`, `cmds_recovery.rs`, `format.rs` | Open replaces the session document; import paths create many IDs and nested assets | New session instance on replacement; migration and import as atomic document transactions |
| Native Asset Warp | `cmds_warp.rs`, `doc::warp` | Native warp mesh/pins are host-owned and coupled to authored artwork | Report `NativeWarpState`; FDM adapter rejects or explicitly supports, never composes implicitly |
| Bone/armature | `cmds_bone.rs`, `parenting.rs` | Bone, weight, and rig state are separate evaluation inputs | Report `ArmatureState`; initial FDM Path2D adapter marks unsupported |
| Plugin/macro/automation | `cmds_plugin.rs`, `cmds_macro.rs`, `Session::execute_nested` | Nested commands run under an outer command and do not independently record history | One outer atomic receipt for a truly atomic plugin transaction; no nested bypass |
| Save/Save As | `cmds_file.rs`, `format.rs` | Save writes the document and changes path/dirty bookkeeping | Save path change is not a geometry mutation; persistent DocumentId must be serialized |
| Undo/redo/gesture | `Session::undo/redo/revert_gesture`, `History` | Persistent snapshots restore document/state; runtime revision is incremented separately | Restore identity ledger with artwork; advance runtime epoch monotonically |

This table is a source-owner map, not proof that every command implementation has already been exhaustively enumerated. The implementation-readiness gate must generate a complete command inventory and pair each mutating command with at least one boundary test.

---

## 5. Transaction, history, and side-effect findings

### 5.1 Main command rollback is a foundation, not a complete transaction protocol

`Session::execute` restores the snapshot on a returned command error. This supports the FDM-06.6 atomicity direction. However, the current implementation has additional state changes around the core call:
- `settle_cut` runs before the main mutation snapshot is captured.
- `heal_cut` can restore a prior document and increment `revision`.
- `settle_floating` can merge floating artwork after the command result has already been classified and can increment `revision` separately.
- `execute_nested` has its own rollback snapshot but intentionally leaves history/revision recording to the outer command.
- Gesture begin/end/revert and provisional-cut lifecycle modify how snapshots and command recordings are grouped.

The future boundary must classify these as either:
1. internal steps within the same transaction and receipt; or
2. distinct committed mutations with their own receipt and epoch.

No path may mutate persistent document state after a receipt has been published without opening another transaction.

### 5.2 History snapshots need an explicit extension design

At the pinned source, `history::Snapshot` stores the `Document`, `StateId`, scene, editing stack, frame, layer, selection, and floating state. `History` stores before-snapshots and reconstructs undo/redo by swapping snapshots.

There is no independent identity ledger or FDM extension envelope in the current snapshot model. Therefore:
- identity ledger and FDM authored state must be part of the same persistent snapshot boundary as the document;
- snapshot equality and document dirty-state semantics must include them;
- history byte estimation must account for their memory;
- undo/redo must restore ledger and payload atomically with artwork;
- runtime session instance and evaluation epoch must remain outside historical restoration.

A separate ledger stored only in session memory would break undo/redo and save/reopen correctness.

### 5.3 Gesture policy is compatible only with per-commit invalidation

AniCraft intentionally coalesces a drag/gesture into one undo step. That does not conflict with FDM-06.6: every visible committed geometry update must advance the evaluation epoch even if the history entry is coalesced. Gesture cancellation restores authored state but must advance the epoch again; it must never restore an earlier epoch.

The current `revision` does increment on each successful changed command inside a gesture and on gesture revert, but that counter is not yet checked for overflow and is not an explicit FDM epoch. A test must cover multiple updates, one undo entry, cancellation, and stale-result rejection.

---

## 6. Identity and allocation review

### 6.1 Current identity model

`ElementId` is a numeric `u64`. `Document` maintains one `next_id` counter shared by layers, elements, symbols, and other ID domains. `alloc_id` uses `saturating_add(1)`; at exhaustion it can repeatedly return the maximum value rather than fail before publication.

There is no persistent per-element incarnation token and no `DocumentId` in the inspected `Document` / `DocumentFile` schema.

### 6.2 Operation-specific semantics confirmed in source

- **Create/duplicate:** draw helpers and duplicate allocate fresh numeric element IDs.
- **Group:** creates a group layer and group element; members are moved into the group.
- **Ungroup:** member IDs are normally retained, but collisions caused by multiple copies can trigger new IDs. This is not a one-to-one stable mapping unless recorded explicitly.
- **Scene duplication:** `cmds_scene::reid` allocates new layer IDs and new element IDs, preserving the relationship of an element across that layer's keyframes and remapping motion tween targets.
- **Cross-document paste:** `transfer::bring` remaps library symbols/bitmaps/sounds; the artwork's element IDs are created by paste/import paths. This is not an FDM target graph remapper.
- **Same-document clipboard restoration:** can restore deleted library objects under their previous numeric IDs; the target incarnation contract must distinguish restored history/undo continuity from a genuinely new target lifetime.

### 6.3 Required identity transition decisions before implementation

FDM-06.8 must define:
1. Whether grouping preserves the identity of member targets while they become nested, and whether the group wrapper itself is independently addressable.
2. Whether ungrouping restores member incarnations or creates new incarnations when the group is dismantled.
3. Whether scene duplication creates a fresh incarnation for every duplicated element across every keyframe, while preserving the original scene's tokens.
4. Whether a same-document cut/paste that semantically restores the same object is a restoration of identity or a new lifetime. The decision must be consistent with delete/recreate and undo semantics.
5. How drawing-object conversion, symbol conversion, bitmap conversion, and merge operations express old-to-new identity mappings.
6. How cross-document paste remaps authored FDM references only when the complete referenced subgraph is present.

Do not implement the identity ledger until these cases are settled in a transition matrix with tests.

---

## 7. Session freshness, frame context, and concurrency

### 7.1 Existing revision is not the proposed epoch

The session field comment defines `revision` as incrementing on document changes, undo, and redo for UI invalidation. The source uses ordinary `+= 1` in command success, replacement, gesture rollback, cut healing, undo, and redo. This is not the checked, monotonic, session-instance-scoped epoch in FDM-06.6.

Required decisions:
- checked arithmetic and a typed overflow error;
- epoch increment at the atomic state-publication point;
- new session-instance identity on every document replacement/reconstruction;
- no epoch restoration from undo/redo snapshots;
- an explicit rule for all non-command state changes that affect evaluation.

### 7.2 Frame, scene, and nested edit context

The session stores `scene`, `frame`, and `editing` separately from the persistent document. The source explicitly describes playhead/view state as outside document history. The evaluator takes a frame, and nested symbol/group/drawing editing changes which timeline and coordinate space commands address.

The FDM stamp must therefore include:
- stable scene identity (not only the current scene index);
- frame;
- full nested edit-context identity/path and its coordinate-space transform;
- evaluated timeline sample inputs;
- target identity resolved in the correct timeline scope.

A frame change can invalidate FDM evaluation without changing document `StateId` or the current document revision. Scene reordering can also make an index-only scene reference unsafe.

### 7.3 Atomic publication and thread model are not yet established

The inspected `Session` uses `RefCell` caches and mutable session methods; it does not expose an evident single shared session mutex/serialized command lane in the inspected API. The renderer uses worker queues, cache locks, and a cloned rasterizer slot. Pending render batches can be replaced, while already-running work completes; `forget_pictures` prevents work started before a cache clear from being reinserted into that cache.

That render behavior is useful for raster output but does not establish FDM result acceptance. Before any asynchronous FDM work:
- identify the actual owner thread and lock/serialization model for each Session;
- specify how the document snapshot and epoch are read as one consistent pair;
- specify the exact atomic check-and-publish operation;
- prove no mutation can commit between acceptance check and publication;
- define cancellation/supersession and session disposal behavior.

**Synchronous FDM evaluation remains the only acceptable first integration mode.** Async work remains out of scope until a separate race-focused review passes.

---

## 8. Required mutation transaction model

The following model is recommended for the next specification, not implemented here:

1. **Session mutation gateway:** the only capability allowed to obtain mutable persistent document access.
2. **Draft/transaction scope:** commands and nested helpers mutate a draft or transaction-scoped document and accumulate semantic effects.
3. **Identity transition journal:** create/preserve/retire/replace/remap outcomes are staged with the same rollback boundary as the artwork.
4. **Validation:** validate document invariants, target uniqueness, FDM payload bounds/version, and all remapped references.
5. **Atomic commit:** publish artwork, identity ledger, extension payload, StateId, and incremented evaluation epoch as one logical state.
6. **Receipt delivery:** publish a durable receipt/epoch that downstream caches can observe or recover; event delivery failure cannot lose invalidation.
7. **History:** record the persistent document + identity ledger + extension envelope in one snapshot; never persist runtime epoch/session identity.
8. **Result acceptance:** accept only if request/session/document/epoch/context/target/dependency stamps still match under the same synchronization authority.

The exact Rust API, whether draft builder or journal, and whether the host uses a lock or serialized command executor remain open design decisions.

---

## 9. FDM-06.6 requirements conformance matrix

| Requirement | Current evidence | Status |
|---|---|---|
| One authoritative mutation boundary | `Session::execute` covers registered commands, but public mutable document access remains | **Blocker** |
| Explicit target transitions | Individual commands allocate/remap IDs; no central transition journal | **Blocker** |
| Atomic document + identity ledger | No identity ledger exists | **Not implemented** |
| Failed transaction rolls back all staged state | Main command snapshot rollback exists; side effects outside it need classification | **Partial** |
| Monotonic evaluation epoch | UI revision exists, unchecked and not a distinct FDM epoch | **Blocker** |
| Frame/scene/edit context stamped | Inputs exist in Session/evaluator; no FDM stamp | **Not implemented** |
| Immutable FDM snapshot | Evaluator reads Document; no dedicated FDM immutable snapshot adapter | **Not implemented** |
| Atomic stale-result acceptance | Render cache behavior is not an FDM result gate | **Not implemented** |
| Identity ledger in history and file | No DocumentId/incarnation ledger/envelope in schema or Snapshot | **Not implemented** |
| Sync-first evaluation | Feasible design choice; no host adapter yet | **Ready as policy** |
| Async after race proof | No race proof or acceptance gate | **Explicitly deferred** |
| Native Asset Warp independence | Native Warp has its own host commands and data model | **Preserve** |
| CCC-01.5 SC/FDM independence | No composition path introduced by this review | **Preserve** |

---

## 10. Proposed host conformance tests

These are proposed tests, not tests run in this review. They extend the FDM-06.6 FDM-MUT-001–032 plan and map concrete source ownership.

| ID | Scenario | Primary source owner | Required assertion |
|---|---|---|---|
| FDM-READY-001 | Direct mutable document access audit | `engine::Session`, `doc::Document` | No external mutation path bypasses transaction capability |
| FDM-READY-002 | Every registered mutating command classified | `commands()`, all `cmds_*` specs | Each mutating command has a dependency/transition classification |
| FDM-READY-003 | Failed command after ID allocation | `Session::execute`, `Document::alloc_id` | Document, allocator, ledger, epoch, and receipt all roll back |
| FDM-READY-004 | Failed nested plugin command | `cmds_plugin`, `execute_nested` | Outer transaction rolls back all nested effects and publishes no receipt |
| FDM-READY-005 | Provisional marquee cut followed by failure | `settle_cut`, `heal_cut`, `cmds_draw` | Cut healing is included in a defined transaction/receipt |
| FDM-READY-006 | Floating paste settles after command | `settle_floating`, `cmds_modify` | No post-receipt mutation; settlement has one atomic receipt |
| FDM-READY-007 | Gesture with many successful updates | `begin_gesture/execute/end_gesture` | One undo step, one epoch increment per committed update |
| FDM-READY-008 | Gesture cancellation | `revert_gesture` | Restores persistent state but advances epoch; old results rejected |
| FDM-READY-009 | Undo then redo | `history::Snapshot`, `Session::undo/redo` | Ledger/payload restore with document; epoch never decreases |
| FDM-READY-010 | History travel across several steps | `Session::travel_to` | Each restored state advances epoch and publishes a receipt |
| FDM-READY-011 | Replace document/open | `replace_document`, `cmds_file::open` | New session instance; prior requests cannot publish |
| FDM-READY-012 | Save path change / Save Copy | `cmds_file::save`, file schema | Save As retains DocumentId; explicit copy receives new ID |
| FDM-READY-013 | Element delete and numeric ID reuse | draw/delete + allocator | Reused ElementId receives a fresh IncarnationId |
| FDM-READY-014 | Duplicate and multi-frame identity | `cmds_draw::duplicate` | One new incarnation per logical duplicate target across frames |
| FDM-READY-015 | Group/ungroup without collision | `cmds_group` | Explicit wrapper/member transitions preserve declared semantics |
| FDM-READY-016 | Group/ungroup with duplicate ID collision | `cmds_group::ungroup` | Every remapped member receives a fresh incarnation and explicit mapping |
| FDM-READY-017 | Scene duplication | `cmds_scene::reid` | All copied targets receive fresh identities; motion target references remap |
| FDM-READY-018 | Cross-document paste with nested symbols | `cmds_modify`, `transfer::bring` | Complete FDM subgraph remaps atomically or is rejected |
| FDM-READY-019 | Import SVG/image and partial failure | `cmds_file::import_to_stage` | All new target IDs/incarnations commit atomically or none do |
| FDM-READY-020 | Frame/keyframe/tween change at fixed StateId | `cmds_timeline`, `cmds_motion` | Frame/sample stamp changes; prior result rejected |
| FDM-READY-021 | Rig-parent transform change | `parenting`, `cmds_timeline` | Effective-transform dependency invalidated |
| FDM-READY-022 | Native Asset Warp mutation | `cmds_warp` | NativeWarpState is reported; no implicit FDM composition |
| FDM-READY-023 | Bone/weight mutation | `cmds_bone` | ArmatureState invalidates or initial FDM adapter rejects |
| FDM-READY-024 | Epoch overflow | Session transaction gateway | Checked failure before publication; no wrap or saturation |
| FDM-READY-025 | Concurrent commit vs result acceptance | Session owner + FDM adapter | Check-and-publish is atomic; no stale output visible |
| FDM-READY-026 | Cache clear with in-flight FDM evaluation | FDM cache/host session | Old-session result cannot repopulate active cache |
| FDM-READY-027 | Unknown future mutation category | Mutation gateway | Global invalidation; selective cache reuse disabled |
| FDM-READY-028 | Extension payload version/size rejection | Document file schema + FDM payload | Unknown payload retained safely or evaluation disabled; no lossy rewrite |

---

## 11. Implementation blockers and exit criteria

Runtime implementation must not begin until the following are resolved in a reviewed design:

### Blocker A — Mutable API closure
- Inventory every public mutable Session/Document/Timeline/Layer/Keyframe API.
- Decide the crate-level ownership boundary.
- Define the migration path that removes or deprecates untracked mutable access.
- Provide compile-time or test-time evidence that command code cannot publish untracked mutations.

### Blocker B — Transaction scope and side effects
- Define whether cut healing, floating settlement, rig settlement, motion auto-key, import, paste, and plugin nested commands are part of the outer transaction or separate transactions.
- Define receipt count for gesture updates, macro steps, nested plugin commands, and post-command settlement.
- Prove rollback includes allocator state and all staged identity/payload state.

### Blocker C — Identity semantics
- Approve the group/ungroup, scene duplication, conversion, merge, same-document paste, cross-document paste, and undo restoration transition matrix.
- Define collision and allocator exhaustion behavior.
- Define persistent DocumentId creation, save, migration, and duplicate semantics.

### Blocker D — Persistent state and history
- Design DocumentFile extension envelope and legacy migration.
- Add identity ledger and authored FDM payload to the same snapshot/equality/dirty-state boundary as artwork.
- Update history memory accounting and undo/redo behavior.
- Define unknown payload preservation and resource limits.

### Blocker E — Session concurrency and freshness
- Identify actual session owner thread/serialization model and any cross-thread access.
- Define session instance and checked evaluation epoch types.
- Define atomic publication and atomic result acceptance.
- Keep async disabled until explicit race tests and a separate review.

### Blocker F — Source coverage and verification
- Generate a complete pinned manifest for every mutation module and mutable helper.
- Build a machine-checkable command/dependency coverage registry.
- Map each registered mutating command and every non-command mutation path to tests.
- Run the host test suite and the 28 readiness tests on the exact implementation head when implementation is later authorized.

**Exit condition:** all six blockers have an approved design, the mutation inventory has no unclassified paths, the transition matrix is complete, and the test ownership map is accepted. Passing CI on this documentation PR does not satisfy any host runtime exit criterion.

---

## 12. Decisions preserved and explicit non-authorization

This review does not change the following:
- FDM core remains host-independent.
- AniCraft owns document mutation, identity, session epoch, snapshot construction, cache invalidation, and result acceptance.
- Initial host support remains vector Path2D only.
- Native Asset Warp remains host-owned and is not silently composed with FDM.
- Smart Controller and Form Deform remain independent under CCC-01.5.
- Synchronous evaluation first; async evaluation remains deferred.
- No FDM algorithm, AniCraft runtime code, plugin ABI, or legacy adapter is changed.
- No implementation, merge, or release authorization is implied.

## 13. Recommended next milestone

**FDM-06.8 — Mutation Boundary and State Ownership Specification**

This should be a design-only milestone that closes Blockers A–E:
1. Define the enforceable mutable API boundary and transaction/draft ownership model.
2. Specify how cut/floating/rig/motion/plugin side effects join or split transactions.
3. Approve the complete target identity transition matrix.
4. Specify the persistent DocumentId/incarnation ledger/FDM envelope and history migration.
5. Specify session instance, checked epoch, synchronization authority, and atomic result acceptance.
6. Produce a concrete implementation sequence with crate owners and acceptance tests.

Only after FDM-06.8 is reviewed should a separate implementation proposal be prepared. No runtime implementation is authorized by this review.
