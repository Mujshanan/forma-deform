# FDM-06.3 — AniCraft Host Contract Conformance Plan

**Status:** Source-pinned conformance plan; implementation not authorized  
**Host repository:** `Mujshanan/anicraft`  
**Audited host ref:** `main`  
**Audited host commit:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Forma Deform design base:** FDM-06.2 PR #64, head `9fb3659f181925eda7dd317e775040359f696a98`  
**Depends on:** FDM-05.4 output contract; FDM-06.2; M1 target geometry, evaluation lifecycle and host conformance contracts; CCC-01.5  
**Scope:** Host source conformance planning and test sequencing only. No AniCraft source changes, no runtime implementation, and no PR merge.

## 1. Purpose and audit boundary

FDM-06.2 defined the logical native integration contract. FDM-06.3 pins that contract to a reproducible AniCraft source revision, identifies the actual host implementation points, converts the contract into source-linked conformance tests, and orders the implementation work by dependency.

This plan was prepared from the public `Mujshanan/anicraft` fork at commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`. The fork is public and its `main` branch resolved successfully during this audit. This is a point-in-time audit: implementation must recheck the host branch and compare the pinned SHA before each host change.

The audit does not infer that a missing abstraction can be implemented safely merely because a nearby API exists. Source inspection establishes current structures and call paths; runtime behavior still requires the tests specified below.

## 2. Pinned host source map

| Concern | Source at pinned SHA | Verified host surface | Conformance implication |
|---|---|---|---|
| Document model and serialization | `crates/doc/src/lib.rs` | `Document`, `DocumentFile`, `Shape`, `Element`, `Keyframe`, `Layer`, `Timeline`; document-local `next_id`; explicit serde conversion | Additive FDM storage requires a deliberate document schema/compatibility design, not a plugin-local sidecar |
| Pure frame evaluation | `crates/doc/src/eval.rs` | `evaluate_scene`, `evaluate_timeline`, `evaluate_player` return `DisplayList`; `DisplayItem` carries path, transform, paint, stroke, layer and element identity | FDM must enter before consumers receive the shared evaluated scene; keep document evaluation pure |
| Layer parenting | `crates/doc/src/parenting.rs`; `crates/doc/src/eval.rs` | Rig-parent hierarchy is evaluated as host transform inheritance | Capture the exact resolved transform/hierarchy inputs; do not treat it as nonlinear FDM propagation |
| Session and commands | `crates/engine/src/lib.rs` | `Session` owns `Document`, `revision`, `StateId`, history and a cached `DisplayList`; mutating `execute` snapshots, runs, rolls back on error and records changes | Host extension edits and apply/bake must use this transaction boundary or a proven equivalent |
| Undo/redo | `crates/engine/src/history.rs`; `crates/engine/src/lib.rs` | Persistent document snapshots restore document and state; session revision increments on undo/redo | A future target/dependency epoch must be invalidated/advanced separately; do not mistake restored `StateId` for a monotonic target revision |
| Stage picture | `crates/engine/src/editing.rs` | Stage picture consumes the session's evaluated display list and has its own picture cache | Ensure the accepted geometry is upstream of this path, not injected only into the rasterizer |
| Raster work/cache | `crates/engine/src/render_service.rs` | `FrameJob` carries a cloned `Document`, frame and `StateId`-derived keys; urgent/prefetch queues; `FrameCache`; clear-generation prevents pre-clear work from repopulating a cleared cache | Existing raster cache is image caching, not an FDM result cache or dependency validator |
| Native Asset Warp | `crates/doc/src/warp.rs`; `crates/engine/src/cmds_warp.rs`; `crates/render/src/warp.rs` | Shape-owned `Warp` stores rest path, resolution, pins and shown fingerprint; `WarpMesh` is derived; warp commands edit shapes/keyframes | Preserve native Warp ownership and keyframing; do not reuse its data as FDM topology or silently double-deform |
| Command plugin bridge | `crates/engine/src/cmds_plugin.rs`; `docs/guide/plugins.md` | WASM command plugins issue registered host commands; each run starts fresh and forms one undo step | Not a continuous geometry evaluator, persistent extension store, or result-acceptance API |
| Export path | `crates/engine/src/plugin_export.rs` | Exporter receives evaluated flattened frame data, not the mutable document | Shared evaluation must feed built-in and plugin export frame paths consistently |

### 2.1 Identity and revision observations

- `ElementId` and `LayerId` are typed numeric wrappers, allocated from the document's `next_id` counter.
- The inspected document model does not establish a target-incarnation generation that survives delete/recreate and ID reuse.
- `Session.revision` increments on document changes and undo/redo, but is session-level and is not a typed geometry, topology, transform or hierarchy revision.
- `StateId` represents document content identity. Undo restores a prior content state ID while the session revision increments. This is intentional for render-cache reuse; it is not a substitute for monotonic per-target freshness epochs.
- The serialized `Document`/`DocumentFile` schema enumerates document-owned fields, including a `data` model. The audit does not establish that this existing data-table model is a safe arbitrary extension namespace for FDM records. Do not overload it without a separate schema and ownership review.

### 2.2 Evaluation and render observations

- The pure document evaluator builds a `DisplayList`; its items identify the originating layer and element and contain evaluated path/transform data.
- The session caches the current display list by content state, scene, frame and open-editing context. Hit testing, bounds and stage-related queries share this evaluated list.
- Raster `FrameJob` values clone the document and are keyed by a render key derived from `StateId`, frame and view. This is downstream image rendering.
- The render worker checks the cache-clear generation before inserting completed images. This protects a cleared cache from late image work, but it does not validate FDM target identity or every consumed dependency.
- No FDM-specific result task, cancellation token, target-generation check or prepared-result acceptance API was established in the inspected host source.

### 2.3 Native Warp boundary

- Native Warp is stored on a shape and contains the rest path, mesh resolution, pins and a fingerprint of the shown outline.
- The mesh is derived from the rest path/resolution and current pin state; it is not the same persistent topology model as FDM's lattice/cage.
- Native pin edits are host commands and are timeline-aware.
- The current plugin guide documents command-driven document edits, not a persistent custom evaluator.
- The initial FDM conformance target remains an explicitly supported vector Path2D target. Bitmap, text, symbol, group and nested nonlinear propagation support must not be inferred from renderer capabilities.

## 3. Contract-to-host conformance matrix

The M1 matrix remains the canonical inventory. The following IDs are the execution plan for AniCraft-side evidence. They refine, rather than replace, HOST-M1-001 through HOST-M1-020.

| Plan ID | Contract surface | Host change eventually required | Required conformance evidence |
|---|---|---|---|
| FDM06-H0 | Version/capability handshake | Native host capability descriptor and version negotiation | Compatible, unsupported-major, unsupported-target, payload-limit and required-capability tests |
| FDM06-H1 | Document-owned persistence | Versioned FDM extension record integrated into document save/load and dirty tracking | Save/reopen, unknown-schema preservation, deterministic migration, malformed-record and size-limit tests |
| FDM06-H2 | Identity/incarnation | Document identity plus target generation/lifetime rules | Delete/recreate, duplicate, import, Save As and ID-reuse tests; old references never resolve to a new target |
| FDM06-H3 | Revision ownership | Typed source geometry/topology/transform/hierarchy/model dependency epochs | Enumerate all mutation paths; each advances only affected epochs; undo/redo never regresses epochs |
| FDM06-H4 | Canonical snapshot | Host snapshot provider for supported Path2D targets | Atomic payload/identity/coordinate/revision capture; immutability; finite/path validity; canonical fingerprint |
| FDM06-H5 | Coordinate and hierarchy | Explicit target-local-rest descriptor and resolved host transform dependencies | Known transform fixtures, parent/ancestor changes, singular transforms, coordinate-space mismatch and no implicit conversion |
| FDM06-H6 | Explicit timeline sample | Read-only sample API independent of global playhead | Repeated same-frame sampling, tween boundaries, subframe support decision, nested timeline and playhead non-mutation |
| FDM06-H7 | Deterministic evaluation | Native adapter invokes FDM core with immutable request | Same request produces same output/fingerprint; no host reads from evaluator; typed errors and resource limits |
| FDM06-H8 | Shared evaluated geometry | FDM result substitution before common evaluated-scene consumers | Same geometry observed by picking, bounds, stage, player and built-in/plugin export |
| FDM06-H9 | Cache and freshness | Host-owned FDM result cache and acceptance gate | Dependency-by-dependency invalidation, stale-result rejection, old-key isolation, no stale publication after undo/delete/recreate |
| FDM06-H10 | Scheduling | Initially synchronous evaluation; later host-owned async task lifecycle | First prove sync correctness; then out-of-order completion, cancellation, document switch, target deletion and stale completion tests |
| FDM06-H11 | Apply transaction | Explicit host command with complete preflight and rollback | Stale/invalid result causes zero writes; injected mid-commit failure restores exact document; duplicate apply rejected |
| FDM06-H12 | Retained bake | Host-native retained source/provenance record and native output representation | Save/reopen restoration, deterministic redo, source recovery, finite frame-range preflight and no double evaluation |
| FDM06-H13 | Diagnostics | Stable host/FDM error mapping | Identity, stale, unsupported, malformed, cancelled, resource and rollback failures remain distinct and actionable |

### 3.1 Required consumer parity fixture

A single test document must contain at least:
- one supported vector Path2D target;
- a second unrelated vector target used as a non-invalidation control;
- one target with a transform dependency;
- one target under a rig-parent hierarchy;
- a native Asset Warp target used only to verify coexistence/rejection rules;
- a short timeline with a keyframe boundary and an interpolated frame.

For the supported FDM target, compare canonical output fingerprints and geometry semantics across:
1. editor display list;
2. picking/hit-test geometry;
3. bounds/spatial queries;
4. stage rendering;
5. player evaluation;
6. built-in vector export;
7. plugin export frame data.

Pixel comparisons alone are insufficient: the test must also compare the evaluated path, target identity, coordinate semantics and the exact sample/dependency stamp.

## 4. Implementation work packages and ordering

This is a dependency order, not authorization to implement any item.

### WP0 — Source pin and conformance harness

**Prerequisite:** none.

- Record the AniCraft source SHA and Forma Deform contract SHA in test output.
- Add a host integration test harness capable of constructing real AniCraft documents and invoking the actual engine/document APIs.
- Keep fixtures deterministic and headless where possible; reserve GUI checks for stage/editor parity.
- Add a test manifest mapping every FDM06-H ID to a test name, source module and evidence artifact.

**Exit:** a reproducible harness can load a real host document, evaluate a frame, inspect the DisplayList and execute/undo a host command.

### WP1 — Host identity and dependency epochs

**Prerequisite:** WP0.

- Decide document identity and target generation ownership.
- Audit every path mutation: direct commands, timeline edits, shape replacement, import, duplicate, symbol/library operations, warp edits, undo/redo and deserialization.
- Define geometry/topology/transform/hierarchy dependency separation.
- Do not retrofit revision counters only at the FDM adapter call site; all host mutation paths must participate.

**Exit:** H2/H3 pass, including undo/redo and ID reuse.

### WP2 — Persistent extension data

**Prerequisite:** WP1 identity semantics.

- Define the FDM extension record schema, version domains, size limits and unknown-field behavior.
- Integrate with document serialization, load validation, dirty state, save/reopen, duplicate, import and history.
- Keep FDM payload migrations separate from host document migrations.
- Preserve unknown required payloads for recovery or reject loading; never silently drop them.

**Exit:** H0/H1 pass; no FDM authored data is stored only in plugin files or session memory.

### WP3 — Atomic snapshot and explicit sampling

**Prerequisite:** WP1; WP2 for authored-model references.

- Implement a single host snapshot operation that captures target generation, canonical Path2D, coordinate descriptor and every consumed dependency from one consistent state.
- Use Shape.path for an unwarped target and Warp.rest for a native-Warp-backed target only where the accepted coexistence rule permits capture. Do not imply that the current contract composes both systems.
- Resolve hierarchy and transform dependencies without mutating the global playhead.
- Capture an explicit timeline sample and model revision.

**Exit:** H4/H5/H6 pass.

### WP4 — Synchronous shared evaluated-geometry path

**Prerequisite:** WP3; FDM request/result contract accepted.

- Add the native host adapter and call the pure FDM evaluator synchronously for the first vertical slice.
- Place result substitution before shared DisplayList construction/consumption, not in a renderer or stage-only path.
- Preserve target/layer/element identity and all non-geometry DisplayItem properties.
- Define fail-closed behavior for unsupported target sets and evaluation errors.
- Keep Smart Controller and Form Deform pin evaluation independent (CCC-01.5).

**Exit:** H7/H8 pass for the one-target Path2D vertical slice.

### WP5 — Freshness and cache integration

**Prerequisite:** WP4 synchronous correctness.

- Add a host-owned FDM result cache keyed by complete typed dependency stamps, authored-model/graph revisions, explicit sample, algorithm/schema and output semantics.
- Validate freshness at publication even on cache hits.
- Ensure deletion/recreation, undo/redo, transform/hierarchy changes and model edits cannot resurrect stale output.
- Reuse the existing raster FrameCache only for its existing image purpose; do not put prepared FDM geometry into it as a shortcut.

**Exit:** H9 passes synchronously. Only then may async evaluation be considered.

### WP6 — Asynchronous scheduling

**Prerequisite:** WP5.

- Host owns task identity, priority, cancellation and publication.
- FDM may support cooperative cancellation but cancellation is never the freshness proof.
- Test deliberately reordered completion, cancellation races, document/tab switch, target deletion/recreation, undo/redo and dependency mutation during evaluation.

**Exit:** H10 passes. If async does not materially improve the first integration, keep synchronous evaluation.

### WP7 — Apply and retained bake

**Prerequisite:** WP4; WP1/WP2; transaction semantics reviewed.

- Implement explicit apply as a host command with complete preflight, duplicate-application guard and one atomic transaction.
- Decide native output representation and retained-source/provenance schema before implementing bake.
- Bake the full target/frame set atomically; retain enough persistent source data for restoration after save/reopen.
- Ensure baked output is not fed back into the same FDM evaluation stack.

**Exit:** H11/H12 pass, including injected commit failure and save/reopen.

## 5. Required negative and adversarial cases

Every corresponding gate must include these cases, not only happy-path examples:

- target deleted and recreated with the same numeric ID;
- target moved to another layer or hierarchy parent;
- geometry changed while an evaluation is in flight;
- transform changed without geometry changing;
- ancestor transform changed;
- frame/playhead changed after snapshot capture;
- authored FDM model or topology changed after evaluation;
- native Asset Warp pin state changed;
- undo then redo to content with a prior StateId;
- document switched while render/evaluation work is in flight;
- duplicate/import remaps host IDs but leaves FDM internal references;
- unknown required schema and malformed extension payload;
- NaN/infinite coordinates, singular transforms and oversized output;
- partial multi-target failure and injected transaction commit failure;
- repeated apply/bake request with the same prepared result.

Expected result is deterministic rejection or correct recomputation. No silent fallback, partial commit, implicit rebase, target substitution or double deformation.

## 6. Current gaps and decisions that remain open

The pinned source confirms useful host infrastructure, but it does not yet provide the complete FDM contract:

1. **Target incarnation:** no verified target generation/lifetime API.
2. **Typed revisions:** session revision and StateId are insufficient for per-target geometry and dependency freshness.
3. **Extension persistence:** no verified FDM-owned extension namespace; the existing document data-table model is not assumed to be a generic plugin metadata store.
4. **Snapshot provider:** no atomic FDM Path2D/dependency snapshot API.
5. **Sampling:** pure frame evaluation exists, but a read-only atomic FDM snapshot-at-explicit-time API has not been established.
6. **Shared insertion point:** DisplayList is the right logical seam, but no FDM result overlay/adapter exists.
7. **FDM cache:** current FrameCache is downstream raster-image cache; it is not an FDM geometry cache.
8. **Async freshness:** render-cache clear-generation protects cleared raster cache insertion; it does not provide typed FDM result acceptance.
9. **Bake:** existing native Warp and Envelope behavior is not automatically equivalent to retained FDM bake.
10. **Host tests:** current host test inventory must be mapped to actual test modules during WP0; source presence alone is not passing evidence.

Open decisions requiring a separate design/review before their work package:
- document UUID and target-generation allocation/remapping;
- exact target-level revision storage and mutation coverage;
- canonical Path2D target eligibility and native Asset Warp coexistence/rejection policy;
- coordinate-space and hierarchy dependency representation;
- extension serialization/unknown-data preservation and migration policy;
- precise shared evaluation seam and behavior for nested symbols/player;
- atomic target-group policy;
- native retained-bake representation and frame-range transaction model.

## 7. Release gates and evidence requirements

| Gate | Required evidence | Blocks |
|---|---|---|
| G0 — Reproducible host baseline | Exact host SHA, build/test command, fixture harness and source map | Any implementation |
| G1 — Identity/revisions | H2/H3 automated host tests | Persistence and evaluation |
| G2 — Persistence | H0/H1 save/load/migration tests | Authored FDM data activation |
| G3 — Snapshot/sample | H4/H5/H6 host tests | Calling FDM evaluator from AniCraft |
| G4 — Synchronous shared output | H7/H8 consumer parity | Declaring live preview integrated |
| G5 — Freshness/cache | H9 adversarial invalidation tests | Async evaluation and cache reuse |
| G6 — Async | H10 race/cancellation tests | Production async scheduling |
| G7 — Apply/bake | H11/H12 rollback, undo/redo and recovery tests | Production apply/bake |

Every test report must record:
- AniCraft commit SHA and branch;
- Forma Deform commit SHA and integration-contract version;
- host build/profile and platform;
- test ID and exact test name;
- pass/fail/skipped status and failure artifact;
- whether the evidence is source inspection, unit test, host integration test, GUI smoke test or export parity test.

A Forma Deform unit-test pass or WASM adapter build does not satisfy a host gate.

## 8. Acceptance checklist for FDM-06.3

- [x] Correct host repository identified and accessible.
- [x] Exact AniCraft main SHA pinned.
- [x] Document, evaluation, session, history, render-cache, plugin and Warp source paths inspected.
- [x] Existing M1 host matrix reconciled with the FDM-06.2 contract.
- [x] Conformance IDs, negative cases, dependencies and release gates defined.
- [ ] Host test harness and actual host-side tests implemented (WP0 onward; separate authorization required).
- [ ] AniCraft implementation authorized and started (not part of this milestone).

## 9. Scope and authorization

FDM-06.3 is a planning artifact. It does not:
- change AniCraft source or its document schema;
- implement target generations, revision epochs, persistence, snapshots, evaluator hooks, cache, scheduler, UI, apply or bake;
- alter FDM algorithms or CCC-01.5;
- replace or compose native Asset Warp;
- authorize implementation or merging.

The next implementation-ready activity is WP0 — Source Pin and Conformance Harness. It requires an explicit implementation authorization in the relevant repository. No host changes are made by accepting this plan alone.
