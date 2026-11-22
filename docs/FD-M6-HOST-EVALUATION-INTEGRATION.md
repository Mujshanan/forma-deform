# FD-M6 — Host Evaluation Integration Contract

**Status: blocked on an AniCraft host extension.** The current audited plug-in contract does not expose a persistent custom evaluator in the document/frame evaluation pipeline. This document defines the boundary and acceptance criteria; it does not claim that live playback or scrubbing is implemented.

## 1. Host capability finding

The current AniCraft sources and public plug-in guide were rechecked on 2026-11-22:

- The engine architecture says time-dependent behavior belongs in the pure `doc::evaluate` path; time is a function, not a document mutation.
- Command plug-ins are synchronous, start in a fresh WASM instance, and have a short execution limit. They run when explicitly invoked, not for each playback frame.
- Frame-script plug-ins are keyframe-entry scripts. They can inspect player state and queue timeline actions, but cannot call the document, element, bone, or warp command interfaces.
- No public SDK entry point registers a custom per-frame evaluator, supplies immutable evaluated bone/target inputs to one, or returns effective deformation values to the renderer.

Sources checked: AniCraft `AGENTS.md` (Golden rules), `docs/guide/plugins.md` (command and frame-script behavior), `examples/plugins/sdk/src/lib.rs` (ABI v1), and the published command reference. The prior [SDK capability audit](ANICRAFT_SDK_AUDIT.md) records the command and frame-script boundaries.

**Decision:** do not use a command plug-in loop, frame scripts, timer polling, or repeated `warp.pin_move` calls to imitate a live evaluator. Such approaches mutate authored keyframes, are not deterministic under scrubbing, and bypass the host evaluation lifecycle. M5's explicit apply/bake workflow remains the supported fallback.

## 2. Ownership boundary

### AniCraft owns

- The authoritative document and versioned plugin-binding storage.
- Timeline time, frame evaluation scheduling, evaluated rig/bone transforms, Asset Warp state, renderer integration, dependency tracking, cache invalidation, and undo/redo.
- The native evaluation hook and the lifecycle guarantees around it.

### Forma Deform owns

- The versioned controller/binding schema and migrations.
- Pure input normalization, controller dependency evaluation, mapping, easing, keyform interpolation, validation, and diagnostics.
- A deterministic function that consumes an immutable, explicit-frame host snapshot and produces deformation contributions. It must not mutate the document, authored keyframes, or host caches.

### Boundary rule

The host invokes the Forma Deform evaluator from its native document/frame evaluation path. The evaluator does not call back into AniCraft commands, query the current playhead, access global state, or issue target writes. The renderer consumes returned effective values through a host-owned overlay/contribution layer; authored Asset Warp pins and keyframes remain unchanged.

## 3. Proposed evaluation contract

The host extension should provide one immutable evaluation context per requested document/frame:

- Stable document identity and document revision.
- Explicit zero-based frame/time and the host's frame-rate/timebase descriptor.
- The versioned binding document and its revision.
- Evaluated source values for every declared input, keyed by stable input/bone identity. Rotation values must declare whether they represent keyed pose only or include spring contribution; no implicit angle policy.
- Stable target identities and the target/topology revisions required to reject stale bindings.
- A deterministic evaluation/cache epoch or equivalent dependency stamp.

The pure Forma Deform evaluation returns:

- A complete set of typed, uncomposed deformation contributions keyed by stable binding and target identity.
- Diagnostics with stable codes and references to the affected controller/input/binding/target.
- Dependency information sufficient for the host to invalidate cached results.

The contract must distinguish **no contribution**, **disabled binding**, **invalid binding**, **missing input**, **stale target**, and **evaluation failure**. A failed controller must not silently reuse a previous frame's output. No partial output set may be published if the host's evaluation policy requires all-or-nothing consistency for a controller dependency graph.

This is a semantic contract, not a proposed public Rust function signature or a claim that the current SDK can supply these values.

## 4. Determinism and dependency rules

1. Same binding revision + same explicit frame + same evaluated input snapshot + same target revisions must produce byte-for-byte equivalent normalized output and diagnostics.
2. Evaluation order is stable and independent of hash-map iteration, selection order, UI focus, or playback direction.
3. Scrubbing directly to frame N must produce the same result as reaching N by sequential playback.
4. Repeated evaluation of the same context is idempotent and side-effect free.
5. Controller dependency cycles are rejected during validation; evaluation must not depend on a best-effort cycle break.
6. Binding evaluation remains separate from Form Deform's independent pin-deformation algorithm. No implicit multi-source composition is introduced by M6.
7. The host owns evaluation order relative to rig constraints, spring simulation, native Asset Warp, and rendering. The contract must explicitly state which evaluated bone channel is supplied and where returned contributions enter the render/evaluation pipeline.

## 5. Invalidation and cache requirements

The host must invalidate or recompute when any dependency changes:

- Binding document contents, schema migration, enable/disable state, or binding revision.
- Source bone pose/transform at the requested frame, including any selected spring channel.
- Target identity, target generation, warp topology, pin identity, or relevant target revision.
- Relevant document/scene/rig changes and undo/redo.
- Evaluation settings that affect timebase, coordinate space, or composition.

Cache keys must include all semantic dependencies, not only frame number. A result for frame N must never be reused for a different document, binding revision, input snapshot, or target revision. Undo/redo must restore authored data while advancing or otherwise invalidating dependency epochs so stale results cannot survive history navigation.

## 6. Performance and failure behavior

- Evaluation must be bounded and allocation-aware; no host command dispatch, filesystem/network access, UI work, or WASM instance creation per frame.
- The host must define whether evaluation is synchronous on the render path or scheduled/cached elsewhere, and how stale results are handled during interactive scrubbing.
- Add representative benchmarks for one, 100, and 1,000 bindings, including dependency fan-out and cold/warm cache cases. Set a frame-budget target with measured evidence before enabling playback by default.
- Malformed persisted data, missing sources, stale targets, and numeric failures must produce structured diagnostics and safe absence of the affected contribution, never NaN/Inf renderer state or document mutation.
- Host shutdown, document close, plug-in unload, and schema migration must not leave callbacks or cached references alive.

## 7. Required implementation order

1. **M4 host persistence:** land the AniCraft-owned versioned document extension and typed commands; verify save/load, migration, undo/redo, and revision invalidation.
2. **Native evaluator seam:** add a pure extension point to the host's document/frame evaluation path. Keep it separate from the WASM command/frame-script plug-in ABI.
3. **Input snapshot:** supply immutable, explicit-frame evaluated bone channels and stable target/dependency revisions.
4. **Core integration:** embed or otherwise consume the Forma Deform pure Rust core at a pinned, reviewed version. Do not duplicate mapping semantics in AniCraft.
5. **Effective contribution layer:** combine controller contributions with host evaluation without rewriting authored keyframes or conflating them with Form Deform pin outputs.
6. **Invalidation/cache:** wire all dependency revisions, undo/redo, document close, and plug-in lifecycle.
7. **Verification and profiling:** complete the tests below before enabling live playback.

This is a host-extension plan. No AniCraft source change is included or authorized by this repository milestone.

## 8. Acceptance tests

### Semantics and determinism
- Evaluate identical contexts repeatedly and compare normalized output and diagnostics.
- Evaluate frame N directly and by forward/reverse scrubbing; outputs must match.
- Evaluate two documents with identical frame numbers but different identities/revisions; caches must not cross-contaminate.
- Verify keyed-pose-only and pose-plus-spring channels are explicit and independently tested.
- Verify dependency ordering and cycle rejection.

### Invalidation
- Edit a source bone keyframe, binding, target pin/topology, or target identity; confirm only valid dependent results are reused.
- Undo and redo each change; confirm output follows the restored authored state and stale cache entries are not observed.
- Close/reopen a document and unload/reload the extension; confirm no stale references or callbacks remain.

### Safety and rendering
- Confirm evaluation never writes authored keyframes or Asset Warp pin data.
- Confirm missing input, stale target, invalid schema, NaN/Inf, and cycle errors produce stable diagnostics and no invalid renderer values.
- Confirm Smart Controller contributions do not implicitly compose with Form Deform outputs.
- Confirm preview, scrub, playback, and export use the same documented evaluation semantics.

### Performance
- Benchmark cold and warm evaluation at 1/100/1,000 bindings and realistic dependency fan-out.
- Measure worst-case frame cost, allocations, and cache hit rates under sequential playback and random scrubbing.
- Set and pass a host-approved frame-budget threshold on supported hardware.

## 9. Current behavior while M6 is blocked

- On-demand Smart Controller preview remains supported.
- M5 explicit current-frame apply and bounded range bake remain the supported mutation paths, subject to their pending AniCraft GUI acceptance tests.
- No live playback/scrubbing evaluator is exposed or claimed.
- M6 can move from blocked to implementation only after the host hook, persistence contract, and evaluated-input snapshot are implemented and reviewed in AniCraft.
