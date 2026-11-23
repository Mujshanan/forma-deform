# FDM-06.6 — Host Mutation Invalidation Contract Design

**Status:** Design contract proposed; implementation gate remains closed  
**FDM baseline:** FDM-06.5, PR #67, head 2494aaaf5e4d55ecb19c13c31ea8b22c183e29e9  
**Identity design baseline:** FDM-06.4, PR #66, head 4942b81a6d0daa11f5d9adced43b3ad9286eabdf  
**Host:** Mujshanan/anicraft  
**Pinned host source:** 2df7775907d1458ce653e7e6314ec8a5d7d565f5  
**Scope:** Define the host-owned mutation, identity, invalidation, snapshot, and stale-result acceptance boundary required before FDM runtime implementation.  
**Out of scope:** AniCraft source changes, FDM runtime implementation, evaluator integration, plugin adapter implementation, native Asset Warp changes, and Smart Controller/FDM composition.

---

## 1. Executive decision

FDM must not observe document mutation by polling UI state, wrapping only selected commands, or maintaining revisions inside its own adapter. AniCraft must own one transaction-aware mutation boundary that covers every mutation path capable of changing target identity, source geometry, effective transform, hierarchy, timeline sampling, native Warp state, or FDM-authored data.

The contract has two layers:

1. **Persistent document transaction state:** authored FDM extension data and the target identity ledger participate atomically in the same document mutation, save/load, undo, redo, duplicate, import, and rollback semantics as the document data they describe.
2. **Non-restorable runtime freshness state:** a host-owned monotonic evaluation epoch and session-instance identity invalidate all work that could have observed an earlier document/evaluation state. These values are never restored from undo snapshots.

The first implementation MUST use **conservative document/session-wide invalidation** for every successful mutating transaction and every externally visible gesture preview. Fine-grained dependency revisions may be recorded for diagnostics and future optimization, but MUST NOT be relied upon to accept stale-sensitive output until exhaustive host mutation coverage is proven.

A command registry's mutates flag, a plugin callback, a UI event, or an FDM-side counter is not a sufficient mutation boundary.

**Implementation remains blocked.** This document defines the required contract and verification plan; it does not prove the host currently implements it.

## 2. Contract vocabulary

| Term | Meaning |
|---|---|
| Mutation transaction | One host-owned unit that either commits all document and FDM extension changes or rolls them all back |
| Mutation batch | The normalized description of changes made by one committed transaction or one externally visible preview update |
| Undo group | The user-visible history grouping; may contain several mutation batches |
| TargetRef | DocumentId + ElementId + IncarnationId |
| IncarnationId | Opaque host-issued identity for one lifetime of a target; not derived from ElementId, allocator state, frame, or array position |
| Dependency set | The source and evaluation inputs whose changes can alter an FDM result |
| Evaluation epoch | Monotonic runtime counter for the current Session; never restored from document history |
| EvaluationStamp | Immutable identity of the exact target snapshot and evaluation dependencies used to compute a result |
| Freshness acceptance | Host check performed immediately before a result becomes visible, cached as current, or is applied/baked |

## 3. Non-negotiable ownership rules

1. AniCraft owns DocumentId, ElementId allocation, target incarnation issuance, identity ledger mutation, transaction boundaries, mutation reporting, runtime epochs, source snapshots, cache lifetime, scheduling, stale-result rejection, undo/redo, persistence, and apply/bake commands.
2. FDM owns its authored deformation model, payload schema, migrations, deterministic evaluation, and a declaration of the inputs it consumes.
3. A target identity is never inferred from a matching numeric ElementId alone.
4. A dependency revision is never incremented only in the FDM adapter. The authoritative owner of the mutated host state must report the mutation.
5. Every mutation path must be covered, including commands, gestures, imports, frame transfer, nested editing, scripts, plugin-issued host commands, undo/redo, document replacement, and rollback.
6. Native Asset Warp remains host-owned. FDM cannot clear, rewrite, reinterpret, or implicitly compose with its state.
7. Smart Controller evaluation and FDM evaluation remain independent under CCC-01.5.
8. A result is never accepted because its task completed successfully. It is accepted only if its complete EvaluationStamp still matches the current host state.
9. No asynchronous FDM evaluation, production evaluator wiring, or host integration may begin before the implementation gate in §15 is satisfied.

## 4. Mutation boundary architecture

### 4.1 Required host seam

Introduce a host-owned mutation coordinator at the Session/document transaction boundary. The conceptual boundary is:

- Begin transaction and capture the persistent document/extension snapshot.
- Create a transaction-scoped MutationRecorder.
- Execute the command or gesture update using host mutation helpers that report identity and dependency changes.
- Validate structural invariants and identity-ledger consistency.
- Commit the document and extension atomically, or restore both on failure.
- Advance runtime freshness state as specified in §8.
- Publish one immutable MutationBatch for cache invalidation, diagnostics, and test assertions.

The exact Rust type names are implementation choices. The ownership and semantics in this document are normative.

### 4.2 No parallel mutation system

The coordinator is not an FDM-only wrapper and must not be bypassable by:
- direct command-module edits to Document, Scene, Timeline, Layer, Keyframe, Element, Shape, or the FDM extension;
- helper functions called from more than one command family;
- nested host commands dispatched by plugins;
- import, paste, duplicate, conversion, or frame-transfer helpers;
- gesture preview/update paths;
- undo/redo or document replacement.

All mutation-capable host helpers must either receive the active MutationRecorder/transaction context or be private behind an API that does. A mutation that cannot be classified still triggers conservative global invalidation; it must never silently retain a prior epoch.

### 4.3 Transaction wrapper fallback

During migration, the existing command transaction wrapper MUST compare a canonical persistent-state digest before and after every command classified as mutating.

- If a mutating command changes persistent state, the transaction is committed and the document-wide epoch advances even if detailed dependency events are missing.
- If a command classified as non-mutating changes persistent state, the host must emit a mutation-bypass diagnostic, perform global invalidation as a safety fallback, and fail a debug/conformance assertion.
- If a command reports detailed events that contradict the before/after state, the transaction must fail closed or commit with global invalidation and a diagnostic; it must not trust the incomplete fine-grained report.
- A state digest is a safety net and audit instrument, not a replacement for identity operations or a permanent substitute for complete mutation ownership.

The digest must cover persistent document state relevant to FDM, including nested group/symbol content, timelines, transforms, geometry, hierarchy, native Warp state, and the FDM extension. It must not depend on pointer identity, HashMap iteration order, renderer output, or non-deterministic serialization.

## 5. MutationBatch contract

Every successful transaction or externally visible preview update produces a normalized batch with at least:

| Field | Requirement |
|---|---|
| batch_id | Unique within the Session; monotonic or otherwise collision-safe |
| transaction_id | Identifies the host transaction or gesture |
| origin | Command, gesture, import, paste, script, plugin-host-command, undo, redo, document-replace, or recovery |
| phase | Preview, commit, rollback, undo, redo, or replace |
| identity_ops | Ordered create/preserve/retire/replace/remap outcomes |
| dependency_changes | Geometry, topology, transform, hierarchy, timeline, native-warp, resource, FDM-payload, or document-context changes |
| affected_targets | Exact TargetRefs where known; otherwise explicit whole-document scope |
| affected_scenes/layers/frames | Scope where known; unknown scope means whole document |
| before_state | StateId and canonical digest before the operation, where available |
| after_state | StateId and canonical digest after the operation, where available |
| invalidation_scope | Target, dependency subtree, scene, or document; v1 MUST resolve to document |
| diagnostics | Missing classification, unsupported transition, remap, fallback, or validation failure |

The batch is an audit record, not a second mutable source of truth. The persistent document and identity ledger remain authoritative.

### 5.1 Dependency change categories

- **Geometry:** path commands, control points, fill/line geometry, source shape replacement.
- **Topology:** point/segment insertion, deletion, split/merge, contour changes, connectivity changes.
- **Transform:** local transform, pivot, transform animation, evaluated transform.
- **Hierarchy:** layer rig-parent, group containment, owner changes, ancestor transform dependencies.
- **Timeline:** keyframe insert/remove/move/copy, frame shift, tween/easing/motion changes, exposure changes.
- **Native Warp:** native Warp presence, rest path, mesh, pins, weights, and host-generated warped outline.
- **Resource:** referenced bitmap, symbol, or other external resource if it becomes an allowed FDM dependency in a later scope.
- **FDM payload:** authored pins, bindings, constraints, settings, target mapping, schema migration, or FDM policy change.
- **Document context:** scene, edit context, document replacement, Save As/copy/duplicate identity transition.

Unknown category or affected scope MUST be represented as unknown and escalated to document-wide invalidation.

## 6. Identity lifecycle outcomes

The host must report one of these outcomes for each affected target:

| Outcome | Meaning | Required ledger action |
|---|---|---|
| Create | A new target lifetime begins | Allocate a fresh IncarnationId and register it atomically with the target's first persistent authored reference |
| Preserve | Same target lifetime; only its content/dependencies changed | Keep IncarnationId; record dependency changes |
| Retire | Target lifetime ends | Mark the exact incarnation retired; references remain dangling |
| Replace | Old target ends and a logically related new target begins | Retire old TargetRef and create a new TargetRef; continuity requires an explicit reviewed mapping |
| Remap | A target is moved/copied into a new identity namespace or owner context | Record old-to-new mapping and whether incarnation is preserved or newly issued according to the operation policy |

### 6.1 Rules for known AniCraft operations

- Ordinary in-place geometry and transform edits preserve target incarnation and report dependency changes.
- Duplicate and paste create new target incarnations, even if geometry is identical.
- Delete retires the exact incarnation. Recreating an element with the same numeric ElementId does not revive it.
- Grouping creates a group target; moved child elements retain their own incarnations but their containment/hierarchy dependencies change.
- Ungrouping retires the group target. Child incarnations are preserved unless host collision repair changes a child ElementId; a collision-remapped child receives a new incarnation and an explicit remap record.
- Convert-to-symbol creates a new stage instance target. Moved symbol children retain their identities, but their owner/edit-context changes. The initial main-scene Path2D FDM adapter does not evaluate nested symbol children.
- Convert-to-bitmap or any destructive shape rewrite must explicitly declare whether a source target is preserved, retired, or replaced. No identity continuity is inferred from output order.
- Frame copy/move operations must define whether the host element lifetime is preserved or a distinct target is created. FDM does not infer this from identical content.
- Scene/document duplication creates new document/scene identity scope and remaps every persistent target reference. It cannot copy the source DocumentId and incarnation namespace unchanged.
- Import into a document allocates new target identities and remaps or quarantines all extension references. Partial dangling references are forbidden.

### 6.2 Incarnation issuance and undo

Incarnation issuance must not be derived solely from Document.next_id, ElementId, StateId, or any counter restored by undo.

The host must use a collision-resistant opaque token source with collision checking. The production source must not roll back with document snapshots. Tests must inject a deterministic token source so exact outcomes can be asserted.

- Undoing a creation removes the created target and its ledger entry from the restored persistent snapshot, but does not rewind the token source.
- Creating a replacement after undo issues a fresh incarnation, even if the numeric ElementId is reused.
- Redoing the exact historical command restores the historical target and historical incarnation from its history snapshot/command record; it does not issue a new token.
- Undoing deletion restores the historical incarnation for that restored target.
- Failed transaction registration must roll back the target and ledger together; no partially registered target may survive.

## 7. Document identity and extension persistence prerequisites

FDM-06.4's open review items remain prerequisites and are not silently treated as accepted decisions.

### 7.1 DocumentId lifecycle

Resolve the contradictory eager/lazy rule with the minimal-impact policy:

- A legacy document without FDM extension remains without a DocumentId until the first persistent FDM extension or target identity registration is created.
- That first registration atomically creates DocumentId, extension envelope, and required identity-ledger entry.
- The DocumentId is preserved across ordinary save and Save As of the same logical document.
- An explicit Save a Copy / duplicate-document operation creates a new DocumentId and remaps all target identities and extension references.
- Importing content into another existing document preserves the destination DocumentId and issues destination target identities for imported targets.

If AniCraft chooses eager DocumentId for every document instead, it must be a root host document field independent of the optional FDM extension. The implementation cannot mix both policies.

### 7.2 Opaque payload preservation

AniCraft's explicit serialized document schema must retain unsupported FDM envelope/payload versions without lossy parse/reserialize.

The host must select and test a concrete representation (raw serialized JSON subtree, preserved byte slice, or equivalent lossless mechanism) and guarantee:
- load/save round-trips unknown payload content exactly;
- Save As preserves the same logical document's opaque payload;
- Save a Copy remaps identity-bearing fields only when the version is understood, otherwise it preserves the opaque subtree and marks identity-bearing operations unsupported or quarantined;
- failed migration leaves the original opaque payload intact;
- undo/redo restores the exact envelope version and content.

Exact preservation tests compare retained raw payload bytes or canonical raw subtree content, not merely successful deserialization.

## 8. Revision and evaluation-epoch semantics

### 8.1 Separate revision domains

| Value | Owner | Persistence | Undo behavior | Purpose |
|---|---|---|---|---|
| StateId | AniCraft history | History/document state identity | May return to historical state | Identifies persistent document content/history node |
| FDM payload revision | Persistent FDM extension | Saved with document | Restores with payload snapshot | Authored FDM data revision; not a freshness epoch |
| Target IncarnationId | Persistent identity ledger | Saved with document | Historical incarnation restored with history | Distinguishes target lifetimes |
| Session instance ID | AniCraft runtime | Runtime only | Never restored | Separates sessions/document replacements |
| Evaluation epoch | AniCraft runtime | Runtime only | Never restored; strictly advances | Conservative invalidation of all prior evaluation work |
| Fine-grained dependency revisions | AniCraft runtime or persistent host-owned metadata as explicitly designed | Implementation-defined | Must not regress in runtime | Future optimization only; not trusted for v1 acceptance |

### 8.2 Required epoch advancement

For v1, increment the monotonic EvaluationEpoch on:

1. Every successful persistent document mutation transaction.
2. Every externally visible gesture preview/update that changes evaluation-visible state.
3. Gesture cancellation/rollback after one or more preview states were visible.
4. Undo and redo, including restoration of an earlier StateId.
5. Document replacement, new/open, Save a Copy identity transition, and recovery/reload.
6. Any mutation-bypass fallback or uncertain mutation report.
7. Any host event that changes frame, scene, or edit context if those inputs are not represented independently in the EvaluationStamp. The required stamp does represent them independently, so this increment is still recommended for a single conservative invalidation stream.

A failed command that made no state visible and was rolled back before publication need not advance the epoch. If any intermediate state could have been observed by a renderer/evaluator, rollback MUST advance the epoch again.

Epoch overflow must fail closed by invalidating all work, replacing the Session instance ID, and restarting the epoch under a new instance identity. It must never wrap silently.

### 8.3 Gestures and history grouping

Undo grouping and evaluation invalidation are different concepts.

- A drag gesture may create one undo entry but many visible preview batches.
- Every preview batch advances the epoch.
- The final gesture commit may publish a final batch and epoch.
- Cancelling a gesture restores the pre-gesture persistent snapshot and advances the epoch to invalidate preview results.
- A multi-command transaction that is not externally visible until commit may advance once at commit.
- No batch may be hidden merely because it is coalesced into one undo step.

## 9. Immutable source snapshot and EvaluationStamp

The host creates an immutable target snapshot only after resolving the exact TargetRef and verifying that it is supported.

### 9.1 Target snapshot

Required contents:

- DocumentId, TargetRef, Session instance ID.
- Scene and explicit frame/time sample.
- Main-scene/edit-context identity; initial implementation rejects unsupported nested symbol/drawing edit contexts.
- Canonical source Path2D representation and source-coordinate-space declaration.
- Local transform and effective host transform as separate values.
- Relevant hierarchy/ancestor transform inputs.
- Timeline/keyframe/tween/motion sample inputs required to reproduce the target at the requested frame.
- Native Warp presence/state classification; initial supported targets must have no live native Warp.
- FDM authored payload snapshot and payload schema/policy version.
- Canonical dependency fingerprint and document/session epoch.

Snapshots are immutable values, not references into mutable Arc-backed host document objects. The FDM core cannot retain mutable host pointers or call back into the document while evaluating.

### 9.2 EvaluationStamp

The stamp must include at least:

| Field | Purpose |
|---|---|
| document_id | Persistent document namespace |
| session_instance_id | Prevents cross-session acceptance |
| evaluation_epoch | V1 conservative freshness token |
| state_id | Diagnostic/persistent state correlation |
| scene_id | Scene sampling identity |
| frame_sample | Exact frame/time input |
| edit_context_id | Main scene or supported editing context |
| target_ref | Exact document + element + incarnation |
| source_geometry_fingerprint | Canonical source path/topology identity |
| transform_fingerprint | Local and effective transform inputs |
| hierarchy_fingerprint | Parent chain and inherited transform inputs |
| timeline_fingerprint | Keyframe/tween/motion sampling inputs |
| native_warp_fingerprint | Explicit unsupported-state guard |
| fdm_payload_revision | Authored deformation inputs |
| fdm_policy_version | Evaluation semantics version |

The host may add fields. It may not omit a field because a particular UI action appears not to use it.

### 9.3 Canonical fingerprint requirements

Fingerprints must be deterministic across process runs and independent of pointer addresses, unordered collection iteration, renderer caches, and floating-point formatting accidents.

- Include canonical path commands, coordinates, fill/contour structure, and topology.
- Include exact transform values and all effective ancestor inputs.
- Include relevant sampled keyframe, tween and motion data.
- Include FDM authored data and policy version.
- Use a versioned canonicalization algorithm and cryptographic digest.
- A hash is a comparison accelerator, not a replacement for the explicit stamp or identity checks.
- If canonicalization fails or encounters unsupported state, snapshot creation fails closed.

## 10. Result acceptance predicate

Every asynchronous or delayed result must carry the exact EvaluationStamp used for its immutable input snapshot.

Immediately before inserting output into a current cache, publishing it to rendering, or applying/baking it, AniCraft must:

1. Verify Session instance ID matches.
2. Verify DocumentId matches.
3. Verify EvaluationEpoch matches.
4. Resolve TargetRef by exact ElementId and IncarnationId; reject missing, retired, replaced, or ambiguous targets.
5. Verify scene, frame sample, and edit context match.
6. Rebuild or retrieve the current authoritative dependency stamp.
7. Compare source geometry, transform, hierarchy, timeline, native Warp classification, FDM payload revision, and policy version.
8. Verify the target remains in the supported Path2D scope.
9. Verify the output is associated with the exact source snapshot and expected output contract.
10. Only then publish/cache/apply the result.

Any failed comparison means **discard the result**. Do not attempt to retarget it by matching ElementId, name, bounds, selection index, or geometry similarity. A fresh snapshot/evaluation may be scheduled.

The acceptance predicate is centralized in AniCraft and must be reused by render publication, cache insertion, preview publication, and apply/bake commands. A successful computation is not permission to mutate the document.

## 11. Cache and scheduling policy

- AniCraft owns cache keys, invalidation, cancellation, scheduling, and result acceptance.
- V1 cache key includes the complete EvaluationStamp or a collision-safe key derived from it.
- Every v1 mutation batch invalidates the document-wide FDM result namespace.
- In-flight work may finish, but stale results are rejected by §10.
- Cancellation is an optimization; acceptance validation is mandatory even when cancellation reports success.
- A cache hit is valid only if its complete stamp matches the current target snapshot.
- No cache entry is promoted to current merely because its TargetRef still resolves.
- Synchronous evaluation is the first implementation path. Async scheduling is disabled until the same acceptance predicate is tested against adversarial race cases.

## 12. Apply/bake and rollback

Apply/bake is a new host mutation transaction, not a side effect of evaluation.

Preconditions:
- Exact TargetRef still resolves.
- EvaluationStamp is current under §10.
- Source and output contracts are supported.
- Native Asset Warp is absent for the initial scope.
- All affected output elements and identity transitions are known before mutation.

Commit behavior:
- Apply/bake changes are recorded through the same MutationRecorder.
- Source-to-output identity outcomes are explicit.
- Document and FDM extension changes commit atomically.
- Any failure restores document and extension together.
- A successful apply/bake advances EvaluationEpoch and invalidates all prior results.
- Undo restores the authored source/extension snapshot and historical identity ledger; runtime epoch still advances.

Evaluation itself never mutates the document.

## 13. Mutation coverage matrix

The following source families from FDM-06.5 must be instrumented or explicitly declared out of scope. This is a coverage requirement, not evidence that the host currently emits these events.

| Mutation family | Required report | Conservative v1 behavior |
|---|---|---|
| Draw/create/delete/duplicate | Identity outcome + geometry/transform | Document invalidation |
| Path point/segment/contour edits | Geometry + topology | Document invalidation |
| Transform/pivot/align/distribute | Transform | Document invalidation |
| Group/ungroup | Identity + hierarchy + transform | Document invalidation |
| Shape rewrite/convert | Identity replace/remap + geometry | Document invalidation |
| Symbol/bitmap conversion | Identity + owner/resource context | Document invalidation; nested symbol FDM unsupported |
| Paste/import | Identity remap + resource + hierarchy | Document invalidation |
| Scene/document duplicate | DocumentId + full graph remap | Document invalidation |
| Keyframe/frame insert/remove/move/copy | Timeline + identity semantics | Document invalidation |
| Tween/easing/MotionTween edit | Timeline + evaluated transform | Document invalidation |
| Layer parent/rig-parent/move | Hierarchy + effective transform | Document invalidation |
| Native Asset Warp edit | Native-Warp state | Reject unsupported target; document invalidation |
| Armature/skin bind/reshape | Unsupported deformation state | Reject unsupported target; document invalidation |
| Nested symbol/drawing edit | Edit-context identity | Reject in initial scope; document invalidation |
| Plugin-issued host command | Same reports as native command | Document invalidation |
| Scripted document mutation | Same reports as native command | Document invalidation |
| Gesture preview/commit/cancel | Preview/commit/rollback phase | Epoch per visible preview |
| Undo/redo | Historical identity restore + runtime epoch | Document invalidation |
| Open/new/replace/recovery | Session/document identity | Replace session identity and invalidate |
| Save As / Save a Copy | Document identity policy | Preserve for same logical document; new ID/remap for copy |
| Failed transaction | Atomic rollback + diagnostic | Invalidate if intermediate state was observable |

No command family is exempt merely because it is not currently used by the FDM UI.

## 14. Conformance and verification plan

These tests are required before the implementation gate can open. They must run against the actual AniCraft host mutation boundary, not a mock MutationRecorder alone.

### 14.1 Transaction and bypass tests

| ID | Scenario | Required assertion |
|---|---|---|
| FDM-MUT-001 | Successful mutating command | One committed batch; document epoch advances |
| FDM-MUT-002 | Mutating command with missing detailed report | Global invalidation; diagnostic; no stale acceptance |
| FDM-MUT-003 | Non-mutating command changes persistent state | Bypass detected; global invalidation; conformance failure |
| FDM-MUT-004 | Failed command before publication | Document and extension rollback atomically |
| FDM-MUT-005 | Failed command after observable preview | Rollback restores state and advances epoch |
| FDM-MUT-006 | Nested plugin-issued host command | Same coordinator and recorder used; no nested bypass |
| FDM-MUT-007 | Script/helper mutation path | Same mutation boundary; no untracked write |
| FDM-MUT-008 | Multiple edits in one transaction | One atomic commit; complete aggregated report |
| FDM-MUT-009 | Mutation report contradicts state digest | Fail closed or global fallback with diagnostic |

### 14.2 Identity and history tests

| ID | Scenario | Required assertion |
|---|---|---|
| FDM-MUT-010 | Create then undo then create with reused ElementId | New incarnation differs from undone target |
| FDM-MUT-011 | Redo exact creation | Historical incarnation restored |
| FDM-MUT-012 | Undo deletion | Historical target incarnation restored |
| FDM-MUT-013 | Duplicate/paste | Fresh identity for every new target |
| FDM-MUT-014 | Group/ungroup | Child identity preservation and collision remap match §6 |
| FDM-MUT-015 | Import/cross-document paste | Destination identities and all references remapped atomically |
| FDM-MUT-016 | Save As vs Save a Copy | DocumentId policy in §7 is enforced |
| FDM-MUT-017 | Unknown payload version | Opaque payload exact-preserved across load/save and failed migration |

### 14.3 Epoch and evaluation tests

| ID | Scenario | Required assertion |
|---|---|---|
| FDM-MUT-018 | Geometry/transform/hierarchy/timeline mutation | Epoch advances; previous result rejected |
| FDM-MUT-019 | Frame change without document mutation | Stamp changes; previous frame result rejected |
| FDM-MUT-020 | Gesture with multiple visible previews | Epoch advances for each visible preview; undo history remains one gesture |
| FDM-MUT-021 | Gesture cancellation | Pre-gesture state restored; preview result rejected |
| FDM-MUT-022 | Undo to prior StateId | Epoch still advances; pre-undo result rejected |
| FDM-MUT-023 | Document replacement with pending work | Session identity changes; all prior work rejected |
| FDM-MUT-024 | Completion races with mutation | Acceptance check rejects result completed against old stamp |
| FDM-MUT-025 | Cancellation races with completion | Acceptance check rejects stale result even if cancellation is late |
| FDM-MUT-026 | Target deleted/recreated with same ElementId | Old TargetRef rejected |
| FDM-MUT-027 | Ancestor rig-parent changes | Descendant effective-transform stamp changes |
| FDM-MUT-028 | Native Warp added/edited | Target rejected as unsupported; no FDM output accepted |
| FDM-MUT-029 | Cache hit after any global mutation | Old cache entry not current |
| FDM-MUT-030 | Epoch overflow simulation | Session identity rotates; no silent wrap |

### 14.4 Coverage proof

In addition to scenario tests, the host must produce a machine-reviewable mutation-path inventory:
- every command registered as mutating;
- every direct mutation helper and its owning transaction context;
- every gesture preview/update/rollback entry point;
- import, paste, conversion, frame transfer, scene duplication and nested-edit paths;
- undo, redo, open/new, recovery and document replacement;
- plugin/script paths that can invoke host mutations.

A test or static audit must fail when a new mutation-capable path is added without a declared coordinator/recorder route. A manual checklist alone is insufficient.

## 15. Implementation gate — conditions to unblock FDM

FDM runtime implementation remains blocked until all conditions below are evidenced in the host repository and reviewed:

1. **Identity contract resolved:** FDM-06.4's DocumentId creation and opaque payload preservation review comments are resolved in an accepted host design.
2. **Mutation coordinator implemented:** all persistent mutation paths are transaction-owned and report identity/dependency outcomes, or are covered by the conservative digest/global-invalidation fallback.
3. **Atomic ledger:** target identity ledger and FDM extension participate atomically in save/load, undo/redo, duplicate/import, rollback and recovery.
4. **Monotonic freshness:** session instance and evaluation epoch never restore or wrap silently; all required events advance them.
5. **Immutable snapshot:** host can create an immutable, versioned Path2D target snapshot with full EvaluationStamp and explicit unsupported-state classification.
6. **Central acceptance predicate:** all result publication/cache/apply paths validate the complete stamp immediately before acceptance.
7. **Coverage tests pass:** all FDM-MUT-001 through FDM-MUT-030 plus the mutation-path coverage proof pass in AniCraft CI.
8. **Adversarial race tests pass:** stale completion, cancellation, undo/redo, gesture rollback, target reuse and document replacement cases pass.
9. **Source baseline re-audited:** implementation is reviewed against the exact AniCraft commit that contains the boundary; this pinned audit SHA alone is not proof of later implementation.
10. **Explicit review authorization:** the user approves the completed host boundary and separately authorizes the next implementation milestone.

A design document, passing Forma Deform documentation CI, or a mocked contract test does not satisfy these conditions.

## 16. Decisions and unresolved implementation choices

### Normative decisions made here

- V1 invalidation is document/session-wide.
- Runtime epoch and session instance identity are not undoable document state.
- Identity ledger and authored extension are undoable persistent document state.
- Every visible gesture preview invalidates immediately; undo grouping does not coalesce epochs.
- Exact TargetRef and full EvaluationStamp are mandatory.
- Async completion must pass centralized freshness acceptance.
- Native Warp and armature/skin are rejected for initial FDM Path2D support.
- Unknown mutation scope escalates to whole-document invalidation.
- No FDM adapter owns or invents host revisions.

### Choices that AniCraft must implement and evidence

- Concrete Rust API/type layout for MutationCoordinator, MutationRecorder and MutationBatch.
- Concrete deterministic canonicalization/fingerprint encoding and version.
- Concrete opaque payload preservation mechanism compatible with DocumentFile serialization.
- Concrete collision-resistant IncarnationId generator and deterministic test injection.
- Exact definition of Save As versus Save a Copy in the host file workflow.
- Exact mutation-path static/instrumentation strategy used by CI.

These are implementation choices within the normative contract; they are not permission to weaken its guarantees.

## 17. Explicit non-goals and preserved boundaries

- No changes to native Asset Warp algorithms, data, UI, cache, or rendering.
- No Smart Controller/FDM result composition; CCC-01.5 independence remains in force.
- No FDM plugin SDK or legacy adapter as an alternate mutation boundary.
- No async evaluator implementation in this milestone.
- No production FDM output integration, apply/bake implementation, or host code changes.
- No claim that the host currently satisfies any conformance case.
- No PR merge or implementation authorization is implied.

## 18. Recommended next step

**FDM-06.7 — Mutation Boundary Implementation Readiness Review**

Before any AniCraft code is changed, review this contract against the exact current host branch and produce:
- a complete source-level mutation entry-point inventory;
- a per-module adoption map for the coordinator/recorder;
- a migration sequence that preserves existing command/gesture/undo semantics;
- the concrete fingerprint and incarnation token design;
- a test harness plan proving mutation-path coverage.

Only after that review should the user decide whether to authorize host implementation. The FDM implementation gate in §15 remains closed until the implemented boundary and its conformance tests pass.
