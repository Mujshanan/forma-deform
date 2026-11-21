# Code Compatibility Contract

- **Document ID:** CCC-COMP-001
- **Milestone:** CCC-01.2 — Compatibility Contract
- **Project:** Forma Deformation / AniCraft
- **Baseline revision:** `6b53aac51ddc15249c5100745726027fd9c75907`
- **Baseline branch:** `main`
- **Status:** Accepted compatibility contract
- **Created:** 2026-11-21

## 1. Purpose

This document defines the compatibility boundary for CCC-01 Code Consolidation and Compatibility. It records the existing behavior that must be preserved while reusable implementations are consolidated in preparation for Forma Deformation Model v2 (FDM v2).

The contract is intentionally conservative. It protects existing behavior and consumers; it does not declare every current implementation detail to be the final FDM v2 design.

**No implementation changes are authorized or specified by this document.** In particular, this milestone does not itself authorize changing vector types, controller bindings, plugin adapters, serialization code, or the AniCraft SDK.

## 2. Scope and authority

This contract applies to:

1. Rust workspace crates and their public APIs.
2. Controller binding evaluation and binding-set serialization.
3. Pin deformation evaluation.
4. AniCraft plugin behavior and the SDK/ABI boundary.
5. The separate TypeScript geometry, timeline, and document prototype.
6. Tests and build workflows used to establish compatibility.

### 2.1 Evidence baseline

The source baseline is the `main` commit listed above. The automated baseline was verified by GitHub Actions run [44658292627](https://github.com/Mujshanan/forma-deform/actions/runs/44658292627), which passed:

- Bun installation, TypeScript typecheck, tests, and browser build.
- `cargo test --workspace`.
- Release WebAssembly builds for Bone Query, Form Deform, and Smart Controller.

This is evidence of automated build/test success at the baseline revision. It is not evidence of a live AniCraft host smoke test or a complete ABI security audit.

### 2.2 Evidence labels

- **Observed:** behavior or interface present in the baseline source.
- **Preserve:** behavior this milestone requires consolidation work to retain.
- **Deferred:** behavior whose future semantics are not yet defined; consolidation must not invent them.
- **Out of scope:** behavior not to be added as part of CCC-01.

Where observed implementation behavior conflicts with documentation, the implementation and tests are the evidence of current behavior; the discrepancy must be documented and corrected deliberately.

## 3. Compatibility principles

1. **Behavior before structure.** Internal code may be reorganized only when observable behavior remains equivalent or an explicit breaking-change decision is approved.
2. **No silent semantic changes.** A refactor must not change coordinate interpretation, units, neutral-pose meaning, interpolation, clamping, easing, endpoint policy, identity, or error behavior implicitly.
3. **No premature deletion.** Existing implementations and compatibility paths must remain available until replacement behavior is demonstrated by regression tests and consumers have been migrated.
4. **Core remains host-independent.** Reusable deformation and controller logic must not acquire dependencies on AniCraft SDK or plugin implementations.
5. **Adapters own host interaction.** Host queries, host-specific coordinate conversion, commands, logging, and document edits remain in adapter/SDK layers.
6. **Serialization is a public boundary.** Persisted data must not be silently reinterpreted or rewritten into an incompatible shape.
7. **Compatibility is testable.** Every preserved behavior must have a corresponding test or an explicit verification procedure.
8. **FDM v2 is not retroactively imposed.** Unsettled FDM v2 architecture decisions remain unsettled until separately decided.

## 4. Compatibility contract by subsystem

### 4.1 Pin deformation core

**Observed baseline:** `form-deform-core` exposes `Vec2`, `DeformSpec`, `DeformError`, and `evaluate_pin_position()`. The current evaluator handles a single pin driven by an angular input and uses stage-space coordinates.

**Preserve:**

- The neutral position is authored input, not the previous evaluation's output.
- At the neutral angle, evaluation returns the neutral position.
- The existing absolute, non-accumulating formula remains:

  `P(theta) = P0 + G * (theta - theta0)`

  where `P0` is the neutral position, `G` is the configured response vector, and `theta0` is the neutral angle.
- Existing finite-input validation and arithmetic-overflow error behavior remain observable.
- Evaluation remains deterministic for identical inputs.

**Deferred:** generalized coordinate spaces, multiple simultaneous deformers, composition order, nonlinear deformation, and FDM v2 document ownership.

### 4.2 Controller and binding core

**Observed baseline:** `smart-controller-core` exposes controller and binding types through its public crate surface, including the legacy binding implementation. The legacy implementation includes stable binding/controller identities, input ranges and neutral values, linear and Smoothstep easing, absolute-pose and relative-delta modes, symbol-frame and geometry-deform targets, three-anchor interpolation, validation, and versioned binding-set serialization.

**Preserve:**

- Existing public types and callable entry points remain source-compatible during CCC-01 unless a reviewed migration explicitly replaces them.
- Input normalization retains current neutral-point, clamping, and range behavior, including reversed-range cases where currently supported.
- Linear and Smoothstep easing retain their existing results.
- Absolute-pose and relative-delta modes retain their current meanings.
- Three-anchor interpolation retains current anchor ordering and interpolation behavior.
- Disabled bindings retain their current non-producing behavior.
- Duplicate identity detection and current validation outcomes remain stable.
- Existing binding lookup and CRUD behavior remain stable.

**Important boundary:** controller binding evaluation and deformation evaluation are distinct responsibilities. Controller evaluation may produce a target value or offset; it must not silently become a second implementation of pin deformation.

**Deferred:** additive versus absolute composition across multiple sources, ordering among controllers/bones/deformers, and a canonical FDM v2 binding schema.

### 4.3 Controller configuration model

**Observed baseline:** the newer `Params` model represents plugin command/configuration data (including driver/action layer and bone choices, rotation limits, frame range, target bone/property, scale, and manual input). It is not equivalent to the richer legacy binding-set model.

**Preserve:**

- Existing plugin command/configuration behavior while the two models coexist.
- Do not assume that similarly named fields in `Params` and legacy bindings have interchangeable meanings.
- Do not delete or merge either model solely because both concern controllers.

**Deferred:** whether `Params` becomes an adapter-only configuration object, a projection of the binding model, or is replaced by a separately versioned configuration contract. That decision requires an explicit design review.

### 4.4 Mapping and preview

**Preserve:**

- Existing rotation normalization behavior.
- Existing frame mapping and endpoint policy behavior.
- Existing scale mapping behavior.
- Smart Controller V1 remains preview-only and does not write timeline keys or move deformation pins.
- Preview output and diagnostics remain deterministic for the same inputs.

**Known limitation:** `transform_value()` accepts an `additive` argument but does not currently implement composition through that argument. Do not assign new additive semantics during consolidation. Record and test current behavior until a separate composition decision is approved.

**API concern:** the public frame-mapping function has a debug assertion for interval ordering while its normal caller validates the interval. A future safe error contract or validated interval abstraction may be proposed, but changing the public signature requires a compatibility review.

### 4.5 AniCraft plugin adapters

**Form Deform — preserve:**

- Host data is queried through the SDK/adapter.
- The core calculation is used for the pin result.
- Applying a result remains an explicit host document edit.
- Existing undoable edit behavior is retained.
- No host-specific query or mutation logic is moved into the reusable core.

**Smart Controller — preserve:**

- V1 remains a preview workflow.
- No implicit timeline writes, pin movement, or document mutation are introduced.
- Documentation must accurately describe preview-only behavior. Instructions that imply an Apply control or pin movement in Smart Controller V1 are documentation defects, not authorization to expand V1.

**Bone Query — preserve:**

- It remains an independent diagnostic/read-only plugin.
- It is not made a runtime dependency of the deformation or controller core.

### 4.6 AniCraft SDK and ABI

**Preserve:**

- Existing ABI v1 command and response contracts.
- Existing SDK provenance/pinning and the separation between guest SDK and reusable core.
- Existing plugin entry points and host-facing behavior.
- Host-call stand-ins remain useful for native tests but are not treated as substitutes for real-host verification.

**Boundary rule:** the SDK may depend on host/ABI definitions. Reusable mathematical and controller crates must not depend on the SDK or plugin crates.

**Required future audit:** pointer and length validation, allocation and response-size limits, malformed host responses, and unsafe memory boundary handling. This security review is separate from semantic consolidation; do not change the ABI opportunistically.

### 4.7 TypeScript prototype

**Preserve:**

- The TypeScript geometry, timeline, and document modules remain independently buildable and testable.
- Existing rectangle point/corner operations, interpolation, hit-testing, and validation retain current behavior.
- Timeline integer-frame keyframes, snapshots, linear interpolation, endpoint holds, and FPS validation retain current behavior.
- Version-1 document serialization remains readable and retains its current meaning.

**Boundary rule:** the TypeScript prototype is a behavior/reference implementation, not the canonical Rust FDM v2 document model. CCC-01 does not require merging it into Rust or making the two implementations share internal types.

## 5. Public API compatibility policy

For the duration of CCC-01:

1. Treat all currently public Rust types, functions, modules, and re-exports as compatibility-sensitive.
2. Do not remove, rename, narrow visibility, or change the meaning of a public item as an incidental refactor.
3. Prefer additive APIs and adapters when introducing a consolidated implementation.
4. If a public item must be superseded, document:
   - the reason;
   - the replacement;
   - source migration steps;
   - behavioral differences, if any;
   - the deprecation period or explicit reason no deprecation period is possible;
   - regression and migration tests.
5. Do not claim strict SemVer guarantees solely from this document. Crate versioning and release policy must be checked separately.
6. Avoid exposing internal implementation details as new public API unless a consumer requirement justifies it.

## 6. Serialization compatibility policy

### 6.1 Controller binding set

- Treat the existing binding-set schema version 1 as a compatibility baseline.
- Existing valid version-1 data must continue to deserialize with the same meaning.
- Serialization changes must not silently rename fields, change numeric interpretation, change identity representation, or alter target semantics.
- Unknown-version behavior and malformed-data error behavior must be explicitly tested and documented before changing the parser.
- If a future schema is needed, introduce an explicit version and migration path. Do not reinterpret version 1 as a newer schema.

### 6.2 TypeScript document

- Treat the existing version-1 TypeScript document format as a separate compatibility baseline.
- Preserve existing version-1 read/write behavior.
- Do not conflate its document version with the Rust binding-set schema version or a future FDM v2 version.

### 6.3 No implicit persistence claims

Serialization support alone does not establish durable storage in AniCraft. Persistent host storage, autosave, revisioning, and document migration remain separate capabilities requiring explicit host integration.

## 7. Identity and coordinate compatibility

### 7.1 Identity

- Preserve the existing distinction between controller IDs, binding IDs, bone IDs, element IDs, pin IDs, and positional indices.
- Never substitute an array index for a stable identity.
- Preserve duplicate detection and lookup behavior.
- Any identity conversion between host and core representations must be explicit and tested.

### 7.2 Coordinates

- The current pin evaluator's stage-space assumption is an existing implementation constraint, not a complete coordinate-space model.
- Do not silently relabel stage-space values as local, parent, armature, deformer, or world-space values.
- Any conversion must identify source and destination spaces and be explicit at a boundary.
- A shared geometry type must not erase coordinate-space meaning.

## 8. Regression requirements for consolidation

A consolidation change is not compatibility-verified until the relevant tests cover the following.

| Area | Required regression evidence |
|---|---|
| Pin evaluator | Neutral pose, positive/negative angular delta, repeated evaluation, deterministic output |
| Numeric safety | NaN/infinity inputs and overflow/error paths |
| Controller normalization | Neutral value, min/max clamping, reversed ranges where supported |
| Easing | Linear and Smoothstep representative values and endpoints |
| Modes | Absolute-pose and relative-delta behavior |
| Anchors | Three-anchor ordering, endpoints, interpolation and out-of-range behavior |
| Identity | Duplicate IDs, stable lookup, CRUD and missing identity behavior |
| Binding serialization | Version-1 round trip, compatibility fixtures, invalid/malformed input |
| Mapping | Rotation normalization, frame endpoints, reversed/invalid interval behavior, scale mapping |
| Preview | Disabled binding, deterministic preview, no host mutation |
| Form Deform adapter | Explicit apply path and undoable edit behavior using host-call test doubles |
| Smart Controller adapter | Preview-only behavior; no timeline write or pin mutation |
| Bone Query adapter | Read-only diagnostic behavior |
| SDK | ABI response decoding, malformed lengths, allocation bounds and error handling |
| TypeScript | Existing geometry, timeline and document tests remain passing |
| Build | Native workspace tests and all three WASM plugin builds pass |

Tests should be added before or alongside a refactor, not after deleting the old implementation. Where a behavior is currently untested, first add a characterization test that records the current intended behavior; if current behavior is defective, record the defect and obtain a decision before changing it.

## 9. Breaking-change decision process

A proposed breaking change must be presented as a separate decision with:

1. Affected public APIs, serialized data, plugins, and downstream consumers.
2. Current behavior and the proposed behavior.
3. Why preserving the current contract is impractical or harmful.
4. Migration and compatibility strategy.
5. Regression tests and fixtures.
6. Documentation updates.
7. Approval before implementation.

A passing build alone does not approve a semantic change.

## 10. Explicitly deferred FDM v2 decisions

This contract does not resolve:

- Geometry ownership: deformation document owns geometry or references host canonical geometry.
- Lattice topology: regular grid or irregular control mesh.
- Nested deformer coordinate/evaluation semantics.
- Composition of multiple deformation sources.
- Animation-key ownership.
- Baking representation: absolute positions, displacements, or both.
- Minimal runtime representation independent of authoring.

It also does not define a final coordinate-space type system, durable host persistence, or a canonical FDM v2 serialized document.

These decisions must be made in their respective architecture reviews, not inferred from compatibility work.

## 11. CCC-01 exit conditions

CCC-01 may be considered compatibility-verified only when:

- Each Rust crate and plugin has a documented responsibility.
- Public API baselines are recorded.
- Binding-set and TypeScript document serialization baselines are recorded.
- The preserved behaviors in this contract have regression coverage.
- Shared primitive consolidation has a reviewed migration path.
- Controller consolidation has one authoritative implementation without losing supported behavior.
- Both deformation plugins consume the consolidated APIs without duplicating mathematical logic.
- AniCraft-specific behavior remains outside the reusable core.
- Native tests and all required WASM builds pass.
- Documentation accurately reflects plugin behavior and architecture.
- Remaining FDM v2 decisions and compatibility gaps are recorded.

This document alone does not satisfy CCC-01 exit conditions; it establishes the contract against which later work is judged.

## 12. Change history

| Date | Change |
|---|---|
| 2026-11-21 | Initial CCC-01.2 contract drafted from the source inventory at baseline revision `6b53aac51ddc15249c5100745726027fd9c75907`. |
