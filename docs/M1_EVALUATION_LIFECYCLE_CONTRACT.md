# M1 — Evaluation and Host Lifecycle Contracts

**Status:** FDM-side evaluation request/result DTOs and validation are implemented; AniCraft host conformance is not included.
**Depends on:** `docs/M1_TARGET_GEOMETRY_CONTRACT.md` and accepted FDM v2 architecture decisions ADM-001 through ADM-007.
**Purpose:** complete the M1 boundary beyond source-geometry identity and immutable Path2D snapshots.

This document is normative for the next FDM implementation slice. It defines data ownership and validation boundaries, not a concrete AniCraft SDK API, wire format, or authorization to change the host.

## 1. Ownership and invariant summary

| Concern | Owner | FDM responsibility |
|---|---|---|
| Artwork, target identity, source topology, transforms and hierarchy | AniCraft | Consume explicit stable references and immutable snapshots |
| Timeline, frame/time sampling and scheduling | AniCraft | Evaluate only the explicit sample supplied by the host |
| Authored deformation definitions and topology-specific data | Forma Deform | Own schemas, validation and deterministic evaluation |
| Deformation-specific interpolation and composition | Forma Deform | Evaluate only explicit, ordered composition graphs |
| Document serialization, dirty state and undo history | AniCraft | Supply versioned payloads and typed command requirements |
| Cache lifecycle and invalidation | AniCraft | Publish complete dependency stamps; never assume cache freshness |
| Apply/bake transactions and native geometry mutation | AniCraft | Return prepared results with source dependencies and provenance |

The following invariants are mandatory:

1. Source snapshots are immutable point-in-time values.
2. Every evaluated result identifies its exact target and every source dependency.
3. Evaluation is deterministic for identical authored data, snapshots, algorithm versions and time samples.
4. Evaluation is read-only. Only explicit host transactions may mutate a document.
5. No implicit coordinate conversion, target rebinding, stale-result rebase, or deformation-source composition is permitted.
6. Smart Controller binding evaluation and Form Deform pin evaluation remain independent under CCC-01.5. ADM-004 does not override that restriction.

## 2. Geometry evaluation input

### 2.1 Source snapshot

The initial source input remains the validated `GeometrySnapshot` from the M1 target-geometry contract:

- `HostTargetRef` including namespace, document ID, target kind, target ID and generation.
- Non-zero geometry and topology revisions.
- Canonical Path2D v1 payload and matching fingerprint.
- Explicit `TargetLocalRest` coordinate descriptor.

The host adapter must capture these values atomically. FDM must not read live host objects or reconstruct missing metadata from names, array positions, selection state or display geometry.

### 2.2 Evaluation context is separate from source geometry

The source-path revisions do not represent every input that can affect a deformation. The host must supply separate, typed dependency stamps for:

- target transform and coordinate conversion context;
- hierarchy/parent relationship and relevant ancestor transforms;
- explicit animation/time sample;
- upstream evaluated deformation results, when the accepted graph requires them;
- host capability or evaluation-stage identity when a result depends on a particular pipeline stage.

A transform-only edit must not advance source geometry revision. It must advance the corresponding transform dependency revision. Hierarchy changes must advance a hierarchy dependency revision. Native Asset Warp pin state is not folded into the Path2D fingerprint; if a future algorithm consumes pin state, it is a separate dependency with its own identity and revision.

### 2.3 Explicit evaluation request

An evaluation request is a value containing:

- versioned evaluation-request schema name and version;
- one or more validated source snapshots;
- the FDM authored-model identity and revision;
- an explicit time-sample descriptor;
- all required transform, hierarchy and upstream-result dependency stamps;
- the selected evaluation-stage identifier;
- an explicit composition graph, if and only if one has been authored and validated.

Missing required dependencies fail closed. FDM must not query the host implicitly, substitute the current playhead, or use a previous frame's values.

## 3. Evaluated geometry output

### 3.1 Result envelope

Every prepared result must contain:

- result schema name and version;
- exact `HostTargetRef`;
- the complete set of source `GeometrySnapshotStamp` values, including every source geometry consumed by a multi-source request;
- complete non-geometry dependency stamp set;
- FDM model identity, model revision and algorithm identifier/version;
- explicit time-sample identity;
- declared output payload kind/schema and coordinate-space descriptor;
- deterministic canonical payload fingerprint.

The result DTO represents a successful prepared geometry output only; failed request validation or evaluation is returned through stable contract errors/diagnostics outside this success-result envelope. The result is a prepared value, not a host mutation command. It is invalid if any required dependency is absent, malformed, unsupported or stale.

### 3.2 Coordinate and operation semantics

The initial source geometry is `TargetLocalRest`. An output must declare its coordinate space explicitly. The initial adapter must not label stage-space `warp.info` values as target-local rest geometry or treat the two as interchangeable.

An output must also declare its operation semantics. At minimum, distinguish:

- an evaluated replacement geometry value;
- an independent deformation contribution that is not composed unless an explicit, validated composition graph is supplied.

No implicit additive, sequential, weighted or precedence-based composition is allowed. A result must never be applied twice. If multiple sources affect a target, their order and blend semantics must be represented by a validated, versioned FDM composition graph. Until then, ambiguous multi-source results are rejected.

### 3.3 Result validity

A result is applicable only when the host confirms that:

1. the full target identity still resolves to the same target incarnation;
2. geometry and topology revisions match the captured source stamp;
3. every transform, hierarchy, time and upstream dependency still matches;
4. the output schema and coordinate descriptor are supported;
5. the result fingerprint validates;
6. the requested operation is supported by the host target capability.

Fingerprint equality alone is never sufficient to establish freshness. A revision regression is a host integrity error. A changed target generation is an identity failure, not an ordinary stale-geometry warning.

## 4. Time and animation sampling

AniCraft owns timeline state and sampling. FDM does not own a playhead, frame clock, keyframe track, or animation scheduler.

A time sample must be explicit and immutable and must identify:

- document timebase/rate or an opaque host timebase identity;
- integer frame/tick position;
- subframe fraction only where the host supports it;
- sample policy (for example, keyed pose only versus a separately declared evaluated pose);
- a host-issued sample/revision token sufficient to detect a changed sample context.

Do not use an unqualified floating-point seconds value as the only time identity. Do not mutate the playhead to obtain samples. Range evaluation supplies a finite, ordered list of explicit sample descriptors; it does not ask FDM to advance time.

For deterministic evaluation, all bone/transform/ancestor values required at a sample must be captured consistently by the host. FDM evaluates the supplied values and reports the exact sample identity in its result.

## 5. Scheduling, dependencies and cache invalidation

AniCraft owns scheduling and cache lifecycle (ADM-007). FDM may expose dependency descriptors and deterministic result fingerprints, but it must not create a competing document cache or claim a result remains current without host validation.

A host cache key must include, at minimum:

- document and full target identity, including generation;
- geometry and topology revisions;
- every transform and hierarchy dependency revision;
- explicit time-sample identity and sample policy;
- upstream evaluated-result fingerprints and graph ordering;
- FDM authored-model identity/revision;
- composition-graph identity/revision where applicable;
- output schema and algorithm versions.

Any dependency change invalidates the corresponding cached result. In particular:

- source geometry or topology change invalidates dependent geometry results;
- transform or hierarchy change invalidates results that consume those dependencies, without pretending source Path2D changed;
- time-sample change invalidates time-dependent results;
- model, algorithm, schema or composition-graph change invalidates results computed from the old definition;
- parent/upstream result change invalidates dependent descendants.

Cache hits must still be checked against the complete dependency key. Partial keys, document-wide revision alone, matching target names, or matching geometry fingerprints alone are insufficient.

## 6. Persistence and schema lifecycle

AniCraft owns document storage, serialization, dirty state and undo/redo. Forma Deform owns the schema and validation rules for its authored deformation data.

The host extension must be:

- versioned independently from the host document format and from each geometry payload schema;
- stored inside the authoritative document, not in plug-in sidecar files;
- validated before commit and after load;
- migrated only through named, deterministic, tested migrations;
- preserved through save/reopen, duplicate/clone and import according to explicit identity-remapping rules;
- bounded by host-enforced size/count limits.

Persist authored definitions, stable FDM IDs, host references and explicit settings. Do not persist runtime pointers, session handles, snapshot buffers, cache entries or transient evaluated results as authoritative data. A duplicate document must remap document-scoped host references and preserve internal FDM relationships without accidentally binding to the source document.

Unknown required schema versions or malformed authored payloads fail with an actionable error or a documented quarantine path. The host must not silently discard authored deformation data.

The Smart Controller `PersistentBindingDocument` in FD-M4 is a separate payload contract. It does not by itself implement persistence for FDM v2 deformation topology, mappings, compositions or bake provenance.

## 7. Apply, bake and restoration transactions

Preview and evaluation are read-only. Applying a prepared result is an explicit AniCraft command.

Before mutation, the host must preflight the complete operation:

1. resolve every target by full stable identity;
2. validate all source and non-geometry dependency stamps;
3. validate output payloads, coordinate spaces and target capabilities;
4. validate the entire requested frame range and all per-frame results;
5. establish resource limits and a deterministic commit order.

If preflight fails, zero writes occur. Commit is atomic: either every requested result is applied or the entire transaction rolls back. A successful user-visible apply/bake is one coherent undo step, with redo restoring the complete result.

ADM-006 requires a retained, non-destructive native bake with provenance and source restoration. The host must retain enough information to identify the source model/revisions, algorithm/schema versions, target identity, time range, and generated native data. Removing or reverting a bake must restore the pre-bake authored state without relying on a stale in-memory snapshot.

The first implementation must not claim a host batch command exists unless AniCraft implements and tests it. A plug-in may use an existing verified outer transaction only when all nested commands are documented to share its rollback and undo boundary.

## 8. Failure and diagnostic taxonomy

The boundary must distinguish at least:

- invalid or unsupported contract/schema version;
- document, target or target generation not found;
- target kind/capability mismatch;
- invalid source snapshot or fingerprint integrity failure;
- stale geometry/topology revision;
- transform or hierarchy dependency mismatch;
- missing or changed time sample;
- missing/stale upstream result;
- invalid/cyclic composition graph;
- unsupported output kind or coordinate space;
- stale authored model/algorithm version;
- resource-limit rejection;
- transaction preflight failure;
- commit failure and rollback failure.

Diagnostics must identify the failing dependency and whether the operation is safe to retry. No automatic target substitution, stale-result rebasing or partial-success fallback is allowed.

## 9. Required contract test groups

The M1 evaluation-contract module includes tests for explicit time-sample validation, source/target identity matching, duplicate source and dependency rejection, model/time/dependency binding, coordinate-space preservation, deterministic output fingerprints, and explicit composition-graph stamps. The request and result values expose read-only accessors and keep their internal state private. The current PR #30 head passed the complete GitHub Actions CI workflow, including workspace tests and all three release WASM builds (run #690).

FDM-owned authored persistence and typed composition-graph validation are implemented in `docs/M1_FDM_PERSISTENCE_AND_COMPOSITION_SCHEMA.md` and the `persistence` / `composition_graph` modules. The v1 schema is the first persisted format; no legacy format is silently upgraded. Future schema changes require named deterministic migrations. Host-side conformance tests are listed in `docs/M1_HOST_CONFORMANCE_TEST_MATRIX.md`; they remain blocked until a separately authorized AniCraft host extension implements the required capabilities.

## 10. Explicit non-goals and authorization boundary

This contract does not implement an AniCraft document UUID, target generation allocator, revision store, timeline sampler, cache, document extension, UI, evaluator hook, transaction command or native bake command.

It does not change Smart Controller/Form Deform composition semantics under CCC-01.5, replace native Asset Warp, or authorize changes to AniCraft. Host-side work requires a separate reviewed and explicitly authorized AniCraft change.

