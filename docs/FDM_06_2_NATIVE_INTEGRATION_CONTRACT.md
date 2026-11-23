# FDM-06.2 — Native AniCraft Integration Contract

**Status:** Design proposal; pending review and acceptance  
**Milestone:** FDM-06.2 — Native Integration Contract Design  
**Depends on:** FDM-05.4 output contract; FDM v2 ADM-001–ADM-007; CCC-01.5; M1 target-geometry and evaluation-lifecycle contracts  
**Host:** `Mujshanan/anicraft`  
**Scope:** Contract design only. No AniCraft implementation, FDM runtime implementation, plugin ABI change, or merge is authorized by this document.

## 1. Purpose

This document turns the accepted FDM architecture and the existing M1 FDM-side contracts into a concrete host-integration design for AniCraft.

It specifies:
- the host-owned integration surfaces and their responsibilities;
- the immutable request and prepared-result exchange;
- where deformation enters the shared evaluated-geometry pipeline;
- persistent extension-data ownership;
- dependency, freshness, scheduling and cache rules;
- transaction, undo/redo and retained-bake boundaries;
- host conformance gates and unresolved decisions.

It does **not** prescribe AniCraft's internal Rust types, threading library, renderer implementation, storage encoding, or plugin ABI. Those choices must be made against a pinned, current AniCraft source revision before implementation.

## 2. Normative architecture decisions

The following decisions are binding for this proposal unless explicitly revised:

1. AniCraft remains the sole authority for document identity, artwork, geometry, hierarchy, transforms, timeline, persistence, cache lifecycle, rendering, commands and undo/redo.
2. FDM owns deformation-authored schemas, topology, mappings, algorithms, specialized interpolation, composition semantics and deterministic evaluation.
3. FDM consumes immutable host snapshots and explicit sampled values. It never reads live AniCraft objects or mutates the document during evaluation.
4. AniCraft accepts or rejects results. A successful FDM result is a prepared value, not a document command and not permission to mutate authored geometry.
5. The integration point is before shared evaluated geometry is consumed by hit testing, bounds, stage rendering, player evaluation and export. A rasterizer callback or stage-only overlay is insufficient.
6. Preview/evaluation is read-only. Apply and bake are explicit host transactions.
7. All dependency and target freshness checks are performed by the host at result acceptance, even when work was cancelled or a cache entry was found.
8. No implicit coordinate conversion, stale-result rebasing, target substitution, or composition is permitted.
9. CCC-01.5 remains in force: Smart Controller evaluation and Form Deform pin evaluation remain independent. No output composition is introduced by this contract.
10. Native Asset Warp remains AniCraft-owned. This contract does not replace it or silently combine its pin state with FDM deformation.

## 3. Integration surfaces and ownership

The host integration consists of six logical surfaces. They may be implemented by fewer internal services, but their semantics must remain independently testable.

| Surface | Owner | Contract responsibility |
|---|---|---|
| Capability and version handshake | AniCraft | Report supported contract versions, target kinds, payload kinds, output operations and limits |
| Document extension store | AniCraft storage; FDM schema | Persist and migrate authored FDM payloads inside the authoritative document |
| Snapshot provider | AniCraft | Resolve stable targets and atomically capture immutable geometry and dependency stamps |
| Evaluation service | FDM core, invoked by AniCraft | Evaluate an explicit request without host reads or document mutation |
| Result acceptance and evaluated-geometry stage | AniCraft | Validate freshness/capability and expose accepted output to every shared geometry consumer |
| Apply/bake command service | AniCraft | Preflight, commit, rollback, persist provenance and integrate with history |

The plugin command ABI is not the live-evaluation boundary. Existing command plugins remain appropriate for explicit user commands and diagnostics, but their fresh-run command lifecycle does not provide the continuous frame-evaluation and result-acceptance semantics required here. A native host integration adapter or equivalent in-process host service is required. The exact module and ABI are open until the host source is pinned and reviewed.

## 4. Versioning and capability negotiation

The host and FDM must negotiate a versioned integration contract before any authored data is evaluated.

The handshake must report:
- host namespace and host build/API identity;
- supported major/minor integration-contract versions;
- supported target kinds and capability flags;
- supported source snapshot payload kinds and schemas;
- supported result payload kinds, coordinate spaces and operation semantics;
- supported persistent extension schema versions and migration capabilities;
- maximum payload size, target count, dependency count and frame-range size;
- supported evaluation stages and whether the host supports synchronous, deferred or asynchronous evaluation.

A major-version mismatch, unsupported required capability, or unknown required payload schema fails closed. Optional capabilities may be omitted only when the authored operation does not require them.

Contract version, host document format version, FDM authored-data schema version, Path2D payload schema version, result-envelope version and algorithm version are separate version domains. They must never be conflated.

## 5. Persistent document extension

### 5.1 Ownership and storage

AniCraft owns the extension storage container, serialization lifecycle, dirty state, save/load, duplication/import, and transaction boundary. FDM owns the payload schema and validation/migration rules for its authored content.

The extension is stored inside the authoritative document. Plugin-local files, session memory, global settings and sidecar files are not authoritative storage for deformation definitions.

A proposed logical record is:

```text
DocumentExtensionRecord {
  owner_namespace: "forma-deform",
  extension_schema: { name, major, minor },
  payload_schema: { name, major, minor },
  payload: <versioned FDM authored document>,
  integrity: <canonical payload fingerprint>
}
```

This is a logical shape, not a committed wire format. JSON, binary encoding and host serialization types remain implementation choices.

### 5.2 Persisted versus transient state

Persist:
- stable FDM object IDs and authored deformation definitions;
- host target references, including target generation;
- topology, control-point IDs, connectivity and mappings;
- explicit composition graph definitions where authored and valid;
- deformation-specific animation property definitions and authored values;
- policy/settings and schema/provenance metadata required to interpret the definition.

Do not persist as authoritative document state:
- runtime pointers or session handles;
- snapshot buffers;
- evaluator instances or prepared mappings that can be deterministically rebuilt;
- task handles, cancellation tokens or scheduler state;
- cache entries or transient preview results;
- stale evaluated geometry masquerading as authored source artwork.

Unknown required schemas and malformed records must not be silently discarded. The host must reject the load, preserve the unknown record for recovery, or use a documented quarantine path. It must not silently strip data on save.

### 5.3 Duplication, import and migration

Save/reopen preserves document and target identities. Duplicate/Save As creates a new document identity and remaps document-scoped target references. Import remaps references into destination-owned identities. Internal FDM references must remain internally consistent after remapping.

Migrations are named, deterministic, version-to-version transformations with tests. No guessed migration or implicit downgrade is allowed. Host document migrations and FDM payload migrations are separate operations.

Extension edits participate in the same host document transaction and dirty-state semantics as other authored edits.

## 6. Target identity, snapshots and dependency capture

### 6.1 Stable target identity

The existing FDM `HostTargetRef` is the canonical adapter-facing reference:

```text
HostTargetRef {
  host_namespace,
  document_id,
  target_kind,
  target_id,
  generation
}
```

AniCraft owns all fields and their allocation/lifetime semantics. A numeric ElementId alone is insufficient where IDs may be reused. A target generation changes on deletion/recreation and is never rolled back by undo.

The host must separately report target capabilities. An ID does not imply that a target supports FDM geometry evaluation, a particular payload kind, preview, apply or bake.

### 6.2 Atomic snapshot bundle

The host captures one immutable `EvaluationInputBundle` from a single consistent document state. It contains:

- exact target reference and target capability descriptor;
- one or more validated source `GeometrySnapshot` values;
- source geometry and topology revision stamps;
- coordinate-space descriptors;
- target transform dependency stamp;
- relevant hierarchy/ancestor dependency stamps;
- explicit time-sample descriptor and sampled animatable values;
- native upstream state stamps only where an accepted algorithm explicitly consumes them;
- FDM authored-model identity/revision;
- selected evaluation-stage identity;
- explicit composition graph stamp, only when a validated graph is supplied.

The snapshot bundle must be internally consistent: payload, target generation, revisions, coordinate descriptor and dependency stamps all represent the same captured state. It is immutable for the lifetime of evaluation.

A host document-wide `StateId` may be used as an additional conservative invalidation token, but it does not replace typed target and dependency revisions.

### 6.3 Canonical source and native Asset Warp

For the currently audited AniCraft model, the source snapshot rule documented by M1 remains:
- unwarped shape: canonical authored `Shape.path`;
- shape with native Asset Warp: canonical pre-warp source `Warp.rest`.

The already-bent/shown path must not be mislabeled as target-local rest geometry.

Native Asset Warp pin state is a separate host-owned dependency. It is not folded into the Path2D fingerprint. This contract does not decide whether FDM consumes native pin state or how native Asset Warp and FDM operations compose. Targets for which the two deformation systems would both affect the same geometry must be rejected or routed through a separately accepted composition contract until those semantics are specified. No double deformation is allowed.

### 6.4 Dependency separation

The host exposes distinct dependency stamps for at least:
- source geometry;
- source topology;
- target transform/coordinate conversion;
- hierarchy and relevant ancestor transforms;
- explicit time sample and sampled pose;
- native Asset Warp state, if consumed;
- upstream evaluated output, if an explicit dependency graph requires it;
- FDM authored model and composition graph;
- evaluation stage and capability identity where relevant.

A transform-only edit advances transform dependency state, not source geometry revision. Undo and redo restore authored values but advance affected revision epochs; revisions never move backward.

## 7. Evaluation request and prepared result

### 7.1 Request

AniCraft invokes the FDM evaluator with a complete, immutable request. Conceptually:

```text
FdmEvaluationRequest {
  contract_version,
  request_id,
  evaluation_input_bundle,
  authored_model_snapshot,
  explicit_time_sample,
  requested_output,
  resource_budget
}
```

The request ID is unique within the host evaluation session and is for correlation only; it is not a persistent identity or freshness token.

FDM must not query the host, read current selection/playhead, resolve IDs by display name, access mutable document objects, or substitute omitted dependencies. Missing or contradictory inputs return typed errors.

### 7.2 Result

A successful evaluation returns a prepared result envelope containing:
- result schema/version and algorithm identity/version;
- exact target reference and complete source snapshot stamps;
- all consumed non-geometry dependency stamps;
- authored model and composition graph stamps, if applicable;
- explicit time-sample identity;
- output payload kind/schema and coordinate-space descriptor;
- operation semantics (replacement geometry or typed contribution);
- canonical payload fingerprint and diagnostics/provenance.

The result is immutable and detached from mutable host objects. It does not contain a host mutation command. It cannot be applied twice: the host acceptance/application record must identify the accepted result and operation transaction.

### 7.3 Operation semantics

The initial live host path supports only a validated evaluated-geometry replacement for a target when that replacement is the complete result at the designated FDM evaluation stage.

A contribution is not automatically additive or sequential. It requires a validated, versioned composition graph with explicit ordering, coordinate/reference compatibility and type signatures. Ambiguous multi-source results fail closed.

CCC-01.5 takes precedence over general composition capability: Smart Controller and Form Deform pin outputs remain separate.

### 7.4 Determinism

For identical authored model, immutable input bundle, algorithm/version and explicit time sample, FDM must return semantically equivalent output and deterministic fingerprints. Host scheduling order, task completion order, selection state and renderer backend must not change the result.

## 8. Shared evaluated-geometry integration point

### 8.1 Required location

FDM output enters the host evaluation pipeline after canonical source geometry and relevant host transforms/samples are resolved, but before the shared evaluated geometry is consumed by:
- hit testing and selection geometry;
- bounds and spatial queries;
- stage rendering;
- player evaluation;
- built-in rendering and export;
- plugin export frame evaluation.

The integration must not be implemented only inside the rasterizer, stage, or one export path. All consumers must observe the same accepted evaluated geometry for the same document/time context.

### 8.2 Logical pipeline

```text
AniCraft Document + explicit frame/context
          |
          v
Host resolves hierarchy, transforms and canonical target snapshots
          |
          v
Host captures immutable FDM EvaluationInputBundle
          |
          v
FDM evaluates request (pure, deterministic, read-only)
          |
          v
Host validates target generation + every dependency + capabilities
          |
          v
Accepted evaluated-geometry overlay / resolved target geometry
          |
          v
Shared DisplayList / evaluated scene construction
          |
          +--> hit testing and bounds
          +--> stage rendering
          +--> player
          +--> built-in and plugin export
```

The phrase “overlay” describes a logical evaluated view, not a second persistent geometry store. It must be keyed to the exact evaluation context and discarded/rebuilt when dependencies change.

### 8.3 No mutation during frame evaluation

The shared evaluated-geometry stage may substitute the prepared geometry in the ephemeral evaluated view. It must not write to `Shape.path`, `Warp.rest`, native pin state, keyframes or persistent FDM authored records.

A consumer must never see a partially applied set of target outputs. The host constructs a complete accepted evaluation view or rejects the affected evaluation according to a documented target-group policy. The initial policy is fail closed for a request's declared atomic target set.

## 9. Scheduling, freshness and cache

### 9.1 Scheduling

AniCraft owns scheduling, prioritization, task lifecycle, cancellation and publication. FDM owns pure evaluation and may expose cooperative cancellation checkpoints, but cancellation does not establish freshness.

Each task captures:
- a unique request ID;
- document/session instance identity;
- full target references and generations;
- immutable dependency stamps;
- evaluation-context identity;
- cancellation token owned by the host.

### 9.2 Acceptance gate

Before publication, AniCraft re-resolves the target and compares every captured dependency against current state. Acceptance requires:
1. same document/session context where required;
2. exact target identity and generation;
3. unchanged geometry and topology revisions/fingerprints;
4. unchanged transform, hierarchy and sampled-time dependencies;
5. unchanged authored model, composition and upstream-result dependencies;
6. supported output schema, coordinate space, operation and target capability;
7. valid result fingerprint and complete target set.

If any check fails, the result is discarded as stale or incompatible. It must not enter the current evaluated scene or be applied by a later consumer. A completed obsolete task may be cached only under its full old dependency key; it can never be selected as current output.

### 9.3 Cache ownership and key

The cache belongs to AniCraft. FDM may supply canonical fingerprints and dependency descriptors but does not own document cache lifetime or invalidation.

A cache key includes:
- document ID and target ID/generation;
- target kind/capability identity;
- geometry and topology revisions/fingerprints;
- transform and hierarchy dependency stamps;
- explicit time-sample identity and sample policy;
- native Asset Warp/upstream result stamps if consumed;
- FDM authored-model and composition-graph revisions;
- algorithm, request/result schema and evaluation-stage versions;
- requested output semantics and relevant resource/policy settings.

A hit is reusable only if the complete dependency key matches and the acceptance gate still succeeds. Document StateId may be an additional conservative key component, not the sole key. Partial keys and fingerprint-only freshness are prohibited.

### 9.4 Invalidation

Any changed consumed dependency invalidates affected results. Invalidation propagates through explicit upstream/downstream deformation dependencies. Unrelated target edits need not invalidate independent results if the host can prove dependency isolation; otherwise conservative document-wide invalidation is safe.

## 10. Apply, bake, transactions and undo/redo

### 10.1 Preview and apply

Preview and live frame evaluation are read-only. “Apply” is an explicit host command that preflights the prepared result and performs one document transaction.

Preflight validates the complete target set, all source/dependency stamps, output schema/coordinate space, target capability, payload integrity, resource limits and duplicate-application guard. A failed preflight performs zero writes.

A successful apply records the exact source and output provenance and creates one coherent undo step. Commit failure rolls back the whole operation. Partial target success is not permitted for an operation declared atomic.

### 10.2 Retained, non-destructive bake

The accepted ADM-006 default is retained, non-destructive bake into AniCraft-native geometry. The host stores enough provenance to identify:
- source target identity/generation and source revisions;
- FDM authored model and topology revisions;
- algorithm, schema and composition versions;
- explicit frame/time or finite ordered frame range;
- generated native output identity/fingerprint;
- retained source/restoration record.

Bake preflights the entire target/frame set before mutation. Commit is atomic and deterministic. Removing/reverting the bake restores the pre-bake authored state from persistent retained source data, not an in-memory snapshot. A baked output must not be fed back through the same deformation stack.

The exact native geometry representation, frame-range storage strategy and transaction API are unresolved host design choices. This contract does not assume AniCraft's existing Envelope or Asset Warp bake is equivalent.

### 10.3 Undo and redo

Undo/redo is host-owned:
- authored edits are host document commands;
- preview results are not history entries;
- apply/bake is one coherent transaction/history step;
- undo restores authored source and extension payloads and removes the applied bake;
- redo deterministically restores the committed bake and provenance;
- all affected dependency epochs advance monotonically after undo/redo.

Restoring old values must never restore old revision counters or permit a stale asynchronous result to become current again.

## 11. Error and diagnostic contract

The host and FDM must preserve distinct error classes:
- unsupported host/contract/schema version;
- missing document, target or target generation mismatch;
- unsupported target kind/capability;
- unavailable or inconsistent snapshot;
- malformed geometry, coordinate descriptor or fingerprint;
- stale geometry/topology revision;
- stale transform/hierarchy/time/upstream dependency;
- missing or incompatible authored model;
- invalid/cyclic composition graph;
- unsupported output kind, coordinate space or operation;
- resource-budget rejection;
- cancelled evaluation;
- stale-result rejection;
- apply/bake preflight failure;
- commit failure and rollback failure;
- persistence/migration/quarantine failure.

Every diagnostic identifies the failing target/dependency and whether retry is safe. Cancellation, staleness and unsupported capability are not interchangeable. No silent fallback, partial success, automatic rebase or target substitution is allowed.

## 12. Host conformance gates

The existing `M1_HOST_CONFORMANCE_TEST_MATRIX.md` remains the canonical test inventory. This contract adds the following integration-level grouping and execution order.

| Gate | Required evidence | Must pass before |
|---|---|---|
| H0 — Version/capability | Handshake compatibility, limits and unsupported-version tests | Loading/evaluating FDM data |
| H1 — Persistence | Save/reopen, duplicate/import remapping, unknown-schema preservation and migration tests | Enabling authored FDM documents |
| H2 — Snapshot | Atomic identity/revision/payload capture, immutable snapshot and coordinate tests | Any FDM evaluation |
| H3 — Evaluation | Explicit time sampling, deterministic result and typed error tests | Live preview |
| H4 — Shared geometry | Same output in hit testing, bounds, stage, player and every export path | Declaring live integration complete |
| H5 — Freshness/cache | Dependency-specific invalidation, stale task rejection, old-key cache isolation | Asynchronous evaluation or cache reuse |
| H6 — Transactions | Zero-write preflight rejection, atomic multi-target/range rollback, one-step undo/redo | Apply/bake |
| H7 — Bake recovery | Retained provenance, exact source restoration and deterministic redo after save/reopen | Production bake support |

Every report records exact AniCraft commit SHA, Forma Deform commit SHA, host build, contract version, test IDs, results and skipped cases. FDM unit tests and WASM builds do not establish host conformance.

## 13. Implementation sequence after contract acceptance

1. Pin and revalidate the AniCraft source revision and all host API assumptions.
2. Implement host capability/version handshake and target capability reporting.
3. Implement document extension persistence and identity/revision semantics.
4. Implement atomic immutable snapshot capture and explicit timeline sampling.
5. Add the shared evaluated-geometry integration stage for synchronous deterministic evaluation.
6. Add result freshness validation and dependency-aware cache integration.
7. Add asynchronous scheduling only after stale-result tests pass.
8. Implement explicit apply transaction and retained bake with undo/redo.
9. Run the full host conformance matrix across all shared geometry consumers.

The first vertical slice should be one supported 2D Path2D target, one explicit time sample, one FDM result, synchronous evaluation and consistent shared DisplayList consumption. It must not include composition, nonlinear nested propagation, native Asset Warp blending, or multi-frame baking unless those are separately implemented and tested.

## 14. Open decisions — not silently fixed here

The following require host-source revalidation and explicit review:
- persistent AniCraft document UUID and target-generation allocation strategy;
- target-level revision storage and all mutation paths that advance it;
- exact canonical source stage for every supported target kind;
- coordinate transform and hierarchy snapshot representation;
- host extension record encoding, size limits and unknown-field preservation;
- exact shared evaluator hook and whether it is synchronous or deferred;
- target-set atomicity and failure isolation policy;
- task cancellation and publication scheduling details;
- native bake output representation and frame-range transaction model;
- whether any native Asset Warp + FDM coexistence path can be supported without a separately approved composition contract.

## 15. Evidence and audit limitation

The preceding FDM-06.1 audit records AniCraft source paths and observed behavior for the document model, pure evaluator, session cache, render service, plugin bridge, command transactions and history. The public host source was not independently retrievable during the current design pass, and the earlier audit did not pin an exact AniCraft commit SHA.

Accordingly, this document treats those findings as audit evidence, not as a substitute for current-source verification. Before host implementation, the source audit must be refreshed against a recorded AniCraft commit. If current source differs, this contract must be reconciled before code is written.

## 16. Scope and authorization

This is a design artifact only. It does not:
- change AniCraft source or document schema;
- implement the host adapter, evaluator hook, cache, scheduler, UI, command or bake;
- change FDM algorithms or topology semantics;
- combine Smart Controller and Form Deform pin outputs;
- replace native Asset Warp;
- authorize a PR merge.

Any AniCraft implementation requires a separate reviewed change and explicit authorization. Acceptance of this design must also be explicit before implementation begins.
