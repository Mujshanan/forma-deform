# FDM-06.8 — Mutation Boundary and State Ownership Specification

**Status:** Architecture specification; runtime implementation not authorized  
**Depends on:** FDM-06.4, FDM-06.5, FDM-06.6, FDM-06.7  
**Host baseline:** `Mujshanan/anicraft` commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**FDM-06.7 review:** PR #70, head `88f648e13d3c7629d773e2d5a8bea504242f2bc2`  
**Scope:** Define enforceable ownership of document mutation, transaction commit, persistent identity, extension state, history, session freshness, and FDM result acceptance.  
**Decision:** Establish a single session-owned mutation gateway and a persistent document-state bundle. Use owner-thread serialization and synchronous FDM evaluation initially. All runtime changes remain separately gated.

---

## 1. Executive architecture decision

FDM-06.7 showed that `Session::execute` is a useful transaction entry point but not an enforceable boundary while public mutable document access and post-command mutation paths remain.

FDM-06.8 specifies the following architecture:

1. **AniCraft Session owns the only mutation gateway.** All persistent document mutation, including nested command helpers, plugins, import, gestures, undo/redo, and replacement, must pass through one session-owned transaction protocol.
2. **Mutable document access is transaction-scoped.** External callers receive immutable document views. A transaction receives exclusive mutable access to a draft state; no unrestricted mutable reference escapes.
3. **Persistent state is one logical bundle.** Artwork, DocumentId, target-incarnation ledger, and versioned extension payload are committed, snapshotted, serialized, and restored together.
4. **Identity transitions are explicit.** Every transaction reports create, preserve, retire, replace, or remap transitions. Numeric ID equality is never proof of target continuity.
5. **Runtime freshness is not history state.** A session-instance ID, checked monotonic document evaluation epoch, and checked evaluation-context generation remain outside undo/redo snapshots.
6. **Commit is atomic at the session owner.** Validation, identity ledger, extension payload, document, StateId, epoch, and receipt publication form one logical commit. No fallible mutation is permitted after publication.
7. **FDM initially evaluates synchronously.** Render/evaluation workers receive immutable snapshots. Any future asynchronous result must return to the session owner and pass a single serialized acceptance-and-publication check.
8. **Unknown mutation categories invalidate conservatively.** Fine-grained cache reuse is deferred until the mutation coverage registry proves complete.

This specification deliberately avoids selecting a particular Rust draft/journal implementation or adding a shared mutex around Session. Those choices must respect the host's actual ownership and threading model and be confirmed during implementation planning.

---

## 2. Ownership model

### 2.1 Ownership matrix

| State / responsibility | Owner | FDM responsibility |
|---|---|---|
| Document tree, scenes, layers, elements, timelines | AniCraft document/session | Read immutable snapshot |
| DocumentId | AniCraft | Treat as opaque scope identity |
| ElementId allocation | AniCraft | Never allocate or infer |
| Incarnation ledger | AniCraft | Declare target references and validate exact identity |
| Mutation gateway and transaction lifecycle | AniCraft Session | Invoke only through host-approved commands |
| StateId and document dirty state | AniCraft | No authority |
| Session instance and runtime epoch | AniCraft Session | Include in snapshot/result stamp |
| Frame, scene, edit context and effective transforms | AniCraft Session | Consume stamped evaluated inputs |
| FDM authored model/payload | FDM schema, stored by AniCraft | Define schema, migrations, validation, limits |
| FDM topology/deformation calculations | FDM core | Own algorithm and deterministic output |
| Dependency declaration/fingerprint contract | FDM defines required inputs; host computes authoritative stamps | Declare what is consumed |
| Cache invalidation and result acceptance | AniCraft | Return exact dependency/output contract |
| Rendering, apply, bake, undo/redo | AniCraft | Provide validated output and provenance |
| Native Asset Warp | AniCraft | No implicit composition |
| Smart Controller | Separate existing host feature | Remains independent under CCC-01.5 |

### 2.2 Session becomes the mutation authority

The following API policy is normative:

- `Session` must not expose a public mutable `Document` field.
- Public consumers may obtain an immutable document view or immutable snapshot.
- `Document` may retain public read-only data where compatible, but mutation-capable fields and helpers must not be reachable outside the transaction capability.
- `Document::alloc_id`, `timeline_mut`, mutable collection accessors, and equivalent APIs must become crate-private or require a transaction-scoped capability.
- Command implementations may mutate only the transaction draft.
- Nested commands must receive the same transaction capability, not reopen the session or publish independent state.
- Plugins/macros/automation must be restricted to the same gateway. No plugin receives unrestricted mutable Session/Document access.
- File open, import, recovery, undo, redo, and gesture restoration are explicit transaction kinds, not privileged bypasses.
- Test-only direct construction/mutation remains possible behind test helpers, but production APIs must not provide a bypass.

A compile-time ownership boundary is preferred over runtime convention. If a crate cannot be migrated immediately, the interim API must be private to the engine crate and covered by an explicit deprecation/migration plan; it must not be described as fully enforced.

### 2.3 Read and write capabilities

Conceptual API shape (illustrative, not an approved Rust signature):

```rust
struct Session {
    persistent: PersistentDocumentState,
    runtime: SessionRuntimeState,
    history: History,
}

struct PersistentDocumentState {
    document: Document,
    extensions: HostExtensionEnvelope,
}

struct MutationTxn<'a> {
    draft: PersistentDocumentState,
    effects: MutationEffects,
    // exclusive borrow/capability owned by Session
}

struct MutationReceipt {
    session_instance: SessionInstanceId,
    document_id: DocumentId,
    epoch: EvaluationEpoch,
    state_id: StateId,
    effects: CommittedMutationEffects,
}
```

No code outside the transaction can obtain `&mut Document` or mutate extension state. The exact implementation may use copy-on-write draft state, a journal, or another transactional mechanism. It must provide the observable semantics in this specification.

---

## 3. Persistent state and extension envelope

### 3.1 One authoritative persistent bundle

The host must treat the following as one logical persistent document state:

- AniCraft `Document` artwork and library data;
- `DocumentId`;
- target identity ledger;
- versioned host extension envelope;
- FDM authored payload and any explicitly supported authored bake provenance.

The bundle is the unit of transaction commit, history snapshot, dirty-state comparison, file serialization, duplication, and recovery.

The runtime session instance, evaluation epoch, context generation, pending requests, caches, and worker state are not serialized as authored document state.

### 3.2 Envelope contract

Logical schema (not final Rust/serde spelling):

```text
HostExtensionEnvelope
  envelope_version
  document_id: OpaqueId128
  target_identities:
    ElementId -> IncarnationId(OpaqueId128)
  payloads:
    ExtensionKey -> VersionedOpaquePayload
```

Rules:

1. AniCraft owns the envelope container, DocumentId, target ledger, file migration, duplicate/import remapping, transaction atomicity, and resource limits.
2. FDM owns the versioned FDM payload schema, FDM migrations, authored topology/deformation semantics, and FDM-specific validation.
3. Payloads are keyed by stable extension identifiers and carry independent schema versions.
4. Payload values are treated as opaque by the host except for envelope validation, size/depth limits, and declared reference-remapping hooks.
5. Unknown extension keys and unsupported payload versions must survive load/save round trips byte-for-byte or through an equivalent lossless representation. If the host cannot preserve them, it must refuse destructive save/upgrade rather than silently drop them.
6. An unsupported FDM payload disables FDM editing/evaluation with a clear compatibility diagnostic. It must not be partially interpreted.
7. All FDM target references must be document-scoped and validated against DocumentId, ElementId, and IncarnationId.
8. Resource limits must cover total envelope bytes, individual payload bytes, nesting depth, number of targets, number of payload records, and remapping work. Limits must be explicit constants and tested.
9. No sidecar file is authoritative for authored FDM state.
10. The envelope is not a generic executable plugin system. Payloads are data only.

### 3.3 Identity ledger invariants

For every live, eligible stage element:

- Exactly one IncarnationId is associated with its ElementId.
- IncarnationIds are unique within a document.
- A live ElementId cannot have two incarnation tokens.
- A retired incarnation is never silently rebound to a new logical target.
- Every persisted FDM target reference resolves to the exact ledger entry or produces a stale/dangling diagnostic.
- IDs in unsupported nested scopes may be represented in the host ledger, but the initial FDM adapter must not claim them as supported targets.
- The host validates that ledger entries correspond to live targets according to the chosen scope policy; orphaned entries are either retired through an explicit transition or rejected during validation.

For v1 integration, the supported FDM target scope remains vector Path2D elements in the supported stage evaluation context. Bitmap, text, symbol-instance, nested group deformation, nonlinear parenting, and multi-deformer composition are not implied by the identity ledger.

---

## 4. Identity lifecycle and transition matrix

### 4.1 Identity tuple

`TargetRef = { document_id, element_id, incarnation_id }`

- DocumentId: persistent opaque document identity.
- ElementId: host-local lookup key.
- IncarnationId: identity of one logical lifetime of that element ID.
- Geometry/topology/transform/hierarchy revisions: dependency freshness, not identity.
- StateId: host history state identifier, not identity or freshness.
- SessionInstanceId/EvaluationEpoch: runtime freshness, not persistent identity.

### 4.2 Normative transition matrix

| Operation | ElementId behavior | Incarnation behavior | FDM reference behavior |
|---|---|---|---|
| New target creation | Fresh host ID | Fresh token | New target may be authored after validation |
| In-place path/transform/style edit | Preserve | Preserve | Existing reference remains valid; dependency stamp changes |
| Rename/selection/view-only change | Preserve | Preserve | No geometry identity change |
| Delete | Remove live ID | Retire token | References become dangling; never auto-rebind |
| Delete then create, even if numeric ID reused | May reuse only under host allocation policy | Fresh token | Old references remain stale |
| Undo deletion | Restore snapshot ID | Restore exact prior token | Historical references resolve again in restored state |
| Redo deletion | Remove again | Retire again | References become dangling again |
| Duplicate artwork | Fresh ID for every logical duplicate | Fresh token for every duplicate | Internal references remap only within copied subgraph |
| Group members | Member IDs preserved when logically moved | Member tokens preserved | Group wrapper gets fresh ID/token; group wrapper itself initially unsupported as FDM target |
| Ungroup without collision | Member IDs preserved | Member tokens preserved | References to members remain valid |
| Ungroup with ID collision/remap | Remapped members receive fresh IDs | Fresh tokens for remapped members | Explicit old-to-new map; old refs do not silently resolve |
| Scene duplication | Fresh IDs for all duplicated layers/elements | Fresh tokens for all duplicated targets | Internal motion/timeline references remap; original scene untouched |
| In-place conversion preserving same logical Path2D target | Preserve ID | Preserve token only if conversion preserves target semantics | Existing ref remains valid; dependency stamp changes |
| Conversion away from supported Path2D | ID may remain host-side | Retire FDM target incarnation | FDM ref becomes unsupported/stale |
| Conversion into Path2D from unsupported type | Host ID as assigned | Fresh token | New FDM target |
| Merge multiple elements | New/host-selected ID | Fresh token | Retire all source targets; record many-to-one provenance |
| Split one element into multiple | New IDs | Fresh tokens for all outputs | Retire source target; record one-to-many provenance |
| Same-document move within one atomic transaction | Preserve ID | Preserve token | Same target, new hierarchy/context dependency |
| Committed delete followed by later clipboard paste | Fresh ID | Fresh token | New lifetime; clipboard origin is not identity proof |
| Cross-document paste/import | Fresh destination IDs | Fresh destination tokens | Remap only complete supported FDM subgraphs; otherwise reject/quarantine payload |
| Save As / rename / move file | Preserve | Preserve | References remain valid |
| Save Copy / Duplicate Document | New DocumentId | Generate fresh tokens in copied document | Rewrite internal refs to copied targets; external refs remain scoped to original |
| Document reopen | Persistent IDs restored | Persistent tokens restored | New SessionInstanceId invalidates all runtime requests |

**Important:** a move and a delete-plus-paste are different operations. The former can preserve identity only when performed atomically with an explicit preserve transition. The latter creates a new lifetime even if clipboard metadata or numeric IDs match.

### 4.3 Grouping, scenes, conversion and remapping

- Grouping does not create a new identity for each moved member. The group container is a separate host object with its own fresh identity.
- The initial FDM adapter does not deform the group wrapper or nested members. It must reject unsupported nested target contexts rather than flatten them.
- Ungroup preserves member identities unless the host must remap a collision. Every remap is explicit.
- Scene duplication creates a complete old-to-new map for every duplicated layer/element and all timeline/motion references. FDM payload references are rewritten only if the full referenced subgraph is present.
- Merge/split/conversion operations must declare provenance and identity transitions. The host must not pick an arbitrary source identity to preserve merely because it is convenient.
- Cross-document import with an incomplete or unsupported FDM reference graph must reject the FDM payload or preserve it as inactive opaque data with a diagnostic. It must never create dangling cross-document references that appear valid.

### 4.4 Allocator requirements

- Allocation must fail before commit on exhaustion or collision.
- Saturating allocation that repeatedly returns the maximum ID is not acceptable.
- The host owns ID allocation and uniqueness across all document ID domains.
- Allocation performed in a failed transaction must not leak partially committed identity or ledger entries.
- Reuse of a numeric ElementId always receives a new IncarnationId unless the exact historical state is being restored by undo/redo.
- Incarnation tokens are generated by a host-owned cryptographically strong or collision-resistant opaque-ID facility. Uniqueness is validated at commit; generation alone is not treated as proof.

---

## 5. Transaction protocol

### 5.1 Transaction kinds

All persistent mutations must use the same commit protocol, with a typed origin:

- User command;
- nested command / plugin / macro / automation;
- gesture update or gesture cancellation;
- import/paste/transfer;
- document replacement/open/recovery;
- undo/redo/history travel;
- explicit FDM apply/bake command;
- migration or document duplication.

The origin is diagnostic metadata; it does not grant bypass privileges.

### 5.2 Required phases

1. **Begin:** Session owner creates a transaction from the current persistent state and captures the before-state required for history/rollback.
2. **Mutate draft:** command and nested helpers operate only on transaction-scoped mutable state.
3. **Collect effects:** helpers record identity transitions, dependency categories, target scope changes, and mutation provenance. Effects are typed, not inferred from command labels.
4. **Settle internal work:** cut/floating settlement, rig/motion settlement, import normalization, and other command post-processing occur before validation/commit.
5. **Validate:** validate document invariants, allocator state, ledger uniqueness/completeness, extension envelope limits/schema, all target references, and all required remaps.
6. **Determine no-op:** compare persistent state and normalized effects. A true no-op does not create a history entry or advance evaluation epoch. A command that changed then restored its draft to the identical committed state is a no-op.
7. **Prepare commit:** compute checked next StateId/history metadata and checked next EvaluationEpoch. Any overflow or validation error aborts before publication.
8. **Atomic publish:** replace the session's persistent bundle, update StateId/dirty state, advance runtime epoch, and append history/receipt metadata as one non-interleavable owner-thread operation.
9. **Publish receipt:** make the committed epoch/effects visible to downstream consumers. No further persistent mutation is allowed after the receipt. Notifications are hints; the session's current epoch and committed state are authoritative.
10. **Return:** return command result only after commit and receipt publication are complete.

### 5.3 Failure semantics

- Any error before atomic publish leaves document, identity ledger, extension payload, StateId, history cursor, epoch, and committed receipt stream unchanged.
- Failed transactions do not publish partial target transitions.
- Failed transactions do not consume an externally visible ElementId/IncarnationId. Internal random tokens generated during an aborted draft may be discarded.
- Any failed validation, ID exhaustion, epoch overflow, unsupported extension migration, or incomplete required remap aborts the transaction.
- The transaction layer must not perform fallible external side effects before commit. File writes, notifications, and other external actions require explicit staging/compensation semantics.
- Once the persistent bundle and epoch are published, later notification-delivery failure cannot roll the commit back or hide the new epoch. Consumers must be able to recover by reading the authoritative current epoch/receipt sequence.
- A command must never return success for a mutation that was not committed.

### 5.4 Cut, floating paste, rig and motion settlement

The current host's `settle_cut`, `heal_cut`, `settle_floating`, gesture, rig and motion paths must be migrated under these rules:

- Provisional UI overlays may exist outside persistent state while they remain previews and cannot be consumed by FDM.
- When a cut/floating operation changes persistent artwork, the settlement is part of the command transaction or is a separate explicit transaction with its own receipt.
- No post-command helper may silently mutate persistent document state after the main receipt.
- A failed command must roll back any staged cut/floating/rig/motion changes and identity effects together.
- Motion auto-key temporary evaluation state must not leak into the committed document. The transaction reports the final authored path/property changes only.
- Gesture updates may share one undo group, but each distinct committed visible update advances the evaluation epoch. Gesture cancellation restores the persistent snapshot and advances the epoch again.

### 5.5 Nested commands and plugins

- Nested command execution reuses the outer transaction and effect journal.
- A nested command cannot publish an independent receipt or history entry.
- The outermost transaction commits once after all nested operations and post-processing succeed.
- Any nested error aborts the outer transaction unless the command explicitly defines and tests a recoverable sub-operation with no partial persistent effects.
- Plugins receive a constrained command API, not mutable document references.
- Macro sequences that intentionally commit each step are separate transactions and receipts; macros declared atomic use one outer transaction. This behavior must be explicit in the macro contract.

---

## 6. History, dirty state, save, and migration

### 6.1 Snapshot model

The history snapshot must include the full persistent bundle:

- Document;
- DocumentId;
- target-incarnation ledger;
- extension envelope and FDM payload;
- any authored bake provenance included in the supported schema.

It may also contain existing undo-restored editor context where the host already treats it as historical state (scene, editing stack, frame, layer, selection, floating state). The precise UI-context policy remains host-owned.

The snapshot must not contain:
- SessionInstanceId;
- EvaluationEpoch;
- evaluation-context generation;
- worker request IDs;
- caches or pending jobs.

History equality, dirty-state detection, byte accounting, snapshot cloning, and history limits must include extension/ledger memory.

### 6.2 Undo/redo semantics

- Undo/redo restores the complete persistent bundle, including exact historical incarnation tokens and FDM payload.
- Runtime EvaluationEpoch always advances on each successful undo/redo/history travel that changes the persistent or evaluation context state.
- Epoch is never restored from a snapshot and never moves backwards.
- SessionInstanceId remains the same for ordinary undo/redo.
- Undo to an earlier StateId is permitted; StateId is not a freshness token.
- Failed undo/redo changes neither persistent state nor epoch.
- If a history restore cannot validate the restored envelope/ledger, it must fail atomically and preserve the current state.

### 6.3 Save and duplicate semantics

| Operation | DocumentId | Incarnation ledger | Epoch |
|---|---|---|---|
| Save current file | Preserve | Preserve | No change |
| Save As / rename / move | Preserve | Preserve | No change |
| Save Copy / Duplicate Document | New | Fresh tokens for copied targets; rewrite internal references | New session if opened as separate document |
| Reopen same file | Restore persistent ID and ledger | Restore | New SessionInstanceId and fresh runtime epoch |
| Legacy file migration | Generate once and persist | Generate tokens for existing eligible targets | New session instance; migration marks document dirty until saved |
| Import into current document | Preserve destination DocumentId | Fresh IDs/tokens for imported targets | Advance on commit |
| Replace current document with another file | Load incoming DocumentId | Load incoming ledger/migrate | New SessionInstanceId; epoch reset only within the new session identity |

Save Copy must operate on a cloned persistent bundle and must not mutate the source document's identity or dirty state. Internal target references are rewritten to the copied document's new IDs/tokens. No cross-document reference is retained as if it belonged to the copy.

### 6.4 Legacy and unknown payload behavior

- Legacy files without an envelope are migrated in memory by generating DocumentId and ledger entries deterministically only within that one migration execution; the generated values are then persisted on the next successful save.
- The migration result must be marked dirty and migration status surfaced to the user.
- Reopening a migrated file restores exactly the persisted identity tokens.
- If a file contains a future envelope version or unknown payload, preserve the raw representation losslessly where possible.
- If safe preservation is impossible, open read-only or refuse save; never silently strip extension data.
- FDM migration is a versioned FDM-owned operation executed inside a host transaction, with a backup/rollback path.

---

## 7. Runtime freshness and evaluation stamps

### 7.1 Separate identifiers

| Identifier | Persistence | Purpose |
|---|---|---|
| DocumentId | Persistent | Scope identity |
| ElementId | Persistent within document | Host lookup |
| IncarnationId | Persistent within document | Target lifetime |
| StateId | History state | Dirty/history identity |
| SessionInstanceId | Runtime only | Distinguish open/replaced session lifetimes |
| EvaluationEpoch | Runtime only, monotonic | Invalidate after committed evaluation-affecting mutations |
| EvaluationContextGeneration | Runtime only, monotonic | Invalidate frame/scene/edit-context changes |
| RequestId | Runtime only | Distinguish requests and supersession |
| Dependency fingerprints | Derived/stamped | Verify consumed geometry/topology/transforms/timeline inputs |

### 7.2 Epoch rules

- EvaluationEpoch is a checked monotonic integer owned by Session.
- It advances once for every committed persistent mutation that can affect FDM output, including undo/redo and gesture cancellation that restores a prior document.
- It also advances conservatively for unknown mutation categories.
- A no-op, selection-only change, panel visibility, viewport pan/zoom, or unrelated UI preference does not advance it unless that state is explicitly introduced as an FDM evaluation input.
- Overflow is a transaction failure before commit. Wrap, saturation, and reuse are forbidden.
- A document replacement creates a new SessionInstanceId. Epoch values may restart only because the session identity changes; stamps always include both.
- The global epoch is the final stale-result fence even if fine-grained dependency revisions are introduced later.

### 7.3 Context generation

Frame, scene, nested edit context, and coordinate-space changes can affect evaluation without changing persistent Document StateId. They must advance a separate checked EvaluationContextGeneration or otherwise produce an equally strong non-repeating context stamp.

At minimum the context stamp contains:
- stable scene identity (not only a mutable array index);
- frame/playhead value and the actual sampled timeline frame;
- full nested edit-context path/identity;
- coordinate-space and effective transform context;
- relevant evaluation mode/visibility context;
- session instance.

A selection change does not alter evaluation if the request already contains an explicit TargetRef. If the host uses selection as an implicit target input, the selection generation must be included or selection changes must advance context generation. The preferred FDM API requires explicit TargetRef.

### 7.4 Immutable snapshot contract

An FDM evaluation request is built from one immutable snapshot captured by Session. The snapshot must include or identify:

- DocumentId and SessionInstanceId;
- EvaluationEpoch and EvaluationContextGeneration;
- StateId for diagnostics only;
- stable scene/edit-context/frame sample;
- exact TargetRef and target capability;
- immutable source Path2D geometry and coordinate-space definition;
- source geometry/topology fingerprints;
- effective transform/hierarchy/timeline sample fingerprints;
- FDM authored payload and schema version;
- binding/topology/deformation/refinement policy versions;
- explicit unsupported markers for native Asset Warp, armature, nested group/symbol context, or other unsupported dependencies.

The snapshot must be self-consistent: all fields are derived from the same committed persistent state and context. FDM core code receives no live Session or mutable Document reference.

### 7.5 Dependency ownership

- FDM declares the semantic dependencies it consumes.
- AniCraft computes authoritative host-side revision stamps/fingerprints from its own data.
- The initial integration uses a conservative document-wide EvaluationEpoch plus explicit context generation and full target/dependency fingerprints.
- Fine-grained geometry/topology/transform/hierarchy/timeline revisions are optimization only after complete mutation coverage and conformance tests.
- Missing or unknown dependency information forces global invalidation or rejects evaluation; it never defaults to a cache hit.

---

## 8. Result acceptance and threading model

### 8.1 Initial synchronous mode

The first host integration performs FDM evaluation synchronously on the Session owner thread or in a controlled call where no mutation can interleave between snapshot capture and publication.

No asynchronous FDM evaluation, speculative cache publication, or background mutation is part of v1.

### 8.2 Future asynchronous mode

If asynchronous evaluation is later approved:

1. Session owner captures immutable snapshot and request stamp.
2. Worker receives only immutable snapshot and cancellation token.
3. Worker returns an immutable result plus the exact request stamp.
4. Result is posted back to the Session owner queue.
5. Session owner validates and publishes result in one serialized, non-yielding operation.

No worker may read live Document, mutate Session, mutate caches as authoritative state, or publish a result directly to rendering.

### 8.3 Acceptance predicate

A result is accepted only if all are true at the acceptance point:

- request ID is still active and not superseded/cancelled;
- SessionInstanceId matches;
- DocumentId matches;
- EvaluationEpoch matches;
- EvaluationContextGeneration matches;
- scene/frame/edit context matches;
- exact TargetRef still resolves to the same live incarnation;
- source geometry/topology and all declared dependency stamps match;
- FDM payload/schema/policy versions match;
- output is structurally valid and all coordinates/metadata are finite and within configured limits;
- session is not closing/replacing;
- target capability remains supported and no unsupported dependency was introduced.

Check and publish must be one owner-thread operation with no await, callback, or yield between validation and making the output visible. A render worker may consume only already-accepted immutable evaluated geometry.

### 8.4 Cache contract

- Cache keys include SessionInstanceId, DocumentId, EvaluationEpoch, EvaluationContextGeneration, TargetRef, dependency fingerprints, FDM payload/schema version, and policy versions.
- Cache invalidation is conservative: any unknown mutation makes prior entries unreachable.
- Cache eviction is not correctness. Acceptance validation is correctness.
- Old in-flight work may finish, but its result is discarded if the acceptance predicate fails.
- Cache clearing must prevent old work from reinserting entries into the active cache namespace.
- Native Asset Warp output and FDM output remain separate products; no shared cache key implies composition.

---

## 9. Mutation effects and coverage registry

### 9.1 Typed effect vocabulary

Every transaction records zero or more typed effects:

- `TargetCreated`
- `TargetPreserved`
- `TargetRetired`
- `TargetReplaced`
- `TargetRemapped`
- `GeometryChanged`
- `TopologyChanged`
- `TransformChanged`
- `HierarchyChanged`
- `TimelineStructureChanged`
- `TimelineSampleChanged`
- `SceneChanged`
- `NativeWarpStateChanged`
- `ArmatureStateChanged`
- `FdmPayloadChanged`
- `DocumentReplaced`
- `UnknownEvaluationDependencyChanged`

Effects are collected from mutation operations and normalized before commit. They are not trusted as the only source of truth: host validation cross-checks target transitions against the resulting document and ledger. Unknown effects force global invalidation and disable selective reuse.

### 9.2 Machine-checkable coverage registry

Before runtime implementation is considered complete, the host must maintain a registry for every command and non-command mutation entry point. Each record contains:

- stable command/helper identifier;
- source module and owner;
- whether it mutates persistent state;
- transaction kind;
- allowed mutation capability;
- dependency/effect categories;
- target transition policy;
- rollback behavior;
- history behavior;
- conformance test IDs.

CI must fail if a registered mutating command has no classification or test mapping. New command registration must require a classification. This prevents a future command from silently bypassing the FDM invalidation contract.

### 9.3 Mandatory inventory groups

The registry must include at least:
- create/draw/delete/duplicate;
- path and raw-point editing;
- transforms, pivots, styles, filters, visibility;
- group/ungroup and symbol conversion;
- scene creation/duplication/reordering;
- layer creation/deletion/reparenting and rig parenting;
- timeline/keyframe/frame transfer/retime/easing/hints;
- motion tween and auto-key;
- bone/armature/weight operations;
- native Asset Warp creation/edit/delete;
- clipboard/cut/paste and cross-document transfer;
- file open/save/save-as/save-copy/import/recovery;
- plugin, macro, automation and nested commands;
- gesture begin/update/end/cancel and floating/cut settlement;
- undo/redo/history travel;
- FDM authored-state edits and explicit apply/bake.

---

## 10. Implementation sequence and ownership

This section defines the required sequence; it does not authorize implementation.

| Stage | Work | Primary owner | Exit evidence |
|---|---|---|---|
| S0 | Re-pin current AniCraft base and generate complete source/command manifest | Host maintainers | Exact commit + blob SHAs + command registry |
| S1 | Encapsulate mutable Session/Document APIs | AniCraft doc + engine | Compile-time proof no external mutation bypass |
| S2 | Introduce transaction-scoped draft/capability and typed effect journal | AniCraft engine | Rollback, no-op, nested-command tests |
| S3 | Integrate cut/floating/rig/motion/gesture side effects | AniCraft engine | No post-receipt persistent mutation |
| S4 | Define identity allocator, DocumentId, IncarnationId and transition journal | AniCraft doc + engine | Identity transition suite |
| S5 | Add persistent envelope and identity ledger to DocumentFile | AniCraft doc; FDM schema owner | Migration, unknown-payload, resource-limit tests |
| S6 | Include persistent bundle in history/dirty-state/Save Copy | AniCraft engine + doc | Undo/redo/Save As/Save Copy tests |
| S7 | Add SessionInstanceId, checked EvaluationEpoch and context generation | AniCraft engine | Overflow and monotonicity tests |
| S8 | Build immutable Path2D snapshot adapter and dependency stamp | AniCraft adapter + FDM core | Snapshot consistency and coordinate-space tests |
| S9 | Integrate synchronous evaluation and acceptance | AniCraft engine + FDM core | Stale-result and deterministic output tests |
| S10 | Evaluate async design separately | Host architecture review | Race tests and separate authorization |

No stage may skip its exit evidence. S10 is not part of the initial implementation.

---

## 11. Conformance test plan

The following test IDs extend FDM-06.7's FDM-READY-001–028 plan. They are required tests, not tests run by this specification.

| ID | Requirement | Expected result |
|---|---|---|
| FDM-OWN-001 | External mutable access audit | No public API can mutate committed Document outside gateway |
| FDM-OWN-002 | Command registry completeness | Every mutating command has effect and test classification |
| FDM-OWN-003 | Transaction rollback after ID allocation | Document, ledger, envelope, history and epoch unchanged |
| FDM-OWN-004 | Nested plugin failure | Entire atomic outer transaction rolls back |
| FDM-OWN-005 | No-op transaction | No StateId/history/epoch change |
| FDM-OWN-006 | Post-processing settlement | Cut/floating/rig/motion effects commit before one receipt |
| FDM-OWN-007 | Gesture updates | Every committed update advances epoch; undo history may coalesce |
| FDM-OWN-008 | Gesture cancellation | Restores persistent snapshot and advances epoch |
| FDM-OWN-009 | Undo/redo | Persistent bundle restored; epoch never decreases |
| FDM-OWN-010 | History travel | Every successful changed restore advances epoch |
| FDM-OWN-011 | Document replacement | New SessionInstanceId invalidates all old requests |
| FDM-OWN-012 | Save As | DocumentId and incarnation ledger preserved |
| FDM-OWN-013 | Save Copy | New DocumentId and fresh copied target incarnations; source unchanged |
| FDM-OWN-014 | Legacy migration | IDs/tokens generated once, dirty migration persisted on save |
| FDM-OWN-015 | Unknown payload version | Opaque payload retained or destructive save refused |
| FDM-OWN-016 | Envelope limits | Oversized/deep/over-count payload rejected before commit |
| FDM-OWN-017 | Delete and numeric ID reuse | New incarnation; old TargetRef remains stale |
| FDM-OWN-018 | Undo deletion | Exact prior incarnation restored |
| FDM-OWN-019 | Group/ungroup | Member tokens preserved unless explicit collision remap |
| FDM-OWN-020 | Scene duplication | Every copied target gets new ID/token; references remap |
| FDM-OWN-021 | Merge/split/conversion | Explicit one-to-many/many-to-one transitions; no arbitrary identity inheritance |
| FDM-OWN-022 | Same-document move vs delete/paste | Move preserves; committed delete/paste creates new lifetime |
| FDM-OWN-023 | Cross-document partial subgraph | Reject or quarantine FDM payload; no dangling cross-doc refs |
| FDM-OWN-024 | Allocator exhaustion/collision | Checked transaction failure before publication |
| FDM-OWN-025 | Epoch overflow | Checked transaction failure; no wrap/saturation |
| FDM-OWN-026 | Context change at fixed StateId | Context generation changes and old result rejected |
| FDM-OWN-027 | Snapshot consistency | Every snapshot field derives from one committed state/context |
| FDM-OWN-028 | Acceptance vs mutation | Owner-thread serialization prevents check/publish race |
| FDM-OWN-029 | Old worker result after replacement | Result rejected by SessionInstanceId |
| FDM-OWN-030 | Unknown mutation effect | Global invalidation; selective cache reuse disabled |
| FDM-OWN-031 | Cache clear with in-flight result | Old result cannot repopulate active namespace |
| FDM-OWN-032 | Native Asset Warp mutation | FDM rejects unsupported composition; native path unchanged |
| FDM-OWN-033 | Armature/bone mutation | FDM marks unsupported or invalidates declared dependency |
| FDM-OWN-034 | FDM payload edit | Payload included in history, dirty state and epoch |
| FDM-OWN-035 | Failed file save | Dirty state remains; persistent identity unchanged |
| FDM-OWN-036 | Atomic FDM apply/bake | Artwork and authored provenance commit/undo as one host transaction |

---

## 12. Explicit non-goals and preserved decisions

- No AniCraft runtime code or FDM runtime code is changed by this specification.
- No new deformation algorithm, topology behavior, interpolation, or output format is defined.
- No bitmap, text, symbol, group-wrapper, nested group, or nonlinear parenting deformation support is implied.
- No native Asset Warp + FDM composition.
- No Smart Controller + FDM composition; CCC-01.5 remains unchanged.
- No multi-deformer composition.
- No async FDM evaluation in initial integration.
- No frame-range bake.
- No plugin SDK or legacy adapter retirement.
- No automatic merge, release, or implementation authorization.

Native Asset Warp remains host-owned. Smart Controller evaluation and Form Deform evaluation remain independent. FDM core remains host-independent and owns only its authored/calculation semantics and output contract.

---

## 13. Decisions still requiring host-specific confirmation

The following are intentionally not guessed by this specification and must be confirmed before implementation:

1. Exact Session owner-thread and command-dispatch model across desktop, CLI, tests, and any server interfaces.
2. Whether transaction draft uses copy-on-write Document cloning, a journal, or a hybrid.
3. Exact Rust visibility/API migration plan for public document fields and mutable helpers.
4. Exact stable SceneId availability or introduction; array index alone is insufficient.
5. Exact nested edit-context identity representation and coordinate-space contract.
6. Whether FDM payload is one versioned blob or a set of normalized records; either must satisfy lossless unknown-version behavior.
7. Whether the host can losslessly preserve unknown envelope fields with its current serde/file format; if not, read-only compatibility behavior is mandatory.
8. Whether any current command intentionally commits multiple independent operations and therefore requires multiple receipts.
9. Exact maximum envelope/payload sizes, target counts, and nesting limits.
10. Exact cryptographic/random opaque-ID facility and test injection strategy.

These are bounded design tasks, not permission to defer the core ownership or atomicity guarantees.

---

## 14. Completion criteria and next gate

FDM-06.8 is complete as a specification when:
- ownership matrix and transaction semantics are accepted;
- identity transition matrix is approved;
- persistent envelope/history/save migration semantics are approved;
- session epoch/context freshness and result acceptance are approved;
- mutation coverage registry schema and required tests are accepted;
- implementation sequence and crate owners are agreed;
- host-specific confirmation items are either resolved or explicitly assigned to a pre-implementation spike.

**Recommended next milestone:** **FDM-06.9 — Host Transaction API and Persistent State Implementation Plan** (design/planning only), to translate this specification into a file-by-file host implementation plan with API signatures, migration ordering, compatibility constraints, and staged test ownership.

A separate explicit user authorization is required before any AniCraft or FDM runtime implementation begins.
