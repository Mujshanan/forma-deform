# CCC-01.8 — FDM v2 Readiness Assessment

- **Milestone:** CCC-01.8 — FDM v2 Readiness Assessment
- **Status:** Assessment drafted; FDM v2 is **not ready for feature implementation**
- **Assessment branch:** `docs/ccc-01-8-fdm-v2-readiness`
- **Base:** CCC-01.7 head `d610a52334ca4b9a2c3593589bb1c01be174e05f`
- **Compatibility authority:** `docs/CODE_COMPATIBILITY.md` (CCC-COMP-001)
- **Evidence input:** `docs/COMPATIBILITY_VERIFICATION.md`
- **Architecture baseline:** FDM-ARCH-002 (accepted)
- **Decision gate:** ADM-001 through ADM-007 (unresolved)

## 1. Executive assessment

FDM-ARCH-002 establishes the accepted direction for Forma Deformation Model v2:

- A host-independent Rust deformation core.
- Deterministic, non-accumulating evaluation.
- Separate authoring, evaluation, and baking responsibilities.
- An extensible model intended to support pins, multi-point deformation, lattice deformation, and nested deformers.
- AniCraft-specific integration through adapters rather than dependencies in the reusable core.
- Rigby2D is a reference for study, not an implementation dependency or an instruction to copy its architecture.

This is an architectural baseline, not a complete implementation specification. ADM-001 through ADM-007 remain unresolved. The approval of FDM-ARCH-002 does not resolve those decisions, authorize repository implementation, or authorize changes to AniCraft.

**Readiness verdict: NOT READY for FDM v2 feature implementation.**

The project is ready to continue architecture decision work. It is not ready to implement the canonical FDM v2 document model, evaluator, persistence, playback integration, or baking pipeline because the required ADM decisions and several host/runtime constraints remain open.

This verdict does not invalidate the CCC-01 consolidation work. CCC-01 and FDM v2 have different exit criteria.

## 2. Repository and compatibility evidence

### 2.1 CCC-01 state at assessment

The assessment is based on the CCC-01.7 branch, not on `main`. The stacked CCC pull requests remain review artifacts; this document does not merge or retarget them.

CCC-01.7 CI run [44902683475](https://github.com/Mujshanan/forma-deform/actions/runs/44902683475) completed with conclusion `success`. It passed:

- Bun typecheck, tests, and build.
- Adapter-boundary dependency check.
- `cargo test --workspace`.
- Release WASM builds for Bone Query, Form Deform, and Smart Controller.

This establishes automated evidence for the tested code paths and build targets. It does not establish live AniCraft behavior, SDK memory-safety, historical external-consumer compatibility, or formatting compliance.

### 2.2 Evidence that remains incomplete

| Area | Current evidence | Readiness consequence |
|---|---|---|
| Rust core and controller tests | Automated tests pass on CCC-01.7 | Useful baseline; not a complete FDM v2 specification |
| Public API and BindingSet v1 | Compatibility tests and schema round-trip coverage | No archived production fixture corpus or baseline-vs-head differential harness |
| Adapter dependency direction | CI boundary script passes | Static dependency enforcement does not prove runtime behavior |
| AniCraft plugin loading | WASM release builds pass | Real host loading remains unverified |
| Form Deform apply/undo | Adapter source and documented smoke-test steps | Live host mutation and undo behavior remain unverified |
| Smart Controller preview-only behavior | Core tests and source review | Live absence of host mutations remains unverified |
| Bone Query read-only behavior | Source review and parsing tests | Live read-only behavior remains unverified |
| SDK ABI safety | Native stand-ins and build coverage | Pointer/length/allocation and malformed-response audit remains outstanding |
| Historical compatibility | No archived external consumer corpus identified | Cannot claim universal historical compatibility |
| Formatting | Not independently checked by this CI workflow | Not verified |

The outstanding runtime procedures are recorded in `docs/COMPATIBILITY_VERIFICATION.md` and `plugins/INSTALL.md`. Host smoke tests should record the AniCraft version, plugin artifact hashes, test project, exact steps, and observed results.

### 2.3 Existing documentation conflict

There is a material documentation inconsistency that must be corrected before claiming a reliable compatibility baseline:

- `docs/CODE_COMPATIBILITY.md`, `docs/ADAPTER_BOUNDARIES.md`, and `docs/SMART_CONTROLLER_PLAN.md` describe Smart Controller V1 as preview-only, with no timeline or document mutation.
- `plugins/INSTALL.md` currently instructs the tester to enable Apply and verify that Smart Controller moves a target pin and that Undo restores it.

The latter conflicts with the accepted CCC-01.2 contract and the V1 preview-only boundary. Treat this as a documentation defect to reconcile by source review; do not infer authorization to add Smart Controller mutation behavior.

## 3. FDM v2 architecture readiness

| Concern | Accepted direction | Remaining readiness question |
|---|---|---|
| Core ownership | Host-independent Rust core | Exact public crate/API boundaries and versioning |
| Evaluation | Deterministic, non-accumulating | Complete evaluation inputs, revision model, invalidation, and failure contract |
| Authoring | Separate from evaluation | Canonical authoring document and persistence contract |
| Baking | Separate from live evaluation | Output representation and reproducibility contract |
| Deformer extensibility | Pins, multi-point, lattice, nested deformers | Stable common abstraction and per-deformer data contracts |
| Host integration | AniCraft adapters outside core | Actual host capabilities, required host extensions, and versioned adapter contract |
| Runtime | Minimal playback representation independent of authoring | Exact runtime payload and how it is produced/updated |
| Identity and geometry | No final ownership model accepted | Stable identity domains, geometry source of truth, coordinate semantics |
| Serialization | CCC protects existing schemas | No canonical FDM v2 schema or migration contract yet |

The current shared `Vec2` is intentionally only an `x/y` value primitive. It does not distinguish point from displacement, assign units, or encode coordinate space. The current pin evaluator uses stage-space values. These are compatibility constraints, not a complete FDM v2 geometry model.

The existing controller BindingSet v1 and Smart Controller plugin `Params` are also distinct contracts. Neither should be silently promoted to the canonical FDM v2 authoring schema.

## 4. ADM-001 through ADM-007 decision gate

The seven topics below are the working mapping of the seven deferred architecture questions explicitly listed in CCC-COMP-001. Confirm their exact titles and wording against the originating ADM decision register before closing any ADM item. All seven remain **unresolved** in this assessment.

| Decision | Working topic | Question that must be decided | Status |
|---|---|---|---|
| ADM-001 | Geometry ownership | Does the deformation document own its geometry, or reference geometry owned by the host's canonical document? Define identity, lifetime, synchronization, and stale-reference behavior. | Unresolved |
| ADM-002 | Lattice topology | Is a lattice a regular rectangular grid, an irregular control mesh, or a versioned abstraction supporting both? Define topology identity and interpolation contract. | Unresolved |
| ADM-003 | Nested deformer semantics | Are child deformers evaluated in parent-local coordinates or against the evaluated parent output? Define transform order, coordinate conversion, and deterministic traversal. | Unresolved |
| ADM-004 | Multiple-source composition | How do bone, controller, lattice, and other sources affect the same geometry: additive, override, blend, ordered stack, or another explicit model? Define ordering and conflict behavior. | Unresolved |
| ADM-005 | Animation ownership | Are animation keys owned by the deformation document or by the host's main timeline? Define time/frame mapping, references, and edit ownership. | Unresolved |
| ADM-006 | Baking representation | Does baking store absolute positions, displacements, or both? Define source revision, target timeline, precision, repeatability, and undo/migration behavior. | Unresolved |
| ADM-007 | Runtime representation | What is the minimal canonical representation needed for deterministic playback without the authoring system? Define versioning, evaluation inputs, and compatibility guarantees. | Unresolved |

No option in this table is selected or recommended as an accepted decision. Each ADM item requires a separate decision record with alternatives, trade-offs, consequences, dependencies, and explicit approval before implementation.

## 5. Host capability and product-boundary risks

The existing AniCraft SDK capability audit (`docs/ANICRAFT_SDK_AUDIT.md`) and plugin smoke-test guide establish important constraints:

1. Command plugins can query bone and warp state on demand, but the documented command-plugin API does not provide a continuous custom evaluator during playback or scrubbing.
2. The documented plugin contract does not provide durable arbitrary plugin-owned document metadata.
3. The host does not provide a verified command for setting a nested graphic symbol instance's displayed frame.
4. A frame script is not a substitute for a continuous deformation evaluator: it runs at keyframe entry, starts a fresh WASM instance, and cannot call the bone/warp command APIs.
5. `warp.pin_move` is a document mutation and must remain inside the host's explicit command preview/undo semantics.
6. The guest SDK is a host-specific ABI boundary. Its unsafe memory handling requires a separate focused audit.

These constraints mean that an AniCraft adapter alone cannot currently be assumed to provide durable authoring storage or live FDM v2 playback. If those capabilities are required, a separately reviewed AniCraft host-extension proposal will be necessary. This assessment does not authorize changes to the AniCraft repository.

## 6. Readiness gates

FDM v2 feature implementation must not begin until the following gates are satisfied.

### Gate A — CCC compatibility baseline

- [x] CCC-01.7 automated CI completed successfully.
- [x] Existing compatibility contract and evidence gaps are documented.
- [ ] Review and disposition of stacked CCC-01 PRs.
- [ ] Live AniCraft smoke tests for all three plugins.
- [ ] Reconcile the Smart Controller V1 documentation conflict.
- [ ] Complete or explicitly scope the SDK unsafe-boundary audit.
- [ ] Decide whether historical fixtures or downstream consumers can be obtained and tested.

A passing automated CI run is not a substitute for the unchecked items.

### Gate B — Architecture decisions

- [ ] Resolve ADM-001 through ADM-007 individually.
- [ ] Record decision dependencies and identify decisions that constrain the canonical document model.
- [ ] Define coordinate-space and point/displacement semantics consistently with the accepted ownership decisions.
- [ ] Decide the persistence boundary and host-extension requirements.
- [ ] Approve a versioned canonical FDM v2 document/schema contract.

### Gate C — Evaluation contract

- [ ] Define immutable evaluation inputs and the meaning of a document revision.
- [ ] Define time/frame input and deterministic evaluation behavior.
- [ ] Define dependency graph construction, traversal order, cycle detection, and error reporting.
- [ ] Define stable identity and missing/stale target behavior.
- [ ] Define multi-source composition and nested-deformer ordering from the approved ADM decisions.
- [ ] Define the separation between authoring data, evaluated output, and runtime data.

### Gate D — Baking and runtime contract

- [ ] Define baking output and source-revision provenance.
- [ ] Define whether runtime data is derived, cached, serialized, or regenerated.
- [ ] Define version compatibility and deterministic playback guarantees.
- [ ] Define invalidation and update behavior when authoring data changes.

### Gate E — Implementation plan

- [ ] Translate accepted decisions into crate responsibilities and public API contracts.
- [ ] Define acceptance tests and golden fixtures before feature implementation.
- [ ] Define adapter and host-extension milestones separately from the reusable core.
- [ ] Establish migration and compatibility strategy before changing existing public APIs or persisted data.

## 7. Recommended sequence after this assessment

1. Reconcile the ADM register with the seven working topics in Section 4.
2. Resolve ADM decisions in dependency order; do not implement around unresolved semantics.
3. Close the CCC-01 review and runtime-verification gaps without treating them as FDM v2 feature work.
4. Approve the canonical document, identity, coordinate, evaluation, and serialization contracts.
5. Approve a crate-level implementation plan with tests and migration strategy.
6. Only then begin the FDM v2 core implementation.
7. Plan AniCraft persistence/playback extensions as a separate host project, contingent on the accepted architecture and explicit authorization.

This sequence is a recommendation, not authorization to start the implementation or modify AniCraft.

## 8. Explicit non-goals of CCC-01.8

This milestone does not:

- Implement pins, multi-point, lattice, or nested deformation features.
- Define or implement multi-source composition.
- Change the existing pin formula or Smart Controller behavior.
- Introduce the canonical FDM v2 document or serialized schema.
- Add persistence, playback hooks, baking, or host commands.
- Change the AniCraft repository or SDK ABI.
- Merge CCC-01 pull requests or alter `main`.

## 9. Exit decision

CCC-01.8 completes its assessment objective when:

- The current compatibility evidence and remaining verification gaps are recorded accurately.
- The accepted FDM-ARCH-002 baseline is distinguished from unresolved ADM decisions.
- The seven decision topics and their dependencies are visible for formal review.
- Host/runtime capability constraints and documentation inconsistencies are recorded.
- Readiness gates and a decision-first implementation sequence are defined.

**Final assessment: FDM v2 is architecturally directed but not implementation-ready.** CCC-01.8 is an assessment/documentation milestone only. It does not close ADM-001 through ADM-007, approve feature implementation, authorize AniCraft changes, or authorize merging any pull request.
