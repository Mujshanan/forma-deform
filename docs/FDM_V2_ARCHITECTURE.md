# Forma Deform v2 — Architecture and Integration Specification

**Status:** Accepted architecture baseline; implementation details remain open  
**Decision set:** ADM-001 through ADM-007 accepted  
**Integration constraint:** CCC-01.5 accepted  
**Host:** AniCraft  
**Last consolidated:** 2026-11-22

## 1. Purpose and scope

Forma Deform (FDM) is AniCraft's deformation engine and plugin. It owns deformation-specific models, algorithms, controls, composition semantics, specialized interpolation, and deterministic evaluation.

FDM is not a second document engine, timeline, renderer, undo system, or general-purpose host runtime.

This specification consolidates the seven accepted architecture decisions and CCC-01.5. It is the architecture baseline for subsequent technical design. It does not define every algorithm or public API and does not, by itself, authorize implementation in AniCraft.

### Goals

- Preserve AniCraft as the sole authority for document and source artwork state.
- Keep FDM's domain model and algorithms independent of AniCraft implementation internals.
- Support multiple explicitly typed deformation topologies.
- Make identity, coordinate spaces, revisions, dependencies, and composition explicit.
- Evaluate deterministically from immutable inputs.
- Integrate deformation animation with AniCraft's timeline.
- Support non-destructive baking with provenance and complete undo/redo.
- Keep persistent authored data, runtime state, and results separate.

### Non-goals

FDM must not introduce a competing document model, source-geometry store, general animation engine, timeline, renderer, undo stack, command registry, plugin lifecycle, or independent cache lifecycle.

## 2. System responsibilities

### AniCraft owns

- Authoritative document state and persistence.
- Canonical artwork, source geometry, and source topology.
- Stable element and target identities.
- Host hierarchy and ordinary transforms.
- Native Asset Warp and its authored pins.
- Timeline, playback time, tracks, keyframes, and animation persistence.
- Host geometry snapshots and sampled animation inputs.
- Evaluation scheduling and host lifecycle coordination.
- Cache lifecycle and invalidation coordination.
- Validation and application of FDM results to the evaluated document.
- Rendering, export, document commands, transactions, undo/redo, and baking.
- Plugin lifecycle and host integration.

### Forma Deform owns

- Deformation-specific persistent models and schemas.
- Deformation specifications and parameters.
- Additional lattice and cage topology.
- Control-point identities, connectivity, and deformation domains.
- Source-to-deformation mappings.
- Deformation algorithms and their mathematical semantics.
- Smart Controller definitions, bindings, and controller-specific validation.
- Corrective poses and deformation-specific contributions.
- Explicit composition graphs and nested deformation semantics.
- Deformation-specific animatable property definitions and specialized interpolation.
- Prepared evaluators, intermediate buffers, and deformation runtime models.
- Deformation-specific diagnostics and bake requirements.

### Integration boundary

The AniCraft–FDM boundary must be versioned, explicitly typed, implementation-independent, and based on stable identities and revisions.

It must communicate coordinate spaces, target references, geometry snapshots, sampled animation values, compatibility information, evaluation results, and diagnostics.

FDM consumes contracts and snapshots, not AniCraft's internal geometry objects. AniCraft validates and applies returned results.

## 3. Accepted architecture decisions

### ADM-001 — Geometry ownership
**Status: Accepted**

Adopt hybrid geometry ownership.

AniCraft owns canonical source artwork, source geometry and topology, stable element/target identities, hierarchy, ordinary transforms, native Asset Warp data, and geometry revisions/fingerprints.

FDM owns additional deformation-specific geometry structures, including lattices, cages, deformation domains, prepared representations, and algorithm-specific intermediate data.

FDM must not maintain a competing authoritative copy of AniCraft artwork or document geometry.

A host integration adapter resolves stable target references and supplies immutable, revisioned geometry snapshots with explicit coordinate-space information. FDM evaluates against those snapshots and returns revision-associated results. AniCraft validates and applies those results.

AniCraft remains responsible for document mutation, persistence, rendering, undo/redo, and baking. Reuse native Asset Warp wherever sufficient. Introduce FDM-specific structures only for capabilities the host does not provide.

### ADM-002 — Lattice topology
**Status: Accepted**

FDM owns additional deformation topology. Its model is extensible and supports multiple explicitly identified topology types, initially including regular structured lattices and irregular cages.

Both generated and directly authored topology are supported.

Each topology has a stable identity, type, schema version, stable control-point identities, explicit connectivity, declared coordinate space, stable host target reference, source geometry revision/fingerprint, and topology-specific validation.

Control-point identity is independent of array position. Connectivity is explicit.

Source-to-deformation mapping is explicit and may be topology-specific or algorithm-specific. A universal mapping algorithm is not required.

Structural topology changes create new topology revisions and trigger compatibility validation of dependent deformation data and animation. Animation references stable control-point identities, never array indices. Incompatible data must not be silently reinterpreted.

FDM defines persistent topology schemas; AniCraft owns document storage and lifecycle. Runtime topology representations are disposable and reproducible.

### ADM-003 — Nested deformation semantics
**Status: Accepted**

Adopt hierarchical parent-driven deformation propagation:

- Parent deformation propagates to descendants.
- Child deformation does not affect ancestors or siblings.
- Each target's deformation is authored in its own local rest space.
- Child results are carried through applicable parent hierarchy and deformation mappings.
- Containment and evaluation dependency are distinct; containment does not create arbitrary dependencies.
- Hierarchy and dependency relationships must be acyclic.
- Stale or incompatible references fail closed with diagnostics.

Multiple operations on the same target require explicit deterministic composition and ordering. Do not assume nonlinear deformation mappings have inverses. Do not assume AniCraft's current shape-specific Asset Warp implements group deformation.

AniCraft owns host hierarchy, transforms, scheduling, persistence, undo, and application. FDM owns deformation semantics and algorithms.

### ADM-004 — Multiple-source composition
**Status: Accepted**

Deformation sources targeting the same geometry evaluate independently by default.

FDM supports an explicit composition graph with sequential composition, parallel composition, and weighted blending subject to compatibility rules.

Every composition operation declares and validates input/output types, coordinate spaces, reference geometry, parameters, dependencies, and ordering.

No implicit addition, blending, sequential execution, creation-order composition, or undeclared dependency is permitted.

Invalid, stale, incompatible, or cyclic composition relationships fail with diagnostics. General composition support does not automatically authorize every possible source combination.

CCC-01.5 remains a specific restriction: Smart Controller binding evaluation and Form Deform pin-deformation evaluation remain independent. Their outputs must not be combined until a separate explicit FDM v2 decision authorizes those semantics.

### ADM-005 — Animation ownership
**Status: Accepted**

Adopt shared ownership with specialized FDM interpolation.

AniCraft owns the authoritative timeline, playback/time, tracks, keyframes, animation persistence, sampling, and scheduling.

FDM owns deformation-specific animatable property definitions, semantics, validation, and specialized interpolation requirements.

AniCraft stores deformation animation data through supported document/extension boundaries, samples values, and supplies them through an explicit evaluation contract.

FDM evaluates deterministically without mutating authored animation data. It does not create a separate timeline or general-purpose animation engine.

Topology-dependent animation references stable control-point identities.

### ADM-006 — Baking representation
**Status: Accepted**

The default is retained, non-destructive baking into AniCraft-native artwork/geometry representations.

- Original deformation sources are retained by default.
- Original deformation definitions and animation remain recoverable.
- Explicit frame-range baking is supported where the host permits it.
- Baked output must not cause the same deformation stack to be applied twice.
- Provenance records source revision, target identity, evaluation time/range, and relevant deformation configuration.
- Baking is transactional.
- Undo restores the pre-bake state; redo restores the committed bake deterministically.
- Source restoration is supported through retained source information.
- Unsupported geometry fails with diagnostics rather than silently degrading.

AniCraft owns bake commands, document mutation, persistence, transactions, and undo/redo. FDM owns deformation evaluation and deformation-specific bake requirements.

### ADM-007 — Runtime representation
**Status: Accepted**

Persistent deformation data, runtime evaluation state, and evaluation results remain separate.

AniCraft owns authoritative document state, persistent extension storage, scheduling, geometry snapshots, host lifecycle coordination, cache lifecycle, and result application.

FDM owns internal runtime deformation models, prepared evaluators, intermediate buffers, and algorithms.

The runtime contract is versioned and implementation-independent. Inputs are immutable. Results identify target, source revisions, coordinate space, and compatibility.

Runtime state and results do not mutate authoritative authored data. Prepared state and caches are disposable and reproducible from persistent data and explicit inputs.

Dependency-based invalidation covers source geometry, topology, transforms, hierarchy, animation, composition, and evaluator versions.

Asynchronous/concurrent evaluation must be deterministic, reject stale results, and prevent partial application. Runtime handles and pointers are session-scoped and never serialized.

No CPU/GPU execution model, buffer layout, or allocator is mandated by ADM-007.

## 4. CCC-01.5 — Smart Controller and pin-deformation separation
**Status: Accepted integration constraint**

Smart Controller binding evaluation and Form Deform pin evaluation remain independent.

- Smart Controller V1 remains preview-only.
- Existing pin-deformation formula and behavior are preserved.
- A host-independent integration boundary may be defined.
- Outputs must not be combined until a separate FDM v2 decision explicitly authorizes the semantics.

This specific restriction is not overridden by ADM-004's general composition capability.

## 5. Unified data and evaluation model

The architecture separates four conceptual layers.

### Persistent authored data
Includes deformation definitions, topology definitions, stable control-point identities and authored coordinates, source mappings, composition graphs, hierarchy references, deformation animation properties and authored values, corrective poses, Smart Controller definitions, and version/provenance metadata.

FDM defines its schemas. AniCraft owns authoritative document storage through supported extension boundaries.

### Host geometry and animation inputs
AniCraft resolves the target and supplies immutable geometry snapshots, stable identities, revisions, coordinate-space information, and sampled animation values. These are evaluation inputs, not a second authoritative document.

### FDM runtime state
FDM prepares internal representations, mappings, evaluators, dependency state, composition state, and intermediate buffers. These are disposable and reconstructible.

### Evaluation results
FDM returns results associated with targets, source revisions, coordinate spaces, and compatibility information. AniCraft validates freshness and compatibility before application.

## 6. Evaluation lifecycle

1. AniCraft identifies the requested targets and evaluation context.
2. AniCraft resolves authoritative geometry and supplies immutable snapshots.
3. AniCraft samples applicable animation values.
4. FDM resolves authored deformation definitions and explicit dependencies.
5. FDM validates topology, mappings, coordinate spaces, and revisions.
6. FDM prepares or reuses compatible runtime representations.
7. FDM evaluates individual deformation sources independently.
8. FDM applies explicitly declared composition operations where authorized.
9. FDM evaluates nested deformation according to ADM-003.
10. FDM returns revision-associated results and diagnostics.
11. AniCraft validates freshness and compatibility.
12. AniCraft applies valid results to its evaluated document representation.
13. AniCraft manages rendering, cache lifecycle, and invalidation.

Implementations may optimize scheduling and preparation without changing these semantics.

## 7. Cross-decision invariants

1. **Authority:** AniCraft is the sole authoritative owner of document and source artwork state.
2. **Explicit semantics:** Topology, identity, coordinate spaces, mappings, dependencies, interpolation, and composition are declared rather than inferred.
3. **Stable identity:** Persistent references use stable identities, never array positions or runtime pointers.
4. **Revision safety:** Geometry, topology, authored deformation, animation, composition, and evaluator changes participate in compatibility and invalidation.
5. **Determinism:** Equivalent persistent inputs, explicit snapshots, sampled animation values, and compatible evaluator versions produce equivalent results.
6. **Fail closed:** Invalid, stale, incompatible, or cyclic data is diagnosed and not silently applied.
7. **Runtime isolation:** Evaluation does not mutate persistent authored data.
8. **Host ownership:** FDM does not take over host responsibilities to support internal representations.
9. **CCC-01.5:** Smart Controller and pin-deformation outputs remain independent unless separately authorized.

## 8. Implementation-level questions still open

The accepted decisions establish architecture, not every mathematical or implementation detail.

- **Geometry/topology:** exact snapshot and revision contract; supported source geometry types; initial lattice dimensionality/resolution; cage validity; per-topology mapping algorithms; rebase/migration; control-point insertion/deletion and identity migration.
- **Algorithms:** initial algorithm inventory and compatibility matrix; per-algorithm input/output and coordinate rules; numerical tolerances; degenerate-geometry behavior.
- **Composition/hierarchy:** composition graph schema; weighted-blending compatibility; parent-to-child mapping; unsupported parent mapping behavior; cycle diagnostics and ordering.
- **Animation:** animatable property schema; per-property interpolation; control-point deletion/migration; time representation and sampling precision; topology revision compatibility.
- **Baking:** supported native output types; retained-source representation; provenance schema; frame-range transaction/recovery; duplicate-application prevention.
- **Runtime/performance:** public API and versioning; cache keys and dependency fingerprints; cancellation/stale-result rejection; concurrency; memory ownership; CPU/GPU strategy; performance targets and benchmarks.

These items must not be treated as already decided.

## 9. Implementation readiness and next phase

The next phase is **M0 — AniCraft host capability and integration audit**.

The audit must establish actual support for document/geometry ownership, native deformation, identity/revisions, hierarchy/transforms, timeline/animation, plugin persistence, transactions/undo, rendering/result application, cache/evaluation scheduling, and Smart Controller/pin integration.

Each finding is classified as Existing capability, Reusable capability, Missing capability, FDM responsibility, or Open decision.

M0 outputs an evidence-linked capability matrix, integration gap list, host-extension candidates, responsibility map, and recommended implementation sequence.

Feature implementation follows the audit and definition of concrete contracts. A host extension, AniCraft change, or PR merge requires separate explicit authorization.

## 10. Governance and change control

Changes to accepted architecture semantics require explicit review and approval.

- ADM-004 is not permission to combine Smart Controller and pin-deformation outputs.
- FDM topology is not canonical source artwork topology.
- Native shape Asset Warp is not assumed to support general group deformation.
- FDM must not add a second timeline, document model, or undo system.
- Runtime handles and prepared state are not serialized as authored data.
- Incompatible topology or animation data is never silently migrated.

**This specification does not itself authorize implementation, AniCraft changes, or merging pull requests.**
