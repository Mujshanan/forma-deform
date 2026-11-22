# FD-M6 — Implementation Proposal

**Proposal status:** for review; no host API or implementation is assumed to exist.  
**Recommended delivery:** a small first-party AniCraft engine extension plus reuse of Forma Deform's pure Rust core.  
**Non-goal:** implementing live evaluation inside the current WASM command/frame-script plug-in ABI.

## Executive recommendation

Implement M6 as a native, read-only evaluation stage in AniCraft's existing document/frame evaluation pipeline. AniCraft should own scheduling, document access, evaluated rig inputs, target resolution, renderer integration, cache lifetime, and undo/redo invalidation. Forma Deform should own the controller graph and the pure deterministic mapping/evaluation that turns an immutable frame snapshot into typed target contributions.

The host should call the Forma Deform core directly as a native Rust dependency (or a small shared, host-independent crate extracted from it), not instantiate a WASM plug-in or dispatch host commands for each frame. Keep the existing command plug-in for authoring/diagnostics and M5's explicit apply/bake operations.

**M6 is gated by M4 host persistence.** A live evaluator cannot safely depend on transient dialog parameters. The versioned binding document must first be stored in the authoritative AniCraft document and participate in save/load, undo/redo, and revision tracking.

## 1. Proposed runtime architecture

```text
AniCraft document + timeline
        |
        v
AniCraft frame evaluation (explicit frame/time)
        |
        +--> native rig / constraint / spring evaluation
        |
        +--> immutable Smart Controller input snapshot
        |       - binding document + binding revision
        |       - evaluated bone/input channels
        |       - target identities + topology revisions
        |       - frame/timebase + document revision
        |
        v
Forma Deform pure evaluator (native Rust core)
        |
        +--> typed target contributions
        +--> stable diagnostics
        +--> dependency stamp
        |
        v
AniCraft contribution validation + composition
        |
        v
effective evaluated deformation state
        |
        v
renderer / preview / playback / export
```

This is a conceptual data flow, not an existing AniCraft API diagram. The evaluator is read-only: it never calls commands, edits the document, writes keyframes, changes the playhead, or mutates the host's authored Asset Warp data.

### Why native core reuse

- The controller mapping, easing, validation, and graph semantics already belong in the host-independent Rust core.
- A native call avoids per-frame WASM startup, JSON command marshalling, and command execution limits.
- One implementation of semantics prevents the plug-in preview and native playback evaluator from drifting.
- AniCraft can pin a reviewed core version and upgrade it through normal dependency review.

Before integration, audit the core's dependency graph and build targets. Any SDK, filesystem, UI, global-state, clock, randomness, or host-command dependency must remain outside the pure evaluation crate.

## 2. Host/core API boundary

The initial host extension should expose two internal Rust-level concepts. Exact public type names and module placement are for the AniCraft implementation PR to determine after reviewing its document/evaluation architecture.

### A. Immutable evaluation context

Required fields:

| Field | Purpose |
|---|---|
| Document identity + document revision | Prevent cross-document and stale-document reuse |
| Explicit frame/time + timebase | Deterministic direct evaluation and scrubbing |
| Binding payload + collection revision | Stable authored controller definitions |
| Evaluated input samples | Values already evaluated by AniCraft at the agreed rig stage |
| Input channel semantics | E.g. keyed pose vs pose including spring; units and coordinate conventions |
| Stable target identity | Element/pin IDs, target generation, topology revision |
| Evaluation epoch/dependency stamp | Cache and invalidation correctness |

The host builds this context once for a requested document/frame. The context owns or immutably borrows data for the duration of evaluation; it must not expose mutable document references.

### B. Pure evaluation result

Return a complete result object containing:

- Typed contributions keyed by stable binding and target IDs.
- A stable dependency stamp or explicit dependency list.
- Structured diagnostics with stable codes and references to controller/input/target IDs.
- A result status distinguishing success, no contribution, disabled binding, invalid binding, missing input, stale target, and evaluation failure.

No result should contain host pointers, UI objects, command handles, mutable document references, or renderer-owned resources.

### Input stage ordering

AniCraft must explicitly choose and document which evaluated bone channel is sampled. Recommended first slice: consume the final host-evaluated bone rotation channel after the host's normal rig constraints and any configured spring stage, because that is the value users see driving the rig. If the host cannot expose a stable post-spring channel, ship keyed-pose-only as an explicitly named mode and do not silently mix channels.

The chosen channel, units (radians/degrees), sign convention, wrapping policy, and frame/time conversion must be explicit in the context. Forma Deform must not infer these from the current playhead or from UI state.

## 3. Target contribution and composition policy

This is the most important semantic decision to settle before renderer integration.

1. Keep Smart Controller evaluation separate from Forma Deform's independent pin-deformation algorithm. M6 must not introduce implicit multi-source composition.
2. For the first live-evaluation slice, support only the target kind whose host evaluation representation can be safely overlaid without modifying authored data (initial candidate: Asset Warp geometry pins).
3. Define whether a contribution is an absolute evaluated pin position or a displacement relative to the host's evaluated base position. Do not infer the meaning from field names or reuse the M5 command payload without checking its coordinate contract.
4. Recommended V1 policy: one enabled Smart Controller binding owns one target channel at a time; duplicate writers to the same target are rejected during binding validation. This avoids order-dependent blending.
5. Unbound pins and all unrelated native deformation channels pass through unchanged.
6. The host applies the contribution to an ephemeral evaluated-state overlay consumed by renderer and export. It does not rewrite keyframes or persist evaluated values.
7. If a target is missing, stale, or unsupported, omit that binding's contribution and emit a stable diagnostic; never redirect it to a nearby array index or selection.

Before coding the overlay, add a host-side design test that proves the chosen contribution convention against the existing Asset Warp evaluator. The same convention must be used by on-demand preview, playback, scrubbing, and export.

## 4. Evaluation and failure semantics

- Evaluate the full validated controller dependency graph in stable topological order.
- Reject cycles at binding-document validation/save time; retain defensive cycle detection at evaluation boundaries for malformed legacy data.
- Normalize inputs once and use the same canonical mapping/easing code as the plug-in preview.
- Sort outputs and diagnostics by stable IDs before returning them.
- A direct evaluation of frame N must equal the result at N reached by forward or reverse playback.
- Evaluation is idempotent. It must not mutate the context, authored document, host caches, or keyframes.
- A controller failure must not reuse the previous frame's output. Publish either a valid complete result or the documented safe partial/no-contribution result; the host must choose one all-or-nothing policy for each dependency group.
- Reject NaN/Inf, unknown schema versions, invalid numeric ranges, and stale target generations before renderer consumption.
- Diagnostics should be rate-limited/coalesced by stable code and object ID so playback does not flood logs or allocate a new UI message every frame.

## 5. Cache and invalidation design

Start with correctness before optimization. The first implementation may recompute every frame if benchmarks show this is acceptable. If caching is added, its key must include all semantic dependencies, not just frame number:

- document identity and document revision;
- binding collection revision and enabled state;
- explicit frame/timebase;
- evaluated input sample values or their stable source revision;
- target identity/generation and topology revision;
- evaluation settings and composition policy version.

Invalidate dependent results when bindings are edited, migrated, enabled/disabled, or removed; source bones or relevant constraints/springs change; targets or warp topology change; document undo/redo occurs; or the document closes. Undo/redo must restore authored binding state and invalidate/re-key derived evaluation state. No cache entry may outlive its document or plug-in/core generation.

Prefer host-owned dependency tracking and cache management. Forma Deform returns dependency identities; it does not create a parallel cache or dirty-state system.

## 6. Implementation work packages

### M6-A — Close M4 persistence dependency (prerequisite)

**Owner:** AniCraft host + Forma Deform core.

- Add the optional, versioned binding extension to AniCraft's authoritative document model.
- Add typed read/upsert/remove/list commands or equivalent document operations.
- Validate complete payloads on edit and load; preserve unrelated extension data.
- Implement explicit schema migration policy and actionable load errors.
- Include binding edits in one host undo step; verify save/load and undo/redo.
- Expose binding and document revisions to the evaluation layer.

**Exit:** save-close-reopen preserves stable IDs and values; migration tests pass; undo/redo restores the whole payload; no plug-in sidecar files.

### M6-B — Pure native evaluator seam

**Owner:** AniCraft engine.

- Add an internal evaluator stage to the existing pure document/frame evaluation path.
- Define the immutable context and result types described above.
- Add a registration/dispatch mechanism for the Forma Deform evaluator that is tied to document and application lifecycle.
- Keep evaluation separate from command plug-ins and frame scripts.
- Add a host test evaluator fixture to prove invocation order and read-only behavior.

**Exit:** the host can invoke a deterministic test evaluator for arbitrary explicit frames without mutating the document or changing the playhead.

### M6-C — Reuse the Forma Deform core

**Owner:** shared Forma Deform/AniCraft integration.

- Make the pure Smart Controller evaluator consumable as a native Rust crate.
- Pin the core version in AniCraft; do not copy/paste mapping or easing logic into the host.
- Convert the host snapshot into core input types in one adapter module.
- Return typed outputs and stable diagnostics through the evaluator result.
- Add parity fixtures that compare plug-in on-demand preview and native evaluation for the same canonical document/input snapshot.

**Exit:** parity tests pass for manual and bone-driven controllers, easing boundaries, neutral values, missing inputs, and invalid bindings.

### M6-D — Geometry target overlay

**Owner:** AniCraft engine; Forma Deform supplies typed contributions.

- Implement a non-destructive evaluated-state overlay for the first supported target kind.
- Confirm absolute-position versus displacement semantics with the native Asset Warp evaluator.
- Reject duplicate writers and stale target generations.
- Preserve all unbound native pins and unrelated channels.
- Wire the same evaluated state into viewport playback, scrubbing, still preview, and export.

**Exit:** live controller evaluation visibly affects the intended targets while authored keyframes and Asset Warp data remain byte-for-byte unchanged.

### M6-E — Dependency invalidation, performance and hardening

**Owner:** AniCraft engine + Forma Deform.

- Wire binding, bone, target topology, document revision, and undo/redo invalidation.
- Test document close/reopen and evaluator unload/reload lifecycle.
- Benchmark 1, 100, and 1,000 bindings with cold/warm cache and realistic fan-out.
- Profile random scrubbing and sequential playback; record allocations, p50/p95/p99 time, and cache hit rates.
- Set a frame-budget threshold from measurements on supported hardware before enabling by default.
- Add safe failure and diagnostics tests for malformed schema, missing inputs, stale targets, cycles, and non-finite values.

**Exit:** all acceptance tests pass; frame budget is approved; no stale results or renderer-invalid numeric values.

## 7. Test strategy and acceptance gates

| Gate | Required proof |
|---|---|
| Pure core | Deterministic fixtures, graph ordering/cycle rejection, numeric validation |
| Host seam | Explicit-frame invocation, immutable inputs, no document/playhead mutation |
| Persistence | Save/load, migration, stable IDs, atomic undo/redo |
| Target overlay | Correct pin result, untouched authored warp/keyframes, pass-through for unbound pins |
| Playback | Direct frame N equals sequential and reverse scrub to N |
| Invalidation | Bone, binding, topology, undo/redo, close/reopen all refresh correctly |
| Parity | On-demand preview and native evaluator produce equivalent normalized outputs |
| Export | Exported frame matches viewport evaluation for the same frame |
| Performance | 1/100/1,000-binding benchmarks; measured p95/p99 and allocation budget |
| Failure | Missing/stale/invalid inputs never reuse stale output or emit NaN/Inf |

No single gate substitutes for another. Passing Rust unit tests does not establish host renderer behavior; a visually correct preview does not establish deterministic scrubbing, undo/redo invalidation, or export parity.

## 8. Rollout and fallback

1. Land M4 persistence and the host seam behind a feature flag.
2. Enable native evaluation in a developer build for manual-input controllers only.
3. Add bone-input evaluation after the sampled rig channel is stable and tested.
4. Enable geometry overlay for a limited target set.
5. Enable playback and export only after invalidation and parity gates pass.
6. Keep the feature opt-in until performance budgets are measured.

If any host gate is unavailable, retain on-demand preview and M5 explicit current-frame apply/range bake. Do not emulate continuous evaluation with polling, frame scripts, or repeated keyframe writes.

## 9. Scope, ownership and open decisions

### In scope
- Native read-only Smart Controller evaluation.
- First supported host input channel and first supported target channel.
- Deterministic frame evaluation, diagnostics, cache invalidation, and export parity.

### Out of scope
- New host timeline, rig, constraint, spring, or renderer systems.
- Replacing AniCraft Asset Warp or rewriting authored keyframes during playback.
- Implicit blending with Forma Deform's independent pin algorithm.
- General-purpose third-party evaluator plug-in ABI in the first delivery.
- Multi-writer blending, GPU evaluation, distributed evaluation, or speculative cache optimization.

### Decisions required from AniCraft maintainers before implementation
1. Which rig evaluation stage supplies the bone rotation input?
2. What exact coordinate-space and composition semantics should a live geometry contribution use?
3. Which existing document evaluation object should own the ephemeral overlay?
4. Is the first release native-core linkage acceptable, and what crate boundary/build policy is preferred?
5. What frame-time budget and supported hardware define playback acceptance?

## 10. Recommended next PR sequence

1. **AniCraft PR A:** M4 document extension, typed operations, save/load, migration, undo/redo.
2. **AniCraft PR B:** internal pure evaluator seam + immutable context/result contract + test evaluator.
3. **Forma Deform PR C:** native core export and host adapter + parity tests.
4. **AniCraft PR D:** geometry evaluated-state overlay and renderer/export wiring.
5. **Joint PR E:** invalidation, benchmarks, diagnostics, and acceptance suite.

Keep these changes separately reviewable. Do not combine persistence, evaluator lifecycle, renderer composition, and performance work into one large host PR.

## Decision requested

Approve the architecture and the five open host decisions above before implementation begins. The first code change should be M4 host persistence, not an evaluator workaround in the current plug-in.
