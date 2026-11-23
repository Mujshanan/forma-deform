# FDM-06.9 — Host Transaction API and Persistent State Implementation Plan

**Status:** File-by-file implementation plan; implementation not authorized by this milestone  
**Depends on:** FDM-06.4 through FDM-06.8  
**FDM-06.8 baseline:** `Mujshanan/forma-deform` PR #72, head `cce907a1c961b5676454b588ee89414382fc90fa`  
**AniCraft source baseline:** `Mujshanan/anicraft` commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Scope:** Translate the FDM-06.8 ownership specification into a staged host implementation plan with concrete module boundaries, API contracts, migration sequencing, compatibility handling, and test ownership.  
**Decision:** Implement the mutation boundary first, then persistent identity/envelope, then history and file compatibility, then runtime freshness and immutable snapshots. Keep FDM evaluation synchronous in the first integration slice. Do not implement FDM deformation in this milestone.

---

## 1. Executive plan

FDM-06.8 established the normative ownership model. FDM-06.9 defines how AniCraft should reach it without attempting a risky all-at-once rewrite.

The implementation is split into independently reviewable stages:

1. **S0 — Baseline and API inventory:** pin host source, map all public mutable access and mutation entry points, establish compile/test baselines.
2. **S1 — Persistent state types:** introduce host-owned document identity, target-incarnation ledger, extension envelope, and versioned payload container as types without changing user-visible behavior.
3. **S2 — Transaction draft and effect journal:** introduce a single transaction capability and typed mutation effects while preserving existing command semantics.
4. **S3 — Command and plugin migration:** route top-level and nested commands through the same transaction; close public mutable escape hatches.
5. **S4 — Side-effect settlement:** bring rig, motion, provisional cut, floating paste, and gesture settlement inside explicit transaction boundaries.
6. **S5 — Identity lifecycle:** add checked ID allocation, incarnation tokens, transition mapping, and explicit duplicate/import/conversion semantics.
7. **S6 — File envelope and migration:** serialize the persistent bundle through a backward-compatible file representation; preserve unknown payloads losslessly.
8. **S7 — History and dirty state:** snapshot the whole persistent bundle and make undo/redo restore identity and extension state atomically.
9. **S8 — Runtime freshness and context:** add session-instance identity, checked evaluation epoch, context generation, and conservative invalidation.
10. **S9 — Immutable evaluation snapshot:** expose one self-consistent immutable FDM input snapshot and retain synchronous evaluation.
11. **S10 — Host conformance and release gate:** complete mutation coverage, persistence, identity, history, failure, and compatibility tests before enabling FDM host integration.

The stages are intentionally ordered so that later features never depend on an unenforced mutation boundary. A type-only introduction is not considered completion of a stage if public mutation bypasses remain.

**No runtime implementation is authorized by FDM-06.9.** The plan itself is documentation-only. Each implementation stage must be separately authorized and reviewed.

---

## 2. Verified host starting point and implications

The plan is grounded in the pinned AniCraft source commit above. It is not a claim about any later host commit.

### 2.1 Current API facts

| Source | Verified current shape | Implementation consequence |
|---|---|---|
| `crates/engine/src/lib.rs` | `Session` exposes `pub doc: Document`; `execute` accepts a command ID and JSON params; command callbacks use `fn(&mut Session, &Value)` | The first hard boundary is a breaking internal API migration, not merely a new wrapper |
| `crates/engine/src/lib.rs` | `Session` owns scene/frame/editing/selection, history, gesture state, caches, provisional cut, floating paste, rig and motion state, plugins, and revision | Transaction design must distinguish persistent document state from editor/session state |
| `crates/engine/src/lib.rs` | `execute` snapshots before mutating commands, calls command code, settles rig and motion, updates StateId/revision, records history, restores on errors, then settles floating paste | Current transaction boundary is incomplete: settlement can occur after the command's principal commit/receipt point |
| `crates/engine/src/lib.rs` | `settle_floating` mutates the document and increments revision independently | It must become part of the triggering transaction or a separately explicit transaction |
| `crates/engine/src/lib.rs` | `replace_document` replaces the whole Session via `Session::new`, then restores editor-shared state | Document replacement needs an explicit transaction/session transition and new runtime session identity |
| `crates/doc/src/lib.rs` | `Document` serializes through private `DocumentFile` using `#[serde(from = "DocumentFile")]` | Add a versioned file envelope at this explicit serialization boundary; retain legacy document decoding |
| `crates/doc/src/lib.rs` | `Document` has public fields, `alloc_id`, `timeline_mut`; Timeline/Layer/Keyframe expose mutation methods | Merely privatizing `Session.doc` is insufficient if mutable document references or methods remain available elsewhere |
| `crates/doc/src/lib.rs` | `alloc_id` uses saturating increment | Replace saturation with checked allocation and explicit exhaustion failure |
| `crates/engine/src/history.rs` | `Snapshot` stores Document, StateId, scene/frame/layer/selection, floating paste, and command recording | Extend snapshots to include the entire persistent bundle, not runtime epoch/session identifiers |
| `crates/engine/src/history.rs` | `History::record/undo/redo` operate on Document snapshots | Update history's snapshot unit and memory accounting together |
| `crates/engine/src/cmds_plugin.rs` | `SessionHost` invokes `execute_nested` on the same mutable Session | Preserve one outer transaction and one undo step for plugin runs |
| `crates/doc/src/eval.rs` | Pure evaluation functions accept immutable Document and frame/scene context | Reuse pure evaluation; add a host-created immutable FDM snapshot alongside, not inside, the evaluator |
| `crates/engine/src/render_service.rs` | Render jobs carry cloned Document and use StateId/frame/view-derived cache keys | Keep rendering behavior independent; FDM result freshness needs a stronger stamp than StateId |
| `crates/engine/src/lib.rs` | `revision` is a public UI invalidation counter | Do not silently repurpose it as the FDM epoch; add a separately owned checked epoch |

### 2.2 Important boundary distinction

AniCraft has two broad categories of state:

- **Persistent document state:** artwork, scenes, layers, symbols, timelines, document metadata, future DocumentId, target ledger, and extension envelope.
- **Session/editor state:** active frame and scene, selection, active layer, editing stack, tool style, clipboards, gestures, provisional cut/floating state, caches, plugin registry, render jobs, and UI invalidation.

Some editor/session values influence FDM evaluation even though they are not authored persistent state. In particular, frame, scene, editing context, effective transforms, and any sampled animation inputs must be captured in the immutable evaluation context. They must not be copied indiscriminately into the persistent envelope.

### 2.3 Source files to pin and re-audit before implementation

The first implementation PR must re-fetch and pin these files at the then-current AniCraft base:

- `crates/engine/src/lib.rs`
- `crates/engine/src/history.rs`
- `crates/engine/src/cmds_plugin.rs`
- `crates/engine/src/cmds_file.rs`
- `crates/engine/src/cmds_recovery.rs`
- `crates/engine/src/transfer.rs`
- `crates/engine/src/parenting.rs`
- `crates/engine/src/cmds_motion.rs`
- `crates/engine/src/cmds_scene.rs`
- `crates/engine/src/cmds_group.rs`
- `crates/engine/src/cmds_modify.rs`
- `crates/engine/src/cmds_raw.rs`
- `crates/engine/src/render_service.rs`
- `crates/doc/src/lib.rs`
- `crates/doc/src/eval.rs`
- relevant file-format, app shell, CLI/control-channel, and export call sites discovered from current code search.

The source list is an audit starting point, not proof that it exhausts every mutation route. S0 must enumerate the actual call graph and mutation entry points on the implementation base.

---

## 3. Proposed module and API layout

These are proposed module boundaries and conceptual Rust signatures. They are not final code or permission to introduce all types in one commit. Names may be adjusted to host conventions while preserving semantics.

### 3.1 Engine-owned transaction module

Proposed new module: `crates/engine/src/transaction.rs` (private initially).

Responsibilities:

- own transaction lifecycle and the exclusive mutation capability;
- create the transaction draft or journal;
- collect typed effects and target identity transitions;
- run command body and all required settlement;
- validate persistent invariants;
- prepare StateId and evaluation epoch advances with checked arithmetic;
- publish persistent state atomically on the Session owner;
- emit one committed receipt;
- guarantee that failure publishes neither state nor receipt.

Conceptual API:

```rust
pub(crate) enum TransactionKind {
    Command,
    PluginRun,
    GestureUpdate,
    GestureCancel,
    Import,
    Recovery,
    Undo,
    Redo,
    DocumentReplace,
    FdmApply,
    FdmBake,
}

pub(crate) struct MutationReceipt {
    pub document_id: DocumentId,
    pub session_instance_id: SessionInstanceId,
    pub state_id: StateId,
    pub evaluation_epoch: EvaluationEpoch,
    pub context_generation: EvaluationContextGeneration,
    pub effects: CommittedMutationEffects,
}

pub(crate) struct MutationTxn<'a> {
    // Private capability; no mutable reference escapes.
    // Draft/journal representation selected after S0 ownership audit.
}

impl Session {
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value>;
    pub(crate) fn execute_nested(&mut self, id: &str, params: &Value) -> Result<Value>;
    fn transact<T>(
        &mut self,
        kind: TransactionKind,
        body: impl FnOnce(&mut MutationTxn<'_>) -> Result<T>,
    ) -> Result<(T, MutationReceipt)>;
}
```

The public command API may continue returning `Result<Value>` for compatibility. Internally, the transaction receipt is consumed by Session and diagnostics. A later API may expose receipts to host subsystems, but no plugin ABI or public UI contract should change in the first migration.

### 3.2 Persistent state module

Proposed new module: `crates/engine/src/persistent.rs` (or a dedicated host document-state module if ownership belongs in `crates/doc` after review).

Conceptual types:

```rust
#[derive(Clone)]
pub(crate) struct PersistentDocumentState {
    pub document: Document,
    pub extensions: HostExtensionEnvelope,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DocumentId(OpaqueId128);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IncarnationId(OpaqueId128);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TargetRef {
    pub document_id: DocumentId,
    pub element_id: ElementId,
    pub incarnation_id: IncarnationId,
}

pub(crate) struct HostExtensionEnvelope {
    pub envelope_version: u32,
    pub document_id: DocumentId,
    pub target_identities: TargetIdentityLedger,
    pub payloads: ExtensionPayloadMap,
}
```

Ownership rules:

- AniCraft generates and validates DocumentId and IncarnationId.
- AniCraft owns the ledger and envelope container.
- FDM owns the schema and migration of its own versioned payload.
- The host does not interpret arbitrary extension payload internals.
- No executable plugin data or arbitrary callback is stored in the envelope.
- Unknown payloads must be retained losslessly or destructive save must be refused.

The implementation must decide whether `DocumentId` and envelope types belong in `anicraft-doc` or `anicraft-engine` based on file-format ownership. The required invariant is one authoritative serialized representation, not a prescribed crate location.

### 3.3 Mutation effect journal

Proposed types, likely in `transaction.rs`:

```rust
pub(crate) enum TargetTransition {
    Created { new: TargetRef },
    Preserved { target: TargetRef },
    Retired { old: TargetRef },
    Replaced { old: TargetRef, new: TargetRef },
    Remapped { old: TargetRef, new: TargetRef },
}

pub(crate) enum MutationEffect {
    Target(TargetTransition),
    GeometryChanged { target: TargetRef },
    TopologyChanged { target: TargetRef },
    TransformChanged { target: TargetRef },
    HierarchyChanged,
    TimelineStructureChanged,
    TimelineSampleChanged,
    SceneChanged,
    NativeWarpStateChanged,
    ArmatureStateChanged,
    FdmPayloadChanged,
    DocumentReplaced,
    UnknownEvaluationDependencyChanged,
}
```

The effect journal is authoritative for invalidation and audit, but not a replacement for inspecting and validating the actual persistent state. If a command cannot report precise effects in an early migration stage, it must emit `UnknownEvaluationDependencyChanged` and trigger global invalidation.

### 3.4 Runtime state module

Proposed new module: `crates/engine/src/session_runtime.rs` or a private submodule of Session:

```rust
pub(crate) struct SessionRuntimeState {
    pub session_instance_id: SessionInstanceId,
    pub evaluation_epoch: EvaluationEpoch,
    pub context_generation: EvaluationContextGeneration,
}
```

- `SessionInstanceId` changes on new/open/replacement/recovery session creation.
- `EvaluationEpoch` advances on every committed evaluation-affecting persistent mutation and every relevant undo/redo/gesture cancellation.
- `EvaluationContextGeneration` advances when frame, scene, editing context, or another sampled runtime input changes.
- All counters use checked arithmetic. Overflow returns an explicit error before publication; never wrap or saturate.
- Runtime identifiers/counters are not serialized and are not restored from history.
- Existing `Session::revision` remains a UI invalidation field until every consumer is audited; do not alias it to the FDM epoch.

---

## 4. File-by-file implementation plan

### 4.1 `crates/doc/src/lib.rs` — document encapsulation and file representation

**Changes planned:**

1. Add a versioned serialized document-file envelope that can decode the existing legacy `DocumentFile` shape.
2. Preserve the existing scene/timeline compatibility conversion.
3. Include host extension envelope fields with explicit defaults for legacy documents.
4. Keep unknown extension payload values lossless, including unsupported versions and unknown keys.
5. Add explicit limits for envelope bytes, per-payload bytes, nesting depth, target count, payload count, and remap work.
6. Replace saturating `Document::alloc_id` with a checked allocator returning a typed error.
7. Inventory all public mutable fields and helpers. Move mutation behind crate-private APIs or a transaction capability in staged changes.
8. Retain immutable getters and pure evaluation interfaces.
9. Ensure sanitization does not silently invent target continuity or discard unknown extension content.

**Compatibility requirements:**

- Existing AniCraft files without an envelope load as legacy documents.
- A legacy document receives a newly generated DocumentId and target-incarnation ledger during migration.
- The migrated document remains dirty until the upgraded format is successfully saved.
- Existing file versions must continue to load through explicit migration steps.
- A file containing unknown extension data must round-trip without data loss, or saving must fail with a clear compatibility diagnostic.

**Do not:** put session epochs, render caches, selection, or pending transaction data into `Document` serialization.

### 4.2 `crates/engine/src/lib.rs` — Session boundary and command dispatch

**Changes planned:**

1. Replace public `Session.doc` with private persistent state.
2. Add immutable accessors for document inspection and snapshots.
3. Route all mutating command execution through `Session::transact`.
4. Replace `RunFn = fn(&mut Session, ...)` with a transaction-scoped command context in a controlled migration. Do not leave a second unrestricted `&mut Session` route for document mutation.
5. Separate persistent mutation capability from editor/session state access.
6. Keep selection, tool style, clipboards, active frame, and view state in the session layer; only changes that affect evaluation advance the relevant runtime context stamp.
7. Make document replacement an explicit transaction/session transition.
8. Ensure failed commands restore persistent state, identity ledger, envelope, history staging, and all transaction-local effects.
9. Ensure no fallible persistent operation occurs after atomic publication.
10. Preserve current command IDs, parameter JSON, confirmation behavior, and user-facing result values during the API migration.

**Command context migration target:**

```rust
pub(crate) struct CommandContext<'a> {
    pub(crate) txn: &'a mut MutationTxn<'a>,
    pub(crate) editor: &'a mut EditorSessionState,
}
```

The final lifetime/type arrangement must be validated in Rust rather than copied literally. Prefer smaller capabilities over passing the entire Session to every command.

### 4.3 `crates/engine/src/history.rs` — persistent bundle snapshots

**Changes planned:**

- Change `Snapshot` from storing only `Document` to storing `PersistentDocumentState`.
- Include DocumentId, target ledger, extension envelope, authored FDM payload, and any explicitly supported persistent bake provenance through that bundle.
- Keep StateId as a historical document-state identity, not an evaluation freshness authority.
- Keep session instance, evaluation epoch, context generation, worker IDs, caches, and pending requests out of snapshots.
- Update unique-memory accounting to include envelope and payload memory without double-counting shared data.
- Ensure history eviction drops the complete snapshot bundle.
- Make undo and redo restore artwork and identity/envelope together.
- Keep gesture history coalescing, command recording, and bounded history behavior intact.

### 4.4 `crates/engine/src/cmds_plugin.rs` — nested command transaction

**Changes planned:**

- Preserve the current one-outer-undo-step plugin-run behavior.
- Make `SessionHost` receive a constrained transaction command capability, not unrestricted mutable Session access.
- Nested commands join the active transaction and effect journal.
- A nested command failure aborts the outer transaction unless explicitly designated recoverable and proven not to leave partial persistent state.
- Keep current plugin ABI v1 request/response behavior during this milestone.
- Do not expose DocumentId, identity ledger mutation, or envelope mutation directly through the plugin ABI.
- Do not retire the ABI v1 SDK or historical adapters as part of this milestone.

### 4.5 Side-effect and command modules

The following modules require migration to the transaction capability and explicit effect reporting:

| Module | Required treatment |
|---|---|
| `cmds_file.rs` | Open/save/import/export; legacy migration; Save As vs Save Copy; atomic replacement |
| `cmds_recovery.rs`, `recovery.rs` | Recovery as explicit replacement/restore transaction; new runtime session identity |
| `transfer.rs` | Clipboard/import transfer identity remapping; no cross-document TargetRef leakage |
| `parenting.rs` | Transform/hierarchy effects and sampled effective-transform invalidation |
| `cmds_motion.rs` | Timeline sample effects and auto-key settlement inside transaction |
| `cmds_scene.rs` | Scene create/delete/duplicate/reorder and scene identity/context changes |
| `cmds_group.rs` | Group wrapper identity; member preservation; ungroup semantics |
| `cmds_modify.rs`, `cmds_path.rs`, `cmds_raw.rs`, `cmds_rawpoints.rs` | Geometry/topology effects and explicit target transitions for split/merge/rewrite |
| `cmds_timeline.rs`, `cmds_frames.rs`, `cmds_retime.rs` | Keyframe structure/sample effects and identity preservation across frame edits |
| `cmds_bone.rs`, `cmds_warp.rs` | Mark unsupported dependency changes; keep native Asset Warp and Smart Controller independent |
| `cmds_plugin.rs`, `plugin_export.rs` | Nested transaction capability and constrained plugin host boundary |
| `render_service.rs` | Keep render cache separate; consume immutable document snapshots; do not use StateId as FDM freshness |
| app shell, CLI, control channel | Read-only document access and unchanged command routing through Session |

This is a migration ownership map. S0 must identify every actual mutating function in each module and expand the table where code search finds additional paths.

### 4.6 New host tests and documentation

Proposed additions:

- `crates/engine/src/transaction.rs` unit tests for commit, rollback, no-op, nested execution, and effect aggregation.
- `crates/engine/src/persistent.rs` tests for identity ledger and envelope invariants.
- `crates/doc/src/lib.rs` serialization/migration tests for legacy and unknown fields.
- `crates/engine/src/history.rs` tests for persistent bundle snapshots and memory accounting.
- Integration tests under the existing engine test organization for command, plugin, file, import, undo/redo, and session replacement behavior.
- A machine-readable mutation coverage registry (location and format selected at S0) consumed by CI.
- A developer document describing how new commands declare mutation effects and add coverage tests.

No test file should be created under a guessed path before the existing test layout is inspected.

---

## 5. Transaction API contract and execution order

A transaction must follow this semantic sequence. The exact draft/journal representation is an implementation choice.

1. **Acquire owner capability:** verify the Session owner/dispatch context; reject reentrant publication.
2. **Capture pre-state:** immutable persistent bundle reference or snapshot, relevant editor context, and pre-transaction StateId.
3. **Create draft:** obtain exclusive transaction-scoped mutation access.
4. **Run operation:** execute command body and nested commands against the same draft and effect journal.
5. **Settle derived mutations:** finish rig edits, motion auto-key, pending cut/floating paste, group-frame constraints, import remaps, and any other persistent side effects that belong to the command.
6. **Validate:** document invariants, target ledger bijection/coverage, identity transitions, envelope limits, payload versions, and reference locality.
7. **Detect no-op:** compare authoritative persistent bundle and relevant evaluation inputs; do not create history or advance epoch for a true no-op.
8. **Prepare commit metadata:** allocate checked StateId/epoch/context values and construct immutable receipt data before publication.
9. **Publish atomically:** swap/publish the complete persistent bundle and history transition on the Session owner. No partial artwork/envelope/ledger publication.
10. **Invalidate caches:** invalidate document/evaluation caches from the committed effects; unknown effects invalidate globally.
11. **Publish receipt:** expose one receipt to internal host consumers only after state publication.
12. **Return result:** preserve command response semantics.

**Failure contract:** any failure before publication discards the draft and effect journal. Persistent document, identity ledger, envelope, history, dirty-state marker, and runtime epoch remain unchanged. If an operation has external irreversible side effects, it must be split into an explicit preflight/commit protocol and must not pretend that in-memory rollback reverses filesystem or process effects.

**Event delivery contract:** once persistent state is committed, an event notification failure cannot roll back the state or suppress the epoch. Consumers must be able to read the authoritative current epoch and recover from missed notifications.

---

## 6. Persistent identity, duplication, and import migration

### 6.1 Checked ID allocation

- Document-local ElementId/LayerId/SymbolId allocation must detect exhaustion before mutation.
- Never use saturating arithmetic, wraparound, or silent reuse.
- IncarnationId must be independently generated and collision-checked within the document ledger.
- Failed allocation must not leak a partially allocated identity into the committed document.
- A deleted numeric ID reused later receives a new IncarnationId.
- Undo restores the exact prior historical ledger state; redo restores the deletion state.

### 6.2 Target transition rules

| Operation | Target identity rule |
|---|---|
| In-place geometry/transform/style edit | Preserve target identity |
| New element | Fresh ElementId and IncarnationId |
| Duplicate/copy/paste | Fresh destination identity |
| Delete | Retire exact TargetRef |
| Undo delete | Restore exact prior TargetRef |
| Redo delete | Retire it again |
| Same-document move/reorder | Preserve identity |
| Group | New group-wrapper identity; members retain identities if they remain the same targets |
| Ungroup | Retire wrapper; surviving members preserve identities |
| Split/merge/shape rewrite | Explicit replace mapping; never infer from geometric similarity |
| Scene duplication | Fresh identities for duplicated targets |
| Import from another document | Fresh destination IDs and incarnations; rewrite internal references |
| Save As / rename / move | Preserve DocumentId and target identities |
| Save a Copy / Duplicate Document | New DocumentId; create fresh copied target incarnations and remap internal references |
| Reopen same file | Restore persistent DocumentId and ledger; create a new SessionInstanceId |

### 6.3 Import policy

Import is not one generic identity behavior. It must declare whether it is:

- complete supported subgraph import, where all internal FDM references can be remapped atomically;
- artwork-only import, where FDM payload and references are intentionally excluded with a user-visible diagnostic;
- unsupported partial FDM subgraph import, which must be rejected or quarantined rather than leaving dangling or cross-document references.

No TargetRef may refer across DocumentId boundaries.

---

## 7. File format and compatibility strategy

### 7.1 Versioning layers

Keep versions distinct:

- file/document format version;
- host extension envelope version;
- per-extension payload schema version;
- FDM authored model schema version;
- FDM evaluation/policy version.

A change in one layer must not silently masquerade as a change in another.

### 7.2 Migration order

1. Decode the existing legacy `DocumentFile` into the existing in-memory Document shape.
2. Detect whether a host envelope exists.
3. If absent, generate DocumentId and a complete target-incarnation ledger for supported live targets.
4. Validate IDs, ledger coverage, payload bounds, and references.
5. Attach the envelope to the persistent state bundle.
6. Mark the migrated document dirty only after successful in-memory migration.
7. Save through the new envelope format using an atomic file replacement strategy already supported by the host.
8. Do not clear dirty state until the new file write succeeds.
9. Preserve unknown extension payloads through load/save. If preservation cannot be guaranteed, refuse destructive upgrade/save and keep the original file intact.

### 7.3 Save semantics

- Save to current path: preserve DocumentId and identity ledger.
- Save As to a new path: preserve DocumentId and identity ledger.
- Explicit Save a Copy/Duplicate: create a new DocumentId and fresh copied target incarnations; rewrite all internal references.
- Import into existing document: retain destination DocumentId; allocate new destination target IDs/incarnations for imported objects.
- Failed save: leave dirty state unchanged.
- Failed Save Copy: do not publish or expose a partially created copy as a valid document.

---

## 8. History, dirty state, and runtime state

### 8.1 History bundle

History snapshots must include:

- Document;
- DocumentId;
- target identity ledger;
- host extension envelope;
- authored FDM payload;
- explicitly supported authored bake provenance.

History snapshots must exclude:

- SessionInstanceId;
- EvaluationEpoch;
- EvaluationContextGeneration;
- request/job IDs;
- worker/cancellation state;
- render/evaluation caches;
- transient notifications.

### 8.2 Undo/redo semantics

- Undo/redo restore the full persistent bundle atomically.
- StateId may return to a historical value and remains useful for document-state/render cache identity.
- EvaluationEpoch advances monotonically after undo and redo.
- Target incarnation values are restored from the historical bundle; they are not regenerated on undo.
- Any outstanding future worker result would be stale after undo/redo; asynchronous evaluation remains disabled in the initial implementation.
- History memory accounting must include payload/envelope allocations and shared ownership correctly.

### 8.3 Dirty state

Dirty-state comparison must account for the persistent bundle, not only artwork Document StateId. A change to DocumentId, target ledger, envelope, or FDM payload is persistent and dirty. Runtime epoch/context changes are not dirty by themselves.

A successful save records the exact persistent bundle/version saved. A failed save cannot update the saved marker.

---

## 9. Evaluation freshness and immutable snapshot

### 9.1 Runtime identifiers

| Identifier | Lifetime | Persisted? | Undo/redo behavior |
|---|---|---:|---|
| DocumentId | One logical document identity | Yes | Restored with persistent bundle |
| ElementId | Document-local numeric lookup | Yes | Restored with persistent bundle |
| IncarnationId | One target lifetime | Yes | Restored with persistent bundle |
| StateId | One document content state | History state | May return to historical value |
| SessionInstanceId | One live Session instance | No | Never restored; replacement creates new |
| EvaluationEpoch | Monotonic within Session instance | No | Advances on undo/redo |
| EvaluationContextGeneration | Monotonic sampled context generation | No | Advances when frame/scene/edit context changes |

### 9.2 Initial invalidation model

- Every successful evaluation-affecting persistent transaction advances one checked global EvaluationEpoch.
- Frame, scene, and edit-context changes advance a separate checked context generation.
- A command with incomplete effect reporting emits an unknown effect and globally invalidates.
- Fine-grained revisions are deferred until the coverage registry proves all mutation paths are classified.
- Existing render cache and render job keys remain separate; do not widen this milestone into a renderer rewrite.

### 9.3 Immutable FDM input snapshot

The host snapshot must be self-consistent and include at minimum:

- DocumentId and SessionInstanceId;
- EvaluationEpoch and EvaluationContextGeneration;
- StateId as diagnostic/history metadata only;
- exact TargetRef;
- scene, frame, and edit-context identity;
- immutable source Path2D geometry and coordinate-space declaration;
- effective transform and relevant hierarchy/timeline sample fingerprints;
- topology/source fingerprints and FDM authored payload/schema/policy versions;
- explicit unsupported markers for native Asset Warp, armature, nested symbol/group contexts, and other unsupported dependencies.

The snapshot is built on the Session owner from one committed persistent bundle and one sampled context. FDM core receives only this immutable value. Initial evaluation remains synchronous; acceptance checks still validate that the snapshot is current before any apply/bake command mutates the document.

### 9.4 Result acceptance

Before a result can be applied or baked, the host must verify in one owner-thread operation:

- request identity and active session instance;
- DocumentId and current EvaluationEpoch;
- context generation, scene, frame, and edit context;
- exact TargetRef including incarnation;
- source/topology/transform/hierarchy/timeline dependency fingerprints;
- FDM payload/schema/policy versions;
- cancellation/supersession state if applicable;
- finite and structurally valid output;
- session not closing or replacing the document.

No result may be accepted based on StateId or ElementId alone.

---

## 10. Mutation coverage registry

S0 must create a machine-checkable registry of every production mutation entry point. The registry may be a Rust table, test manifest, or generated inventory; its format must fit current build conventions.

Each entry must record:

- command/helper ID and source location;
- whether it can mutate persistent document state;
- transaction kind and required capability;
- identity transition behavior;
- effects/dependencies emitted;
- settlement behavior;
- rollback and history behavior;
- owner tests;
- status: classified, migrated, or explicitly exempted.

CI must fail when a newly registered mutating command/helper lacks a classification. A source-text grep alone is not sufficient proof of coverage; it is an inventory aid. The registry must be reconciled against the actual command registry, direct Document mutation call sites, non-command mutation helpers, file/recovery routes, gestures, and nested plugin execution.

No mutation path may be marked complete merely because it calls `Session::execute` if it can subsequently mutate persistent state outside that transaction.

---

## 11. Test ownership and conformance matrix

The following test IDs are proposed for the host implementation. They are planning requirements, not tests claimed to have run.

| Test family | Owner | Required cases |
|---|---|---|
| TXN-001–008 | `transaction.rs` | commit, rollback, no-op, nested command, nested error, receipt ordering, checked counter overflow, no fallible post-publication step |
| TXN-009–014 | engine integration | plugin outer transaction, plugin nested command, confirmation path, command response compatibility, non-mutating command, reentrancy rejection |
| TXN-015–022 | command/gesture integration | rig settlement, motion auto-key, provisional cut, floating paste, gesture coalescing, gesture cancellation, selection-only update, unknown effect invalidation |
| ID-001–010 | persistent identity module | new target, preserve edit, delete, ID reuse/new incarnation, undo/redo restoration, group/ungroup, split/merge replacement, scene duplicate, import remap, cross-document rejection |
| FILE-001–010 | document/file module | legacy load, envelope load, migration dirty state, Save As identity, Save Copy new identity, failed save, unknown payload round trip, unsupported payload diagnostic, resource limits, atomic replacement |
| HIST-001–008 | history module | full bundle snapshot, undo/redo exact ledger, payload restore, StateId restoration, epoch monotonicity, memory accounting, history eviction, redo clearing after new commit |
| CTX-001–006 | session runtime | frame change, scene change, edit-context change, session replacement, reopen, checked overflow |
| SNAP-001–008 | snapshot adapter | exact target resolution, coordinate space, immutable snapshot, timeline sample stamp, transform/hierarchy stamp, unsupported marker, payload version stamp, stale apply rejection |
| COV-001–006 | CI/coverage registry | all command namespaces inventoried, direct mutable access absent, every mutation helper classified, nested plugin path classified, side effects classified, new mutating command fails closed |

### Required regression invariants

- Existing command IDs, JSON parameter schemas, confirmation requirements, and return values remain stable during the transaction API migration.
- Existing undo/redo labels, gesture coalescing, cut/floating behavior, and plugin one-step undo behavior remain stable.
- Existing files without extension envelopes remain loadable.
- Unknown extension payloads are not lost.
- Native Asset Warp behavior is unchanged.
- Smart Controller and Form Deform remain independent under CCC-01.5.
- No bitmap/text/symbol/group/nested nonlinear deformation support is introduced by this host transaction work.
- No asynchronous FDM evaluation is introduced.

---

## 12. Staged implementation PR sequence

Each stage should be a separately reviewable AniCraft PR or a narrowly scoped series of PRs. Do not combine all stages into one branch.

| Stage | Proposed PR scope | Exit gate |
|---|---|---|
| S0 | Re-audit current base, mutation inventory, API compatibility report, test baseline | Complete mutation registry and exact source pin |
| S1 | Add DocumentId, IncarnationId, TargetRef, envelope and persistent-state types; no behavior migration beyond construction defaults | Type invariants and serialization design tests |
| S2 | Add private transaction draft/effect journal and transaction unit tests behind existing command path | Atomic commit/rollback demonstrated |
| S3 | Migrate command callback/context API; privatize mutable Session document access | No production direct mutable Session document escape |
| S4 | Move rig/motion/cut/floating/gesture settlement into transaction lifecycle | Side-effect regression suite passes |
| S5 | Checked ID allocation and target identity ledger lifecycle | Identity transition matrix passes |
| S6 | Versioned file envelope, legacy migration, Save As/Copy and import remapping | File compatibility and unknown payload tests pass |
| S7 | Persistent-bundle history and dirty-state semantics | Undo/redo/save/recovery tests pass |
| S8 | SessionInstanceId, EvaluationEpoch, context generation and global invalidation | Freshness and overflow tests pass |
| S9 | Immutable FDM snapshot adapter and synchronous evaluation seam | Snapshot conformance passes; no async |
| S10 | Full coverage registry, integration suite, release evidence | All gates green on exact final host commit |

### PR sequencing constraints

- S0 is read-only and must precede implementation.
- S1 may introduce types but must not be advertised as persistent support until S6 and S7 are complete.
- S2/S3 should be staged so the repository remains buildable; use compatibility shims only temporarily and document their removal gate.
- S4 must land before claiming all mutations are transaction-atomic.
- S5 must precede storing durable FDM target references.
- S6 and S7 must be reviewed together for file/history state completeness, even if implemented in separate PRs.
- S8 must not reuse StateId or the existing UI revision counter.
- S9 is not a deformation algorithm or UI feature; it is a snapshot boundary only.
- S10 is the integration release gate, not authorization to enable async or expand supported target types.

---

## 13. Compatibility and migration risks

| Risk | Why it matters | Mitigation / gate |
|---|---|---|
| Public `Session.doc` API consumers | Removing it may affect app shell, tests, CLI, plugins, and downstream crates | S0 consumer inventory; immutable accessor migration; compile-time check |
| `RunFn(&mut Session)` signature | Commands currently have broad authority | Introduce CommandContext and migrate namespace by namespace; no dual unrestricted path at completion |
| Public mutable Document fields/methods | Direct mutation can bypass Session | Capability/private API migration and coverage checks |
| Serde representation changes | Existing document files must remain readable | Versioned envelope plus legacy decoder and round-trip fixtures |
| Unknown extension payload | A host upgrade must not destroy future data | Lossless opaque retention or refuse destructive save |
| History memory estimates | Payloads may be large or shared | Account for envelope/payload bytes and preserve bounded history |
| Clipboard transfer between documents | Current editor clipboards survive document switching | Remap identities at destination; no cross-document TargetRef |
| Group/scene/split/merge semantics | Numeric ID movement does not prove target continuity | Explicit per-operation identity maps and tests |
| Save As vs Save Copy ambiguity | Different identity semantics | Separate explicit commands/operations and user-facing tests |
| Gesture/cut/floating settlement | Persistent mutation may happen after command execution | Make settlement transaction-owned; receipt only after settlement |
| UI revision consumers | Existing revision is a UI invalidation counter | Keep separate from FDM EvaluationEpoch until all consumers are migrated |
| Async render/evaluation | Existing render service has separate job lifecycle | Do not assume renderer freshness is sufficient for FDM; keep FDM sync |

---

## 14. Explicit non-goals and constraints

This milestone does not:

- modify AniCraft runtime code;
- modify FDM runtime code;
- implement a deformation operator or evaluator;
- add FDM UI, persistence behavior, apply, or bake;
- authorize asynchronous evaluation;
- change native Asset Warp;
- combine native Asset Warp with FDM;
- combine Smart Controller and Form Deform;
- introduce multi-deformer composition;
- add bitmap, text, symbol, group-wrapper, nested-symbol, or nonlinear-parent deformation support;
- change the plugin ABI or retire the ABI v1 SDK;
- retire historical WASM adapters;
- change the existing renderer architecture;
- merge PR #72 or any earlier PR;
- authorize implementation or release.

CCC-01.5 remains binding: Smart Controller binding evaluation and Form Deform pin evaluation are independent. No output composition is introduced here.

---

## 15. Decisions required before implementation authorization

The following host facts must be confirmed at S0 rather than guessed:

1. Which thread owns Session mutation, and how app-shell/CLI/control-channel calls are dispatched to it?
2. Are commands ever reentrant or concurrent against one Session?
3. Which external crates consume `Session.doc`, `RunFn`, or mutable Document methods?
4. Which file read/write layer owns the final serialization envelope and atomic replacement?
5. How are Save As and Save a Copy currently represented in the command/UI flow?
6. Which command and helper paths mutate persistent data without `Session::execute`?
7. What is the complete target universe for an incarnation ledger: elements only, or also layers/symbols/other addressable FDM targets?
8. How are ElementId values represented and remapped in nested groups, symbols, timeline keyframes, and imported assets?
9. What opaque 128-bit ID generator is already available and acceptable for Windows and all supported builds?
10. How should unknown extension payload bytes be preserved by the current serde/file format?
11. Which persistent fields are included in dirty-state comparison and recovery/autosave?
12. What exact import modes can preserve a complete FDM subgraph?
13. Which FDM source model and payload schema version will be stored first?
14. What is the host's existing test organization and CI gate for file compatibility fixtures?

Any unresolved item that affects identity, atomicity, file losslessness, or result freshness blocks the dependent implementation stage.

---

## 16. Completion criteria for FDM-06.9

FDM-06.9 is complete when:

- the host baseline and source pin are explicit;
- each known mutable Session/Document escape has a migration owner;
- transaction and persistent-state API boundaries are defined;
- every major host source module has a planned migration responsibility;
- command/plugin/gesture side-effect ordering is specified;
- file migration and unknown-payload compatibility are specified;
- identity and Save As/Copy/import transitions are mapped;
- history and dirty-state ownership are specified;
- runtime freshness and immutable snapshot interfaces are specified;
- test ownership and release gates are assigned;
- open host facts are listed as blockers rather than silently assumed;
- the document remains planning-only and makes no runtime implementation claim.

**Recommended next milestone:** FDM-06.10 — Host Baseline Reconciliation and S0 Mutation Inventory. It should re-pin the then-current AniCraft base, inspect all Session/Document mutable consumers and mutation entry points, map command dispatch and file/recovery flows, confirm owner-thread behavior, and produce the implementation-ready S0 inventory. Runtime implementation should begin only after that audit and separate explicit authorization.
