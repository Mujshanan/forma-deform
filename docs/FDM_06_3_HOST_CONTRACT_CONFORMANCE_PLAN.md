# FDM-06.3 — Native AniCraft Host Contract Conformance Plan

**Status:** Planning baseline; implementation not authorized by this document  
**Milestone:** FDM-06.3 — Host Contract Conformance Planning  
**Depends on:** FDM-06.2 native integration contract; FDM-05.4 output contract; M1 target geometry, evaluation lifecycle and conformance matrix; CCC-01.5  
**Host repository:** `Mujshanan/anicraft` (user-owned fork; integration target)  
**Pinned host revision:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5` (`main`, observed 2026-11-23)  
**Scope:** Source-revision-pinned conformance plan. No AniCraft code changes, FDM runtime changes, host API changes, or merges are authorized by this document.

## 1. Purpose and release rule

FDM-06.2 defines the logical host/FDM boundary. FDM-06.3 turns that boundary into an executable, source-pinned conformance plan: what must be proven in AniCraft, where the proof belongs, which existing host surfaces can be reused, what must be added, and in what order.

A passing `cargo test --workspace` in `forma-deform` proves only the FDM-side contracts. It does not prove host persistence, identity, evaluation placement, freshness, transactionality, rendering parity or export parity.

**No host implementation milestone may begin until the pinned source has been rechecked, the host-side design choices listed in §8 are resolved, and a separate implementation authorization is given.** A changed host source revision invalidates source-specific assumptions until the affected audit rows are revalidated.

## 2. Pinned source baseline

### 2.1 Repository identity

- Repository: [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft)
- Repository is a fork of: [storytold/anicraft](https://github.com/storytold/anicraft)
- Default branch at audit: `main`
- Commit: [`2df7775907d1458ce653e7e6314ec8a5d7d565f5`](https://github.com/Mujshanan/anicraft/commit/2df7775907d1458ce653e7e6314ec8a5d7d565f5)
- Commit message: `Merge pull request #525 from storytold/docs/principal-review`
- Commit date: 2026-11-21
- Tree: `80071fb3f714944e49608668871d4cf53a6159a5`

This SHA is the baseline for this plan, not a claim that the host is frozen. Before implementation, compare the current host head with this SHA and repeat the source-diff review for all affected surfaces.

### 2.2 Inspected source files and blob identities

| Host source | Blob SHA at pinned commit | Conformance relevance |
|---|---|---|
| `crates/doc/src/lib.rs` | `59559a0b4599fd0ec34c720004b42ceda3080883` | Document, DocumentFile deserialization, Shape, Element and IDs |
| `crates/doc/src/eval.rs` | `53e3e7489ebbbca76a436211d822168ecab0fb2e` | Pure timeline/frame evaluation and DisplayList construction |
| `crates/engine/src/lib.rs` | `de144ad9996bf1c29c52c28135b8f00a359ec338` | Session, revision/state, command execution and display-list cache |
| `crates/engine/src/history.rs` | `204847cfb538942b495ef58e21a90d62fbfa1ee4` | Snapshot-based undo/redo and StateId |
| `crates/engine/src/render_service.rs` | `abf5c53b83a5d87ef6d6eb1811fb4b07dbabe676` | FrameJob, tile/frame scheduling and render cache |
| `crates/engine/src/cmds_plugin.rs` | `48cea8e7a94d01f7e9733deac6a1ddb1ef7f8440` | Existing command-plugin bridge and atomic plugin.run |
| `crates/engine/src/plugin_export.rs` | `2604e904f85ca7a05c5499f787eb9c2a4401f214` | Exporter frame access and export snapshot boundary |

The blob SHAs are recorded to make the audit reproducible even if branch names move. The source is linked at the pinned commit in §10.

## 3. Source-verified host facts

The following are verified at the pinned commit and are the basis for the conformance plan.

### 3.1 Document persistence and target model

- `Document` is serializable data and uses `#[serde(from = "DocumentFile")]` for deserialization.
- `DocumentFile` explicitly enumerates supported serialized fields and performs compatibility normalization. There is no generic plugin extension-data map in the inspected schema.
- `Shape` owns an `Arc<Path>` plus optional `Arc<warp::Warp>`. The source comment describes `Shape.path` as the bent outline when Asset Warp is present, with the warp retaining the outline as drawn.
- `Element` has a document-local `ElementId`, transform and `ElementKind`.
- `ElementId`, `LayerId` and `SymbolId` are distinct numeric ID types.
- `Document.next_id` allocates IDs for layers, elements and symbols.

**Conformance implication:** FDM authored data cannot be persisted by assuming a plugin-owned metadata slot exists. A host-owned extension field/schema or an explicitly host-owned sidecar strategy must be designed; FDM-06.2 requires authoritative document persistence, so a sidecar is not the default solution. Target generation and geometry revisions are not established by numeric IDs alone.

### 3.2 Evaluation and DisplayList

- `anicraft_doc::evaluate` maps a document and movie frame to a `DisplayList`.
- `evaluate_scene` builds the scene's timeline in stage space and is documented as the stage view used by picking and bounds.
- `DisplayItem` carries a path and local-to-stage transform together with paint, stroke and picking metadata.
- Document evaluation is a pure function; it does not mutate authored document data during playback.

**Conformance implication:** FDM must be integrated into the host's shared evaluated geometry, not only the rasterizer or a stage overlay. The implementation must reconcile scene, symbol/timeline and export evaluation paths; the public evaluator entry points must not accidentally bypass FDM.

### 3.3 Session cache and render scheduling

- `Session` owns the document, current scene/frame, a monotonically bumped session revision, a document `StateId`, and a cached `DisplayList`.
- The display-list cache key includes state, scene, frame and open-symbol editing context.
- `FrameJob` carries a cloned `Document`, frame, view and rendering context.
- `RenderService` schedules tile/frame jobs and evaluates a scene for rendering.

**Conformance implication:** The existing document state/cache model is useful for conservative invalidation, but does not by itself express FDM's per-target incarnation, geometry/topology, transform, hierarchy, model, composition or upstream-output dependencies. FDM acceptance freshness must be explicit before the result enters a cached evaluated view.

### 3.4 Commands and history

- Mutating commands pass through `Session::execute`, which snapshots before execution, clears the display-list cache and restores on failure.
- Undo/redo history stores document snapshots with state identity.
- Plugin command execution is one host command and one undo step; plugin code invokes allowed host commands through the bridge.

**Conformance implication:** FDM apply/bake should be a native host command participating in the same transaction/history model. A preview result must not mutate the document. The plugin command bridge is not the live evaluation API.

### 3.5 Plugin and export boundaries

- `plugin.run` is a fresh command-plugin run, not a continuously invoked frame evaluator.
- The exporter receives document/frame access through a separate `FrameHost` interface and evaluates requested frames from a document snapshot.

**Conformance implication:** Do not implement live FDM evaluation by repeatedly invoking a command plugin. Export must consume the same accepted FDM evaluated geometry as the editor/player path; otherwise output parity is not established.

## 4. Conformance scope and explicit exclusions

### In scope

1. Persistent FDM extension data and deterministic migration.
2. Host target identity, incarnation/generation and typed capability checks.
3. Atomic immutable Path2D snapshot capture with coordinate and dependency stamps.
4. Deterministic FDM request/result exchange.
5. One shared evaluated-geometry insertion path.
6. Cache invalidation and stale-result rejection.
7. Native host transaction, rollback, undo/redo and retained bake provenance.
8. Editor, hit-test, bounds, player and export parity.
9. Save/reopen, duplicate, import and target deletion/recreation behavior.

### Explicitly out of scope for the first host slice

- Bitmap deformation.
- Text, symbol-instance and group deformation.
- Nonlinear propagation through layer parenting or nested symbols.
- FDM + native Asset Warp composition.
- Smart Controller + Form Deform composition (CCC-01.5 remains unchanged).
- Multi-deformer composition beyond already accepted, explicitly versioned graph semantics.
- Asynchronous scheduling before synchronous freshness and publication semantics are proven.
- Frame-range bake, destructive flattening and partial-target apply.
- Replacing native Asset Warp, its pin model, interpolation or editor workflow.

The first implementation target remains one supported vector Shape/Path2D with an explicit frame sample and a complete replacement result.

## 5. Host conformance test matrix

Tests are grouped by release gate. Each test must be automated at the lowest layer that can prove the property, with an end-to-end host test where a lower-level test is insufficient.

### Gate A — Persistence, identity and migration

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-001 | FDM extension save/reopen | Authored payload, schema versions, stable FDM IDs and target references round-trip without loss |
| FDM-HOST-002 | Unknown required schema | Load fails closed or preserves/quarantines the unknown record; save never silently strips it |
| FDM-HOST-003 | Deterministic migration | Each supported migration produces the same canonical payload and fingerprint for identical input |
| FDM-HOST-004 | Duplicate / Save As | New document identity is allocated; all internal references are remapped consistently |
| FDM-HOST-005 | Import | Imported target references become destination-owned; no source-document identity leaks |
| FDM-HOST-006 | Delete and recreate target | Reused numeric ElementId cannot satisfy an old FDM reference because generation differs |
| FDM-HOST-007 | Undo/redo extension edits | Authored FDM edits participate in document dirty state and exactly one coherent history step |

**Gate A pass:** Save/reopen, migration, duplication/import and target incarnation tests pass on real AniCraft serialization and command paths.

### Gate B — Snapshot integrity and coordinate semantics

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-010 | Atomic snapshot capture | Geometry, target generation, coordinate descriptor and all dependency stamps come from one consistent host state |
| FDM-HOST-011 | Canonical unwarped source | Snapshot matches authored Shape.path |
| FDM-HOST-012 | Native Asset Warp source | Snapshot uses the agreed rest source; bent Shape.path is not mislabeled as rest geometry |
| FDM-HOST-013 | Coordinate descriptor | Units, axes, origin, scale and local/stage space are explicit and fingerprinted |
| FDM-HOST-014 | Transform-only edit | Transform dependency changes without falsely reporting source geometry mutation |
| FDM-HOST-015 | Geometry/topology edit | Relevant geometry/topology epochs advance on every supported mutation route |
| FDM-HOST-016 | Hierarchy edit | Relevant ancestor transform dependencies are captured and invalidated |
| FDM-HOST-017 | Timeline sample | Frame/time and sampled animated values are explicit, deterministic inputs |
| FDM-HOST-018 | Invalid target capability | Unsupported element kinds or output operations are rejected before evaluation |

**Gate B pass:** No live mutable host reference escapes into FDM evaluation; snapshots are immutable, self-consistent and fully stamped.

### Gate C — Evaluation and shared geometry

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-020 | Deterministic request | Identical authored model, snapshot and time produce equivalent output and fingerprint |
| FDM-HOST-021 | Request/result binding | Result binds to exact target generation, source stamps, model revision, time and dependencies |
| FDM-HOST-022 | Replacement semantics | Initial host slice accepts only a complete, typed Path2D replacement at the declared FDM stage |
| FDM-HOST-023 | Shared DisplayList path | Accepted geometry is observed by stage, picking, bounds and player |
| FDM-HOST-024 | Export parity | Built-in and plugin export frame evaluation observe the same accepted FDM geometry |
| FDM-HOST-025 | No authored mutation | Preview/playback leaves Shape.path, Warp.rest, pins, keyframes and authored FDM payload unchanged |
| FDM-HOST-026 | Multi-target atomicity | A declared atomic target set is wholly accepted or rejected; no partial display publication |
| FDM-HOST-027 | Symbol/editing context | Open-symbol editing and nested timeline evaluation cannot silently bypass or double-apply FDM |

**Gate C pass:** All shared consumers see the same accepted geometry for the same document/time context. No renderer-specific deformation path exists.

### Gate D — Cache, freshness and stale work

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-030 | Geometry changes during evaluation | Old result is rejected before publication |
| FDM-HOST-031 | Target deleted/recreated | Old generation result is rejected |
| FDM-HOST-032 | Transform/hierarchy changes | Result computed against old coordinate transform is rejected |
| FDM-HOST-033 | Model/topology/policy changes | Old result is rejected even when source path is unchanged |
| FDM-HOST-034 | Time/playhead changes | Result for a previous sample cannot appear in the new frame context |
| FDM-HOST-035 | Cancellation race | Correctness does not depend on cancellation winning the race |
| FDM-HOST-036 | Cache-key completeness | Every consumed dependency changes the cache key or invalidation token |
| FDM-HOST-037 | Undo to equal content | Cache reuse is allowed only if target incarnation and every dependency stamp remain valid |
| FDM-HOST-038 | Failed evaluation | Last accepted output is retained or removed according to an explicit policy; stale output is never silently shown as current |

**Gate D pass:** Every publication is freshness-validated, including cache hits and late completions. Cancellation is an optimization, not the correctness mechanism.

### Gate E — Apply, bake and history

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-040 | Apply preflight | All targets, revisions, coordinate spaces and output payloads validate before mutation begins |
| FDM-HOST-041 | Atomic apply | Either every declared target is applied or the document is restored exactly |
| FDM-HOST-042 | Single undo step | One user apply/bake action creates one coherent history step |
| FDM-HOST-043 | Undo/redo | Source geometry and FDM authored state restore correctly; revision epochs remain monotonic |
| FDM-HOST-044 | Retained provenance | Bake retains source identity/fingerprint, model/topology revision, time/range and algorithm/schema versions |
| FDM-HOST-045 | Repeated apply | Duplicate transaction/result application is rejected or explicitly idempotent |
| FDM-HOST-046 | Save/reopen baked result | Applied geometry and retained provenance survive document round-trip |
| FDM-HOST-047 | Failure injection | Failure at each mutation stage rolls back without partial geometry, dirty-state drift or extra history entries |

**Gate E pass:** Apply/bake uses the native command transaction and rollback mechanism; preview remains read-only.

### Gate F — Compatibility and regression

| ID | Test | Required assertion |
|---|---|---|
| FDM-HOST-050 | No FDM payload | Existing AniCraft documents behave identically |
| FDM-HOST-051 | Native Asset Warp regression | Existing Warp editing, interpolation, save/load and rendering are unchanged |
| FDM-HOST-052 | CCC-01.5 separation | Smart Controller and Form Deform pin evaluation remain independent |
| FDM-HOST-053 | Unsupported target | Clear typed rejection; no fallback to bitmap/text/symbol or native Warp behavior |
| FDM-HOST-054 | Existing plugin ABI | No live FDM dependency is introduced into the command plugin ABI |
| FDM-HOST-055 | Windows build | Required Windows host build and host integration tests pass |

**Gate F pass:** Existing projects and native tools remain compatible; no implicit behavior or composition is introduced.

## 6. Implementation sequence and dependency gates

| Stage | Work package | Exit evidence | Must wait for |
|---|---|---|---|
| 0 | Revalidate pinned host source | Current head SHA, source diff review, updated file/blob manifest | FDM-06.3 review |
| 1 | Host extension and identity design | Accepted schema proposal; ID/generation/revision ownership; migration plan | Gate A design review |
| 2 | Snapshot adapter | Immutable Path2D bundle; coordinate and dependency stamps; unit tests | Gate B |
| 3 | Synchronous evaluation adapter | Request/result envelope, deterministic core invocation, typed errors | Stage 2 |
| 4 | Shared evaluated-geometry seam | One host-owned resolved geometry path for editor/picking/bounds/player/export | Gate C |
| 5 | Freshness and cache integration | Complete key/invalidation; publication gate; stale-result tests | Stages 2–4 |
| 6 | Native apply command | Preflight, atomic mutation, provenance, rollback and history | Gate E |
| 7 | Compatibility and release validation | Full gates A–F, Windows CI and regression evidence | All prior stages |

**No parallel host implementation across these stages** until the ownership and mutation boundaries are proven. The initial evaluator is synchronous by design. Async scheduling can be introduced only after the same acceptance gate is covered by tests.

## 7. Required evidence package

Every implementation PR must include:

1. Exact AniCraft base commit SHA and the source files changed.
2. A mapping from each FDM-06.2 contract clause to host implementation symbols.
3. Host unit/integration tests for every affected conformance ID.
4. Before/after behavior tests for documents without FDM payload and native Asset Warp.
5. Save/reopen and undo/redo evidence for any persistent or mutating change.
6. Editor/picking/bounds/player/export parity evidence for evaluated geometry.
7. Windows build and CI results on the exact PR head.
8. Explicit list of unsupported targets and unimplemented capabilities.
9. Confirmation that no CCC-01.5 composition or other deferred behavior was introduced.

A test plan or passing core-only CI is not a substitute for host evidence.

## 8. Open decisions that block implementation

These are deliberate decision points, not assumptions to fill in during coding.

| ID | Decision | Required resolution |
|---|---|---|
| FDM-DEC-01 | Document extension placement | Choose a host-owned serialized field/container and its compatibility behavior; do not assume arbitrary extension maps |
| FDM-DEC-02 | Persistent document identity | Define ID allocation, Save As, duplication, import and recovery semantics |
| FDM-DEC-03 | Target generation and revision epochs | Identify every mutation path that advances geometry/topology/transform/hierarchy dependencies |
| FDM-DEC-04 | Canonical source with Asset Warp | Reconcile Shape.path and Warp.rest with a single non-ambiguous FDM source contract |
| FDM-DEC-05 | Evaluation seam | Select the earliest shared point covering editor, picking, bounds, player and export without double evaluation |
| FDM-DEC-06 | DisplayList replacement | Define path coordinate space, transform ownership, stroke/fill/clip/mask/merge behavior and picking identity |
| FDM-DEC-07 | Cache ownership | Define key construction, invalidation and result publication under Session/RenderService lifecycle |
| FDM-DEC-08 | Error presentation | Decide whether a failed evaluation hides the target, shows source geometry, or retains last accepted output; never show stale output as current |
| FDM-DEC-09 | Apply representation | Decide whether FDM authored definitions remain live after apply, are disabled, or are explicitly baked/removed |
| FDM-DEC-10 | Provenance storage | Define host document representation and user-visible inspectability for retained bake metadata |
| FDM-DEC-11 | Export parity | Route exporter frame access through the same evaluated geometry without making exporter plugins an alternate evaluator |
| FDM-DEC-12 | Native Warp coexistence | Define initial rejection criteria and UI diagnostics for targets with active native Warp |

Each decision must be resolved in a reviewed design change before the dependent stage starts. Decisions that alter FDM semantics require corresponding FDM-side contract changes and tests.

## 9. Exit criteria for FDM-06.3

FDM-06.3 is complete when:

- [x] The actual host repository is accessible and identified.
- [x] The host base commit and inspected source blob SHAs are pinned.
- [x] Current source capabilities and gaps are separated from FDM-side assumptions.
- [x] Host conformance tests are defined across persistence, snapshots, evaluation, freshness, transactions and compatibility.
- [x] A staged implementation sequence and evidence package are defined.
- [x] Blocking host design decisions are explicitly enumerated.
- [ ] The plan is reviewed and accepted.
- [ ] Current AniCraft head is revalidated immediately before implementation.
- [ ] A separate FDM-06.4 implementation authorization is provided.

The last three items are intentionally not claimed complete.

## 10. Source links

All links below are pinned to the audited commit:

- [AniCraft repository](https://github.com/Mujshanan/anicraft)
- [Pinned host commit](https://github.com/Mujshanan/anicraft/commit/2df7775907d1458ce653e7e6314ec8a5d7d565f5)
- [Document model and serialization](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/doc/src/lib.rs)
- [Pure timeline evaluation and DisplayList](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/doc/src/eval.rs)
- [Session, command execution and cache](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/engine/src/lib.rs)
- [Undo/redo history](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/engine/src/history.rs)
- [Render service](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/engine/src/render_service.rs)
- [Plugin command bridge](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/engine/src/cmds_plugin.rs)
- [Plugin exporter frame host](https://github.com/Mujshanan/anicraft/blob/2df7775907d1458ce653e7e6314ec8a5d7d565f5/crates/engine/src/plugin_export.rs)

## 11. Relationship to prior work

- FDM-05.4 remains the verified FDM output-contract baseline.
- FDM-06.2 remains the logical native integration contract.
- The M1 FDM-side conformance matrix remains authoritative for core request/result/persistence contracts; this document adds the host-side tests and source mapping.
- CCC-01.5 remains unchanged: Smart Controller binding evaluation and Form Deform pin evaluation are independent.
- Native AniCraft Asset Warp remains host-owned and is not replaced.
- No AniCraft code, Forma Deform runtime code, plugin ABI or existing PR base branch is modified by this planning document.
