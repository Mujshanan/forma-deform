# M1 — FDM Authored Persistence and Composition Graph Schema

**Status:** FDM-side v1 schema and graph validation implemented in PR #31; CI pending.  
**Ownership:** Forma Deform owns this payload schema and its validation. AniCraft owns embedding it in the authoritative document, save/load, dirty state, transactions and undo/redo.

## 1. Schema envelope

The first persisted FDM document format is:

- Schema name: `forma-deform-document`
- Schema version: `1`
- Stable FDM document ID and monotonically increasing FDM document revision
- Deformation definitions
- Explicit composition graphs

The host document format version is independent. Geometry payload schema versions are also independent. Unknown schema names/versions, unknown fields, malformed values and resource-limit violations fail closed.

V1 is the first persisted FDM format. There is no known legacy FDM document schema to migrate. The loader therefore validates v1 and rejects unsupported versions; it does not fabricate a v0 conversion. Every future version transition must have a named, deterministic migration function, explicit input/output versions, tests, and fail-closed behavior for data it cannot preserve.

## 2. Persisted deformation definition

Each definition stores:

- Stable FDM definition ID, revision and user-facing name
- FDM-owned deformation kind (pin, lattice, cage or mesh)
- Full host target reference, including namespace, document ID, target kind, target ID and generation
- Reference geometry stamp, including geometry/topology revisions and fingerprint
- Explicit coordinate-space descriptor
- Typed input and output value kinds
- Typed parameter map

Parameter values are Boolean, integer, finite number, text or finite 2D vector. Parameter keys are stable strings and stored in an ordered map for deterministic serialization.

A definition is invalid if its ID/name is empty, revision is zero, target identity is invalid, reference geometry belongs to another target, coordinate metadata is invalid, a parameter is non-finite, or parameter/resource limits are exceeded.

## 3. Runtime data excluded from persistence

Do not persist host pointers, plugin/session handles, live object references, snapshot buffers, evaluation requests, evaluated geometry results, transient diagnostics, caches, playhead state or preview state as authoritative authored data.

The host reference is a stable serialized identity token, not a persisted pointer. The host must remap document-scoped references on duplicate/import and must not silently bind a duplicate to the source document.

## 4. Composition graph

A graph is scoped to one target, one reference geometry stamp and one coordinate-space descriptor. It has its own stable graph ID, revision and graph schema version.

Node kinds:

- **Deformer:** references one persisted FDM deformation definition.
- **Sequential:** contains an explicitly ordered list of two or more node IDs. Each stage's input type must match the previous stage's output type.
- **Parallel:** contains two or more explicit branches evaluated independently from the same reference input. Branch input and output signatures must match.
- **Weighted blend:** contains two or more explicit node references with finite positive weights whose sum is 1 within the schema tolerance.

Graph output node IDs are explicit. Every node must be reachable from an output; orphan nodes are rejected. Node IDs and output IDs must be unique. Every reference must resolve. Cycles are rejected before evaluation.

### Compatibility checks

Every deformer referenced by a graph must match the graph's exact target identity, reference geometry stamp and coordinate-space descriptor. Sequential stages must have compatible adjacent input/output value types. Parallel branches must have identical input and output types. Weighted-blend inputs must have identical output types. Multiple graph outputs must agree on output type.

A graph cannot infer coordinate conversion, reference geometry, ordering, weight normalization or a missing node. Incompatibility is a validation error, not a fallback.

## 5. Composition semantics and exclusions

Independent evaluation is the default. Definitions not explicitly included in a composition graph remain independent.

Composition operators are only activated by explicit graph nodes:

- Sequential: explicit ordered application.
- Parallel: independent branch evaluation against the shared reference input; this does not itself imply merging.
- Weighted blend: explicit blending of compatible branch outputs using validated weights.

No creation-order composition, implicit addition, automatic blending, precedence rule or automatic source combination is permitted.

The graph schema contains only FDM deformation definitions. Smart Controller output is not a graph node/source. CCC-01.5 remains binding: Smart Controller binding evaluation and pin-deformation evaluation stay independent. Any future cross-system source combination requires a separate explicit semantic decision and authorization.

## 6. Resource limits and host boundary

V1 validation bounds the serialized document to 16 MiB, 10,000 deformation definitions, 1,000 composition graphs and 256 parameters per definition. Host adapters may enforce stricter limits before parsing or commit.

This implementation validates and serializes the FDM payload in memory. It does not implement AniCraft document extension storage, host save/load integration, dirty-state tracking, host reference remapping, transaction semantics or undo/redo. Those remain host-side acceptance gates and require separate authorization.

## 7. Acceptance coverage

FDM-side tests cover deterministic JSON round-trip, current-schema loading, unknown schema rejection, unknown-field rejection, non-finite parameter rejection, duplicate definition IDs, missing deformer references, explicit sequential order, weighted blending, cycle rejection, orphan/missing node rejection, coordinate-space mismatch and incompatible value types.

AniCraft host tests still need to prove document save/reopen, duplicate/import remapping, unknown-extension preservation/quarantine, migration atomicity, undo/redo and resource-limit enforcement in the real document lifecycle.
