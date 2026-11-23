# FDM-06.12 — Mutation Boundary Implementation Plan

**Status:** Planning artifact only; no runtime implementation authorized  
**Parent:** FDM-06.11, PR #76, head `d5c752c8a1e5d8e05f4976605cdb5a62e1d7a3f3`  
**Host baseline to revalidate:** AniCraft `Mujshanan/anicraft` commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Purpose:** Turn the FDM-06.6–06.11 mutation-boundary, identity, ownership, and persistence contracts into a gated, testable host implementation sequence.  
**Change policy:** This document does not change AniCraft or FDM runtime code, does not close S0, does not authorize S1 or later implementation, and does not authorize merging any PR.

---

## 1. Executive decision

The implementation must be split into two distinct authorizations:

1. **S0 closure work:** finish the evidence and source-level verification gaps recorded by FDM-06.11. This is still outstanding and is not runtime implementation.
2. **Host mutation-boundary implementation:** after S0 closure, after the FDM-06.6 mutation/invalidation boundary has been implemented and verified, and only after explicit user authorization.

A plan or passing documentation CI is not evidence that the runtime boundary exists. No FDM evaluator, native host integration, production result application, or adapter-based bypass may be introduced by this plan.

### Non-negotiable constraints

- AniCraft Session is the sole mutation authority for its live document state.
- All persistent writes must occur within one explicit transaction protocol.
- Workers receive immutable snapshots and return immutable results/messages; they never mutate Session or Document.
- Runtime identity and evaluation freshness are distinct from undoable persistent state.
- Undo/redo restores authored persistent state but never rewinds runtime freshness counters.
- The first invalidation policy is conservative and document/session-wide.
- Native Asset Warp behavior remains unchanged and host-owned.
- Smart Controller and Form Deform remain independent under CCC-01.5.
- First FDM vertical slice remains synchronous and Path2D-only.
- No asynchronous evaluation until freshness acceptance is proven.
- No bitmap, text, symbol, group-wrapper, nested-symbol, armature, or unsupported nonlinear dependency scope is inferred.
- No merge, PR closure, or release authorization is implied.

## 2. Entry gates and dependency order

| Gate | Required evidence | Status at plan creation |
|---|---|---|
| G0 — Source baseline | Revalidate AniCraft main SHA, tree completeness, source hashes, toolchain and workspace state | Must refresh before work |
| G1 — S0 command/effect registry | Every registered command and non-command mutator classified by actual effects, transaction owner, identity effects and tests | Open |
| G2 — S0 ownership/consumer proof | Owner-thread dispatch invariant selected; workspace consumers and public API break surface compiler-inventoried | Open |
| G3 — S0 CI baseline | Exact pinned host commit has recorded Windows and repository CI outcomes, or explicit reproducible baseline exception | Open |
| G4 — FDM-06.6 boundary | Mutation/invalidation contract implemented and verified against its conformance suite | Hard blocker |
| G5 — Authorization | User explicitly authorizes the specific implementation stage and repository/branch scope | Not granted by this plan |

**No S1 runtime implementation starts until G0–G5 are satisfied.** If the host baseline advances, source inventories and plans must be reconciled against the new SHA before implementation.

## 3. Phase A — Close S0 before runtime work

### A1. Normalize the command registry

Produce a machine-readable registry generated from the actual `engine::commands()` registration graph, not a hand-maintained copy of the FDM-06.11 table.

For every command ID, record:

- source module, function/spec location, and source hash;
- declared `mutates` flag (informational only);
- direct persistent writes and indirect helper writes;
- Session/context changes that affect evaluation but not persistent artwork;
- filesystem, preference, process, network, or other external side effects;
- settlement/finalization calls reached before and after the command body;
- transaction owner and whether nested execution is possible;
- identity transition classes (create, preserve, retire, replace, remap, restore);
- affected dependency categories and invalidation scope;
- existing tests, missing tests, and planned regression test IDs.

Classify each entry as `document-persistent`, `evaluation-context`, `external-side-effect`, `read-only`, or a justified combination. A false `mutates` flag must never imply read-only.

### A2. Audit non-command mutation routes

Build a second registry for mutation entry points outside registered command bodies, including:

- Session settlement and gesture lifecycle;
- cut healing and floating-element settlement;
- rig/motion auto-key finalization;
- import, paste, transfer, and document replacement;
- history restore, undo/redo, recovery, and migration;
- plugin nested commands;
- public Document mutators and mutable field access;
- autosave/recovery snapshot and file persistence boundaries.

Every persistent write must map to exactly one transaction owner. Any unclassified write blocks S0 closure.

### A3. Prove the owner/consumer migration surface

Use compiler-backed searches and builds to enumerate all workspace consumers of:

- `Session.doc` and public Session fields;
- public mutable Document fields and methods;
- `CommandSpec.run` and `RunFn`;
- public engine re-exports and CLI/MCP embedding APIs;
- plugin SessionHost and nested command interfaces;
- autosave, recovery, export, render, history, and editor panels.

The output must include a before/after migration table and an explicit compatibility decision for downstream users. Do not assume a textual search is exhaustive.

### A4. Establish exact baseline CI

On the revalidated AniCraft commit, record:

- exact commit SHA and recursive tree SHA;
- Windows toolchain/runner details;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- configured cargo-deny checks;
- failures, flaky tests, skipped tests, and known environment limitations.

If CI cannot be run on the exact baseline, document the blocker and an approved reproducible alternative. Do not label the baseline green without exact evidence.

### A5. S0 exit review

S0 may close only when:

1. every registered command ID is classified at body/effect level;
2. every non-command persistent mutation route is owned and classified;
3. every persistent write has a transaction boundary;
4. context-only and external side effects are separately identified;
5. workspace consumer migration is compiler-proven;
6. exact CI baseline evidence is recorded;
7. no unresolved bypass is hidden behind a `mutates` flag or helper abstraction.

Deliverable: a reviewed S0 closure report and machine-readable registry. A green documentation build alone is insufficient.

## 4. Phase B — Implement the host-owned mutation foundation

This phase is conditional on G0–G5 and a separately authorized implementation request.

### B1. Introduce the persistent state bundle

Define a single host-owned persistent bundle containing:

- artwork/document content;
- persistent DocumentId policy;
- target-incarnation ledger;
- versioned FDM extension payload;
- any metadata required to preserve unknown extension payloads losslessly.

Keep runtime-only values outside the persistent bundle:

- SessionInstanceId;
- checked EvaluationEpoch;
- context generation;
- pending evaluation tokens and cache state.

Acceptance criteria:

- save/load round-trips supported payloads;
- unsupported/unknown extension versions survive load/save without lossy reserialization;
- duplicate, Save As, Save Copy, import, and recovery semantics are explicit and tested;
- failed decode/migration leaves the live document unchanged.

### B2. Establish exclusive Session mutation authority

Replace unrestricted mutable access with a capability-scoped transaction API.

Required properties:

- owner-thread/owner-context assertion at transaction entry;
- private mutation draft or equivalent exclusive mutable capability;
- no public route to invoke arbitrary `CommandSpec.run` against live Session;
- immutable read access for UI and integrations;
- workers cannot receive mutable Session/Document references;
- nested plugin commands join the current outer transaction rather than opening an independent transaction.

Migrate consumers incrementally, using compiler errors as the authoritative in-repository inventory. Preserve read behavior while moving writes behind the transaction capability.

### B3. Centralize all mutation and settlement

One transaction lifecycle must cover command execution, nested commands, gestures, cut/floating settlement, rig/motion settlement, import/transfer, undo/redo, recovery, and document replacement.

Transaction sequence:

1. assert owner and acquire exclusive mutation capability;
2. capture the pre-state needed for rollback/history;
3. execute requested operation and nested operations;
4. run required settlement/finalization within the same transaction;
5. validate invariants and identity transitions;
6. commit persistent state and history atomically;
7. advance runtime freshness/context counters as required;
8. publish invalidation notifications only after commit.

On failure, restore the full persistent bundle and publish no committed-state notification. Runtime freshness counters remain monotonic even when a transaction rolls back.

External side effects must use explicit preflight/commit or post-commit protocols; do not pretend filesystem/process operations are automatically rolled back with in-memory state.

### B4. Add checked identity allocation and transition ledger

Centralize allocation and identity transitions. Use checked arithmetic; exhaustion must fail explicitly without wrapping or saturating into a reusable identity.

For each target operation, define and test:

- create: allocate a fresh incarnation;
- preserve: retain the same incarnation;
- retire: permanently invalidate the prior incarnation;
- replace: retire old and allocate new;
- remap: create destination identities and rewrite all internal references;
- undo: restore the historical persistent identity state;
- redo: restore the exact historical result of that command;
- undo-create then new-create: allocate a fresh incarnation even if a numeric ElementId is reused.

No target reference may cross DocumentId boundaries. Every identity-bearing nested reference must be included in duplicate/import/paste/scene/symbol remapping.

### B5. Integrate history, save, recovery, and autosave

History snapshots must capture the complete persistent bundle, not artwork alone. Update memory accounting to include ledger and extension payload ownership.

Autosave/recovery workers receive immutable bundle snapshots plus a matching persistence version. A completion may mark the live state saved only if its snapshot version still matches the intended saved state.

Undo/redo restores persistent bundle content, then advances runtime evaluation freshness. Runtime epochs are never restored from history.

### B6. Add evaluation context and conservative invalidation

Introduce checked, monotonic runtime counters independent of document StateId:

- SessionInstanceId changes on live Session replacement;
- EvaluationEpoch advances on every relevant committed mutation, visible gesture preview, rollback that changes visible state, undo/redo, and document replacement;
- ContextGeneration advances on frame, scene, symbol/edit context, and other evaluation-context changes.

Start with document/session-wide invalidation. Do not introduce per-target optimization until the conservative policy is proven complete.

## 5. Phase C — Snapshot and result acceptance boundary

This phase implements host freshness infrastructure only; it does not itself authorize FDM runtime wiring.

### C1. Immutable evaluation snapshot

Create an immutable snapshot with a stable stamp covering at minimum:

- DocumentId and SessionInstanceId;
- EvaluationEpoch and ContextGeneration;
- persistent state/history identity;
- scene, frame, and edit-context fingerprint;
- target ElementId and IncarnationId;
- source geometry and relevant transform/hierarchy/timeline fingerprints;
- native Asset Warp state marker;
- FDM payload revision and policy/version identifiers.

Only include dependencies that the declared supported vertical slice actually consumes. Unsupported dependencies must be rejected or explicitly marked unsupported, not silently omitted.

### C2. Centralized acceptance predicate

One host-owned predicate must guard every result consumer: preview/render cache, visible display application, and explicit apply/bake.

A result is acceptable only if:

- its DocumentId and SessionInstanceId still match;
- its target incarnation is still live;
- its epoch and context generation match current state;
- its payload/policy and dependency fingerprints match;
- the target remains eligible for the supported operation;
- no newer request supersedes it.

Reject stale results without mutating authored state or replacing a newer cache result. The predicate must be tested independently of the render service's existing queue behavior.

### C3. Synchronous-first policy

First prove snapshot construction and acceptance synchronously on the owner thread. Add asynchronous scheduling only in a later, separately reviewed scope after stale-result rejection is demonstrated under adversarial tests. Workers remain pure snapshot consumers.

## 6. Phase D — Verification and conformance

### D1. Transaction and ownership tests

Cover:

- direct command and helper writes;
- nested plugin commands;
- gesture preview/commit/cancel;
- cut/floating/rig/motion settlement;
- import/paste/transfer;
- rollback after partial mutation;
- owner-thread violation;
- attempted mutable access outside transaction;
- external side-effect failure boundaries.

### D2. Identity and persistence tests

Cover create/edit/delete/recreate, duplicate/paste, group/ungroup, merge/split, scene duplication, symbol conversion, cross-document transfer, Save As/Save Copy, unknown extension payload preservation, allocator exhaustion, undo/redo identity restoration, and recovery.

### D3. Freshness and invalidation tests

Cover frame/scene/edit-context changes, every document mutation family, visible gestures, rollback, undo/redo, document replacement, target retirement/recreation, delayed old results, out-of-order completions, and superseded requests.

### D4. Windows verification

Every implementation stage must pass the exact repository Windows CI suite and its focused tests. Record exact commit, workflow run, jobs, and conclusions. A local Linux pass or documentation CI pass does not substitute for mandatory Windows verification.

## 7. Recommended implementation PR sequence

| Stage | Proposed PR scope | Main output | Depends on |
|---|---|---|---|
| P0 | S0 closure evidence | Complete effect registry, consumer map, exact CI baseline, closure review | G0–G3 |
| P1 | Persistent state and identity types | Versioned bundle, opaque extension preservation, checked identity allocator | G0–G5 |
| P2 | Session mutation capability | Owner assertion, private transaction draft, immutable accessors | P1 |
| P3 | Transaction unification | Nested commands, settlement, gestures, import and rollback in one protocol | P2 |
| P4 | History and persistence | Bundle-aware history, save/load, autosave/recovery, Save As/Copy semantics | P1–P3 |
| P5 | Mutation/effect registry enforcement | Generated registry, coverage guard, no unclassified mutation route | P2–P4 |
| P6 | Runtime freshness | SessionInstanceId, checked epoch, context generation, conservative invalidation | P2–P5 |
| P7 | Snapshot/result acceptance | Immutable stamps and single acceptance predicate | P6 |
| P8 | FDM vertical-slice integration proposal | Path2D synchronous integration design and test evidence | P0–P7 plus separate authorization |

Each PR must be narrowly scoped, independently reviewable, Windows-tested, and based on a refreshed host baseline. Do not combine several stages into a large unreviewable change. Do not create a PR for a later stage before its dependencies are verified.

## 8. Rollout and failure policy

- Prefer additive internal APIs first; deprecate public mutable APIs only after consumer migration is measured.
- If a consumer cannot migrate without bypassing transaction ownership, stop and revise the boundary rather than adding an escape hatch.
- If unknown extension data cannot be preserved losslessly, do not enable writes that would destroy it.
- If any persistent write cannot be assigned to one transaction, the transaction stage is not complete.
- If identity semantics are ambiguous for an operation, reject FDM targeting for that operation until semantics are specified.
- If stale-result acceptance cannot be proven, keep evaluation synchronous and do not expose async results.
- If Windows CI fails, classify the failure against the recorded baseline; do not waive it by assumption.
- Any change to native Asset Warp or Smart Controller/FDM composition requires a separate approved scope.

## 9. Explicit exclusions

This plan does not authorize:

- FDM evaluator or deformation algorithm implementation;
- AniCraft FDM host integration or production evaluator wiring;
- plugin adapter or temporary evaluator bypasses;
- Native Asset Warp replacement or composition;
- Smart Controller/FDM output composition;
- async evaluation;
- apply/bake implementation;
- non-Path2D target support;
- ABI v1 retirement;
- PR merge, release, or deployment.

## 10. Exit criteria

The host mutation-boundary implementation may be considered ready for a separately proposed FDM integration only when:

1. S0 has been formally closed with evidence;
2. FDM-06.6 mutation/invalidation boundary is implemented and verified;
3. every persistent write is transaction-owned;
4. identity transitions and history persistence pass conformance tests;
5. owner-thread enforcement is active;
6. conservative invalidation and stale-result acceptance are proven;
7. mandatory Windows CI passes on the exact implementation head;
8. no unsupported target/dependency is silently accepted;
9. the user separately authorizes the next FDM implementation scope.

**Current disposition:** Planning only. S0 remains open. FDM runtime implementation remains blocked. No runtime source changes are included in this deliverable.
