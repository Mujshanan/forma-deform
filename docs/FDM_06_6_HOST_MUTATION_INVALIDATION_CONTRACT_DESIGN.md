# FDM-06.6 — Host Mutation Invalidation Contract Design

**Status:** Design proposal; implementation not authorized  
**Host:** `Mujshanan/anicraft`  
**Pinned host source:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**FDM lineage:** FDM-06.5 PR #67, head `a67a2391365ffbe3491db1832e64eb304c4e2987`  
**Prior identity design:** FDM-06.4 PR #66  
**Scope:** Define the host-owned mutation/invalidation seam, transaction and rollback semantics, target identity transitions, immutable evaluation snapshots, and stale-result acceptance.  
**Non-goals:** Implement the host seam, change AniCraft source, change FDM algorithms, enable async evaluation, or define FDM/native Warp or Smart Controller/FDM composition.

---

## 1. Executive decision

FDM-06.5 found that AniCraft has a coarse session revision and many distributed mutation paths, but no incarnation ledger or dependency-class invalidation API. FDM-06.6 proposes one **transaction-aware host mutation boundary** through which every document mutation reports its semantic effects.

The design selects a staged strategy:

1. **First implementation: conservative global invalidation.** Every successful document mutation advances one monotonic session evaluation epoch. The same transaction reports target identity transitions and a set of dependency categories for audit/diagnostics. The host may compute fingerprints for immutable snapshots, but must not reuse results selectively until mutation coverage is proven.
2. **After coverage: fine-grained revisions as an optimization.** Per-target geometry/topology/transform and per-layer hierarchy/timeline revisions may be added only when every mutation path, undo/redo, import, frame transfer, and rollback is instrumented and tested. They never replace the global epoch as the final stale-result fence.
3. **Synchronous evaluation first.** The acceptance predicate is designed now, but async work remains disabled until the host proves snapshot isolation, atomic acceptance, and stale-result rejection under races.

The host owns the mutation boundary, identity ledger, epoch, snapshot creation, and result acceptance. FDM owns its authored model, deterministic calculation, and declaration of the dependencies it consumes.

**This is a contract design only. It does not authorize AniCraft or FDM runtime implementation.**

---

## 2. Contract invariants

The implementation must preserve these invariants:

1. No document mutation bypasses the host mutation boundary.
2. A failed or rolled-back transaction changes neither persistent document state nor the identity ledger, and does not publish a successful mutation event.
3. Every successful mutation that can affect any FDM evaluation advances the monotonic session evaluation epoch before another result can be accepted.
4. Target identity changes are explicit outcomes: create, preserve, retire, replace, or remap. No continuity is inferred from numeric ID, name, order, selection, or geometry similarity.
5. A target reference resolves only in its owning document and only when its ElementId and incarnation token match the live ledger.
6. Undo/redo restore persistent authored document state and its identity ledger from the same history snapshot; runtime epoch is never restored backwards.
7. Frame, scene, and edit context are evaluation inputs even when document content and StateId are unchanged.
8. A result is accepted only against the exact immutable snapshot stamp from which it was calculated.
9. Native Asset Warp and armature/skin state are either explicitly supported by a future contract or rejected by the initial FDM adapter. They are never silently treated as ordinary FDM Path2D.
10. No FDM result mutates the document during evaluation. Applying/baking is a separate explicit host command and transaction.
11. A missing event, unknown dependency, unsupported payload, overflow, or unclassifiable mutation fails closed: invalidate globally or reject the FDM evaluation; never reuse a possibly stale result.
12. Fine-grained invalidation is an optimization only. Correctness must remain valid when the host falls back to a global epoch.

---

## 3. Ownership and boundary

### 3.1 Host-owned types (conceptual)

The following names are illustrative contract types, not current AniCraft APIs:

```rust
struct MutationContext {
    transaction_id: TransactionId,
    origin: MutationOrigin,
    scope: MutationScope,
}

struct MutationOutcome {
    target_transitions: Vec<TargetTransition>,
    dependencies: DependencySet,
    affected_scope: AffectedScope,
    invalidation: InvalidationStrength,
}

enum TargetTransition {
    Created { element: ElementId, incarnation: IncarnationId },
    Preserved { target: TargetRef },
    Retired { target: TargetRef },
    Replaced { old: TargetRef, new: TargetRef },
    Remapped { old: TargetRef, new: TargetRef },
}

struct MutationReceipt {
    transaction_id: TransactionId,
    committed_epoch: EvaluationEpoch,
    changed_state: StateId,
    outcome: MutationOutcome,
}
```

The exact Rust representation can differ. The semantic obligations cannot.

### 3.2 Mutation origins

The origin is diagnostic and must not change invalidation correctness:

- user command;
- pointer/gesture edit;
- multi-command macro or batch;
- import/paste/transfer;
- timeline operation;
- undo;
- redo;
- document replacement/open;
- recovery/migration;
- host automation or plugin command;
- FDM apply/bake command.

A plugin command is not exempt. If it changes the document, its host command path must produce the same receipt as a built-in command.

### 3.3 Boundary placement

The preferred seam is the existing host command/transaction boundary, with lower-level helpers required to return or accumulate semantic mutation outcomes. The transaction wrapper is the final enforcement point; individual command modules are responsible for reporting target transitions and affected dependency classes.

A command must not mutate a live Document through an untracked mutable reference after the transaction has emitted its receipt. Any post-commit deferred mutation must start a new transaction and advance the epoch again.

For direct document helpers used by importers, tests, recovery, or internal tools, require one of:
- a transaction-scoped mutation context passed into the helper; or
- a builder/mutable draft that cannot be published until its complete mutation outcome is committed.

There must be no public "mutate document and remember to bump revision later" path.

---

## 4. Dependency taxonomy

Use a typed dependency set. Initial categories:

| Dependency | Meaning | Typical mutation sources |
|---|---|---|
| `TargetLifecycle` | Target created, retired, replaced, or remapped | Draw, delete, duplicate, convert, group/ungroup, import |
| `SourceGeometry` | Authored Path2D commands/coordinates changed | Path/raw-point edits, shape rewrite |
| `SourceTopology` | Command/subpath/anchor structure changed | Add/delete/convert point, close/open, split/join |
| `LocalTransform` | Local affine transform, pivot, or placement changed | Move/rotate/scale, transform commands |
| `Hierarchy` | Parent, containment, layer placement, or effective transform ancestry changed | Rig-parent, group, layer move |
| `TimelineStructure` | Keyframe/frame/layer/tween structure changed | Insert/remove/move/copy frame, tween create/remove |
| `TimelineSample` | Evaluated value at the requested frame changed | Motion key/ease/path/property edit, keyframe content |
| `NativeWarpState` | Native Asset Warp rest path, mesh, pins, or output changed | Warp commands, native Warp apply |
| `ArmatureState` | Skin binding, rest shape, pose, or weight changed | Bone bind/reshape/weight commands |
| `EditContext` | Active nested drawing/symbol edit scope changed | Enter/exit nested editing, symbol edit |
| `FdmAuthoredState` | FDM payload or topology/policy changed | FDM authoring commands |
| `VisibilityOrAvailability` | Target enters/leaves evaluated scene or becomes unsupported | Layer visibility/type, masks, scene changes |

Unknown future dependencies map to `Unknown`, which forces global invalidation and disables selective reuse until classified.

A transaction may report multiple dependencies. The host must not attempt to infer a narrower category from the command's display name.

---

## 5. Mutation transaction protocol

### 5.1 Required lifecycle

A document mutation follows this protocol:

1. **Begin:** allocate a transaction ID and capture the pre-mutation state/history boundary. Start a staged identity-ledger view or equivalent rollback-safe journal.
2. **Mutate:** execute command logic. All identity-affecting helpers report explicit transitions. All dependency-affecting helpers mark dependency categories and affected scope.
3. **Validate:** verify document invariants, identity uniqueness, target-ledger consistency, payload limits, and operation-specific preconditions.
4. **Commit document and ledger atomically:** publish the document and its identity ledger as one logical state.
5. **Advance epoch:** increment the session evaluation epoch with checked arithmetic. Epoch overflow is a hard failure; never saturate or wrap.
6. **Publish receipt:** expose the committed epoch and semantic outcome to cache invalidation, rendering, diagnostics, and (eventually) FDM scheduling.
7. **Record history:** the persistent document snapshot includes the identity ledger and FDM authored extension state. The runtime epoch is excluded from historical restoration.

The order of steps 4–6 must be implemented so no observer can accept a result against the old epoch after the new document state becomes visible. The simplest correct implementation is one session lock/serialized command lane that publishes state and epoch atomically to readers.

### 5.2 Failed commands and rollback

If mutation, validation, or persistence fails before commit:
- discard staged document changes;
- discard staged identity transitions;
- restore allocator/identity-ledger state to the pre-transaction state;
- do not advance the epoch;
- do not publish a success receipt;
- return the original command error.

If a failure occurs after the commit point, it is not a rollback of the document. The committed state remains authoritative; downstream consumers must recover/retry invalidation delivery from the committed receipt or epoch. An event queue failure must never silently erase the invalidation.

### 5.3 Gestures and coalesced history

History grouping and freshness are separate concerns.

- Each visible committed mutation in a drag/gesture advances the epoch, even when many such mutations are coalesced into one undo step.
- The gesture's history policy may keep one before-snapshot and one final snapshot; it must not collapse multiple runtime epochs into one.
- A no-op command that produces no document/identity/dependency change does not advance the epoch.
- A command that changes only UI selection/view state does not advance the document evaluation epoch unless that state is explicitly an FDM evaluation input (for example, nested edit context or active scene).

### 5.4 Multi-command operations

An operation advertised as atomic (paste, import, group/ungroup, apply/bake) must execute as one transaction and publish one receipt only after all sub-operations succeed. Partial success is forbidden unless the command contract explicitly describes partial application and reports each committed transaction separately.

A macro that intentionally runs independent user commands may produce one receipt per committed command. One outer undo group does not imply one mutation epoch.

---

## 6. Target identity transition rules

The identity ledger is persistent document state and changes in the same transaction as artwork.

| Operation | Required transition |
|---|---|
| Create element | Allocate ElementId and fresh IncarnationId; register before commit |
| In-place path/transform/style edit | Preserve TargetRef; mark affected dependencies |
| Duplicate | Fresh ElementId and fresh IncarnationId for every duplicate |
| Delete | Retire exact TargetRef; descendants/contained targets removed by the operation must also be enumerated |
| Delete then recreate same numeric ElementId | New IncarnationId; old TargetRef remains dangling |
| Replace/convert | Explicit old→new transition; default is retire old + create new unless command explicitly guarantees continuity |
| Group | Create fresh group identity; moved children retain or replace identity only according to explicit command semantics |
| Ungroup | Every reconstructed output gets an explicit mapping; never map by array index/order |
| Paste/import/cross-document transfer | Fresh destination identities; remap complete internal FDM subgraphs atomically or reject/quarantine |
| Undo/redo | Restore ledger with the matching historical document snapshot; advance runtime epoch |
| Save As/rename/move | Preserve DocumentId for ordinary save/path change |
| Save a Copy/duplicate document | New DocumentId; all target refs are scoped to the new document identity |

### 6.1 Allocation failure

ElementId allocation and IncarnationId generation must fail closed:
- ElementId exhaustion returns an error before publication; no saturating reuse.
- IncarnationId uses a collision-resistant opaque 128-bit value and is checked against live ledger entries.
- Duplicate incarnation tokens within a document are rejected.
- Random/token generation failure aborts the transaction.
- No fallback to timestamp, name, geometry hash, or sequence derived from mutable artwork.

### 6.2 Ledger consistency

At commit:
- every live FDM-addressable ElementId has exactly one incarnation token;
- no deleted ElementId remains live in the ledger;
- every authored FDM target reference resolves to a live exact TargetRef or is retained as an explicit dangling reference with a diagnostic state;
- no target reference crosses DocumentId boundaries;
- identity ledger entries are unique and bounded by document limits.

Legacy documents without an identity ledger require deterministic one-time migration on load. Migration must allocate tokens, mark the document as migrated/dirty as appropriate, and never invent continuity with an external pre-migration TargetRef.

---

## 7. Evaluation snapshot and stamp

### 7.1 Immutable snapshot

The host constructs a snapshot on the serialized session lane from one committed document state. It must not expose mutable document references to the evaluator.

Conceptual shape:

```rust
struct FdmEvaluationSnapshot {
    stamp: EvaluationStamp,
    target: TargetRef,
    source_path: ImmutablePath2D,
    coordinate_space: CoordinateSpace,
    local_transform: TransformSnapshot,
    effective_transform: TransformSnapshot,
    dependency_fingerprints: DependencyFingerprints,
    support: FdmSupportStatus,
}
```

The snapshot is either fully constructed and internally consistent or not returned. If the target disappears, its incarnation mismatches, or any required dependency is unsupported, snapshot creation returns a typed error.

### 7.2 Evaluation stamp

```text
EvaluationStamp {
  document_id,
  session_instance_id,
  evaluation_epoch,
  state_id,
  scene_id,
  frame,
  edit_context_id_or_fingerprint,
  target_ref,
  source_geometry_fingerprint,
  source_topology_fingerprint,
  local_transform_fingerprint,
  effective_transform_fingerprint,
  hierarchy_fingerprint,
  timeline_sample_fingerprint,
  native_warp_fingerprint_or_unsupported,
  armature_fingerprint_or_unsupported,
  fdm_payload_revision,
  fdm_policy_version,
  snapshot_schema_version
}
```

Rules:
- `session_instance_id` is fresh on every document replacement/open/session reconstruction. It is not persisted.
- `evaluation_epoch` is monotonic for the life of a session instance. It is not part of undo history.
- `state_id` is useful for diagnostics/cache partitioning, but is not a freshness authority.
- `frame`, scene, and edit context are explicit even if a global epoch is used.
- Fingerprints are canonical, versioned, and computed from the actual evaluated dependencies. Hash equality is an optimization/identity check, not a substitute for target identity.
- The stamp carries explicit unsupported markers; absence of a field must not be interpreted as "no dependency".
- Policy and snapshot schema versions prevent reuse across changed semantics.

### 7.3 Coordinate-space contract

The snapshot must declare whether the core input is:
- authored local Path2D;
- target-local evaluated path after timeline sampling; or
- a host-transformed path in a documented coordinate system.

The initial integration must choose exactly one representation and test it. Do not transform the source path twice. The output coordinate space must be explicit and compatible with the host apply/render seam.

---

## 8. Cache invalidation strategy

### 8.1 Phase A — global correctness fence (required first)

- One session-wide monotonic evaluation epoch.
- Any committed document mutation that may affect FDM increments it.
- Frame/scene/edit-context changes change the request stamp independently.
- Every accepted result is associated with the exact epoch and full request stamp.
- Unknown/unclassified mutations invalidate all FDM cache entries.
- Cache entries from prior session instances are unreachable.
- No cross-document cache reuse unless the full DocumentId and complete dependency stamp match (default: prohibit).

This is intentionally conservative. It may recompute more than necessary, but its correctness does not depend on perfect dependency classification.

### 8.2 Phase B — fine-grained revisions (deferred optimization)

Only after mutation coverage is complete, introduce optional counters:
- per-target lifecycle generation;
- per-target source geometry/topology;
- per-target local transform;
- per-layer hierarchy;
- per-scene/timeline structure and sample;
- FDM payload revision.

Each counter is host-owned, monotonic, and updated inside the same transaction as its mutation. Undo/redo must advance the runtime generation or restore a content stamp while retaining the global epoch fence; counters must never permit an old result to become current again.

Fine-grained revisions may avoid rebuilding fingerprints or recomputing unaffected targets. They may not allow an old result to pass the global epoch acceptance predicate if any global mutation has occurred since its snapshot. A future optimization can relax that only with a separately specified dependency-scoped acceptance protocol and complete proof of mutation coverage.

**Decision:** do not implement fine-grained counters in the first host integration. Use global invalidation plus canonical fingerprints for diagnostics and future optimization.

---

## 9. Result acceptance and stale-result suppression

Even though initial evaluation is synchronous, define acceptance before any async scheduling is added.

A result may be accepted only if all conditions hold at one atomic acceptance point:

1. Session instance matches.
2. DocumentId matches.
3. Current evaluation epoch equals the snapshot epoch.
4. Current scene, frame, and edit context match.
5. TargetRef still resolves to the exact same incarnation.
6. The target remains in the supported evaluation scope.
7. Required dependency fingerprints and policy/schema versions match.
8. The request has not been cancelled or superseded.
9. The result is finite, structurally valid, and conforms to the FDM output contract.
10. The host has not entered document replacement, close, or recovery transition.

Conceptual predicate:

```text
accept(result, snapshot, current_session) iff
  result.request_id == snapshot.request_id
  && current_session.instance_id == snapshot.session_instance_id
  && current_session.document_id == snapshot.document_id
  && current_session.evaluation_epoch == snapshot.evaluation_epoch
  && current_session.scene == snapshot.scene_id
  && current_session.frame == snapshot.frame
  && current_session.edit_context == snapshot.edit_context
  && resolve_exact(snapshot.target_ref) == LiveTarget
  && current_dependencies_match(snapshot.stamp)
  && !request.cancelled
  && output_valid(result)
```

If any check fails, discard the result without document mutation, cache publication, selection changes, or user-visible replacement of newer output. A stale result is not an error in the document; it is an expected rejected computation.

The check and publication must be atomic relative to mutations. A check-then-unlock-then-publish race is forbidden.

---

## 10. Undo, redo, save, and document replacement

### Undo/redo
- Identity ledger, FDM authored extension payload, and artwork are one persistent history snapshot.
- Undo/redo restore them together.
- The runtime epoch advances after each successful undo/redo, even if StateId returns to an earlier value.
- Any pending request from before undo/redo is rejected.
- Failed undo/redo leaves document, ledger, and epoch unchanged.

### Save/reopen
- DocumentId and identity ledger persist in the document file envelope.
- Session instance and runtime epoch do not persist.
- Reopen creates a new session instance and a new runtime epoch.
- Invalid, duplicate, or inconsistent identity ledger data blocks FDM evaluation and requires repair/migration; never silently regenerate tokens while retaining old authored TargetRefs.

### Save As and duplicate
- Path rename/move/ordinary Save As retains DocumentId.
- Explicit Save a Copy creates a new DocumentId and scopes all copied target references to it.
- The host must distinguish these commands explicitly; file path differences are not sufficient to infer duplicate semantics.

### Failed save
A failed file write must not publish a new saved baseline or clear dirty state. Persistence of identity envelope and artwork must be atomic at the document-file level, consistent with host file-save semantics.

---

## 11. Host mutation coverage map

The following table defines what each source family must report. Exact command names may change as the host evolves; the category obligation persists.

| Source family | Required transition/report |
|---|---|
| Draw/create/delete | Create/retire target transitions; geometry/availability |
| Duplicate/copy/paste/transfer | Fresh destination identities; complete remap; source unaffected |
| Path/raw point editing | Geometry and topology; preserve target identity for in-place edits |
| Shape rewrite/conversion | Explicit replacement map; geometry/topology and lifecycle |
| Group/ungroup | Group identity and all child identity outcomes; hierarchy |
| Transform/modify | Local transform; preserve target identity |
| Layer commands | Hierarchy, availability, affected descendant targets |
| Rig parenting | Hierarchy/effective transform for all affected descendants and keyframes |
| Timeline commands | Timeline structure and affected frame samples/target presence |
| Motion commands | Timeline sample/effective transform for affected frames |
| Native Warp commands | NativeWarpState; initial FDM support rejects live Warp |
| Bone/armature commands | ArmatureState; initial FDM support rejects unsupported skin targets |
| Scene operations | Scene identity/availability and complete identity remap |
| File/import/recovery | Document identity and ledger migration/remap; atomic publication |
| Undo/redo/history | Restore persistent ledger/payload with document; advance runtime epoch |
| Plugin/automation commands | Same transaction and invalidation obligations as native commands |
| FDM apply/bake | Explicit host transaction; source target transitions only if host command replaces source |

A coverage registry should list every mutating command ID and helper that can reach a mutable Document. CI should fail when a new mutating command is added without a declared mutation contract or an explicit reason it is non-document state.

---

## 12. Required conformance tests

These are future host implementation tests. They were not run in this design milestone.

| ID | Test | Required result |
|---|---|---|
| FDM-MUT-001 | Successful path edit | One committed receipt; epoch advances; target incarnation preserved |
| FDM-MUT-002 | Failed path edit | Document, ledger, StateId and epoch unchanged |
| FDM-MUT-003 | Invalid identity transition | Commit rejected; no partial artwork or ledger mutation |
| FDM-MUT-004 | Duplicate | Every duplicate gets fresh ElementId/incarnation |
| FDM-MUT-005 | Delete/recreate numeric ID | Old TargetRef never resolves to new incarnation |
| FDM-MUT-006 | Group/ungroup | Explicit identity outcome for every affected target |
| FDM-MUT-007 | Import/paste failure halfway | Atomic rollback; no leaked identities or partial FDM payload |
| FDM-MUT-008 | Gesture with many committed updates | Epoch advances each update; history may coalesce |
| FDM-MUT-009 | No-op command | No epoch advance |
| FDM-MUT-010 | Macro with independent commands | Receipt/epoch per committed command |
| FDM-MUT-011 | Atomic multi-command operation | One receipt after full commit |
| FDM-MUT-012 | Undo | Ledger/payload restored; epoch advances |
| FDM-MUT-013 | Redo | Ledger/payload restored; epoch advances again |
| FDM-MUT-014 | Undo returns same StateId | Old result still rejected by epoch |
| FDM-MUT-015 | Frame change only | Request stamp changes; old frame result rejected |
| FDM-MUT-016 | Scene/edit-context change | Request stamp changes; old result rejected |
| FDM-MUT-017 | Document replacement | Session instance changes; old result rejected |
| FDM-MUT-018 | Epoch overflow | Mutation fails closed; no wrap/saturation |
| FDM-MUT-019 | ElementId allocator exhaustion | Allocation fails; no duplicate ID |
| FDM-MUT-020 | Incarnation collision/generation failure | Transaction aborts or retries safely; no duplicate token |
| FDM-MUT-021 | Event delivery failure after commit | Receipt/epoch remains discoverable; cache cannot remain trusted |
| FDM-MUT-022 | Concurrent mutation at acceptance | Either result publishes before mutation or is rejected after mutation; never races between check and publish |
| FDM-MUT-023 | Unknown mutation category | Global invalidation; selective cache reuse disabled |
| FDM-MUT-024 | Native Asset Warp target | Initial adapter rejects unsupported target |
| FDM-MUT-025 | Armature/skin target | Initial adapter rejects unsupported target |
| FDM-MUT-026 | Plugin mutates document | Same mutation receipt and epoch rules apply |
| FDM-MUT-027 | Save/reopen | DocumentId/ledger persist; session instance/epoch renew |
| FDM-MUT-028 | Save a Copy | New DocumentId; internal TargetRefs correctly scoped |
| FDM-MUT-029 | Failed save | Dirty state and saved baseline remain correct |
| FDM-MUT-030 | Uninstrumented command introduced | Coverage test/registry check fails |
| FDM-MUT-031 | FDM apply/bake | Explicit host transaction; rollback atomic |
| FDM-MUT-032 | Stale cache after global unknown mutation | Cache entry is not reused |

---

## 13. Implementation sequencing and release gates

### Stage A — host mutation inventory
- Enumerate all command IDs and direct document mutation helpers.
- Classify document mutation vs UI-only mutation.
- Add a machine-checkable coverage registry.
- Gate: no unclassified document mutator.

### Stage B — identity transaction
- Add host-owned DocumentId and incarnation ledger.
- Make create/retire/replace/remap transitions transactional.
- Handle group/ungroup/conversion/import/scene operations explicitly.
- Gate: identity lifecycle and rollback tests pass.

### Stage C — global evaluation epoch
- Add session instance ID and monotonic evaluation epoch.
- Advance on every committed relevant mutation, undo/redo, document replacement, and external evaluation-input change.
- Include frame/scene/edit context in request stamps.
- Gate: stale result rejection and race tests pass.

### Stage D — immutable snapshot and synchronous evaluation
- Build one immutable Path2D snapshot with explicit coordinate space.
- Validate exact TargetRef and supported host feature set.
- Evaluate synchronously and accept only through the atomic predicate.
- Gate: snapshot consistency and output conformance tests pass.

### Stage E — cache and async (future)
- Cache only by complete stamp.
- Keep global epoch acceptance as final fence.
- Add cancellation and worker scheduling only after acceptance tests pass under concurrent mutations.
- Gate: deterministic concurrency tests, stale-result tests, and memory/cancellation tests pass.

Fine-grained revisions are not a release gate for the first integration and must not delay a correct global invalidation implementation.

---

## 14. Decisions and unresolved implementation choices

### Decisions made by this design
- Use a single transaction-aware mutation boundary.
- Use explicit target transitions, not inference.
- Make the identity ledger part of persistent document/history state.
- Advance a non-persistent monotonic runtime epoch for each committed relevant mutation, undo/redo, and session replacement.
- Use conservative global invalidation first.
- Include frame, scene, and edit context in the evaluation stamp.
- Require immutable snapshots and atomic result acceptance.
- Keep synchronous evaluation first; async remains deferred.
- Keep Native Asset Warp and Smart Controller/Form Deform evaluation independent.

### Choices intentionally deferred to implementation planning
- Exact Rust type names and ownership/borrowing strategy.
- Whether the transaction context is an explicit parameter, a command-scoped journal, or a draft-document builder.
- Exact DocumentId/IncarnationId generator implementation and persistence encoding.
- Whether the epoch is a u64 with checked increment or a wider opaque generation; either must fail closed on exhaustion.
- Canonical fingerprint algorithm and serialization format.
- Whether a coverage registry is generated from command metadata or maintained as a checked manifest.
- How to atomically coordinate document state, epoch publication, and cache invalidation under AniCraft's actual threading model.

These choices require a host-specific implementation RFC and tests. This design does not select an implementation based on unverified assumptions about AniCraft internals.

---

## 15. Next milestone

**FDM-06.7 — Mutation Boundary Implementation Readiness Review**

Before code changes:
1. Map every current AniCraft mutating command and direct mutable-document helper to the proposed boundary.
2. Identify operations that currently bypass the normal command transaction path.
3. Confirm how session state and epoch can be published atomically under the real host threading model.
4. Define exact identity transition maps for group/ungroup, shape conversion, scene duplication, and import.
5. Confirm history snapshot placement for the identity ledger and FDM extension envelope.
6. Turn FDM-MUT-001–032 into a host test plan with concrete source owners.
7. Review failure/rollback and event-delivery behavior.

**No AniCraft implementation should begin until this readiness review is accepted.** A separate explicit authorization is required before changing the host or FDM runtime.

---

## 16. Audit/design limitations

- This is a design document based on the FDM-06.5 source audit and pinned host commit, not a new exhaustive source audit.
- No runtime code, tests, benchmarks, or CI workflows were changed or executed for this milestone.
- The proposed contract must be reconciled against the current AniCraft source again before implementation, especially if the host head has advanced.
- The design does not claim that any proposed API, identity ledger, epoch, event journal, or acceptance gate already exists.
- No merge authorization is implied.
