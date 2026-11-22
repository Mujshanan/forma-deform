# Code Reuse Registry

**Purpose:** record where each capability should come from before adding or porting implementation code.
**Scope:** Forma Deform modules, AniCraft host capabilities, and external candidate projects.
**Status:** initial registry, based on the repository structure and the source-level audit in `DEVELOPMENT.md`. A candidate is not an approval to copy code.

## Decision vocabulary

| Strategy | Meaning |
|---|---|
| **Reuse** | Call or consume an existing capability without duplicating its implementation. |
| **Adapter** | Translate between Forma Deform's contract and an existing host/library API; keep ownership with the provider. |
| **Port** | Copy a narrowly scoped implementation after provenance, license, dependency, and maintenance review. |
| **Reimplement** | Implement the required semantics independently, using references for design but not copying code. |
| **Reference** | Study behavior or algorithms; no code is imported. |
| **Build** | A demonstrated product-specific gap requires a new implementation. |
| **Research** | Compare alternatives before selecting a strategy. |

**Approval rule:** no external code is approved by this document. Approval requires exact source-file paths and revision, file-level license and notices, dependency/license closure, a documented reason not to use AniCraft's existing capability, tests, and (for performance-sensitive algorithms) comparative benchmark evidence.

## 1. Internal module registry

Paths below are current repository paths. Test references identify existing coverage areas, not a claim that every edge case is covered.

| Module / path | Responsibility | Existing implementation to reuse | Strategy | Dependencies / boundary | Test evidence | Status / next gate |
|---|---|---|---|---|---|---|
| `crates/smart-controller-core/src/mapping/range.rs` | Range normalization and directed ranges | Canonical range helpers | Reuse internally; extend only for new semantics | Pure core; no host dependency | `canonical_mapper_tests.rs` | **Implemented**; add tests before changing semantics |
| `crates/smart-controller-core/src/mapping/easing.rs` | Easing functions | Existing easing functions | Reuse internally | Pure core | `canonical_mapper_tests.rs` | **Implemented**; avoid parallel easing implementations |
| `crates/smart-controller-core/src/mapping/frame.rs` | Frame endpoint policy | Existing frame mapping helper | Reuse internally | Pure core; host frame conventions enter through adapter | `canonical_mapper_tests.rs` | **Implemented** |
| `crates/smart-controller-core/src/mapping/canonical.rs` | Canonical typed mapping, pose sets, five-pose joystick field | Canonical mapper and its typed interpolation helpers | Reuse / extend | Uses core model, values, pose library; no host mutation | `canonical_mapper_tests.rs` | **Implemented, M7 initial slice**; design grid interpolation separately |
| Planned `PoseGrid` mapping in `mapping/canonical.rs` | Bounded 2D/3D regular keyform grid | Reuse `Pose2D`, `PoseLibrary`, `TypedValue`; native multilinear implementation | Build; Aether Canvas is reference only | No new dependency; max 3 axes / 256 forms; exact typed binding validation | Evaluation plan: `FD-M7-MULTIAXIS-KEYFORM-EVALUATION.md` | **Approved for design; implementation pending** |
| `crates/smart-controller-core/src/pose.rs` | Runtime pose definitions/library | Existing pose lookup and validation | Reuse internally | Runtime-only; not durable persistence | `canonical_mapper_tests.rs` | **Implemented**; do not create competing pose stores |
| `crates/smart-controller-core/src/value.rs` | Typed scalar, angle, and vector values | Existing typed value semantics | Reuse internally | Pure core | Canonical mapper tests | **Implemented**; extend type semantics centrally |
| `crates/smart-controller-core/src/composition.rs` and `contribution.rs` | Contribution collection/composition | Existing contribution model and composition stage | Reuse internally | Keep Smart Controller and Form Deform dependency boundary explicit | Core tests; `dependency_boundary_tests.rs` | **Implemented**; no implicit cross-system composition |
| `crates/smart-controller-core/src/evaluator.rs` and `graph.rs` | Deterministic controller graph evaluation | Existing graph ordering, preflight, and evaluator | Reuse internally | AniCraft schedules lifecycle; core evaluates explicit inputs | Core integration tests | **Implemented**; do not add a second scheduler |
| `crates/smart-controller-core/src/validation.rs`, `diagnostic_codes.rs`, `error.rs` | Validation and diagnostics | Existing validation and diagnostic registry | Reuse internally | Pure core; host capability failures are adapter diagnostics | `diagnostic_registry_tests.rs` and validation tests | **Implemented**; add diagnostic codes centrally |
| `crates/smart-controller-core/src/persistent.rs`, `migration.rs`, `schema.rs` | Versioned plugin-owned data model and migration | Existing envelope, schema, migration helpers | Reuse internally | Host-owned document persistence still required | `canonical_document_tests.rs` and migration tests | **Core model implemented**; host storage is not implemented |
| `crates/form-deform-core/src/lib.rs` | Form Deform core contracts | Existing deformation types/contracts | Reuse internally | Keep independent of Smart Controller core unless an explicit composition contract is approved | Core tests and `dependency_boundary_tests.rs` | **Implemented foundation**; do not collapse crate boundary for convenience |
| `crates/anicraft-plugin-sdk/src/lib.rs` | AniCraft plugin API surface | Existing SDK wrappers and typed host calls | Adapter | AniCraft owns command registration, lifecycle, document and undo | Plugin adapter tests | **Reuse**; do not recreate host APIs |
| `plugins/smart-controller/src/targets.rs` | Host target resolution and geometry apply adapter | AniCraft `warp.info` and `warp.pin_move` commands | Adapter | AniCraft owns Asset Warp state, identity, validation, and mutation | Target adapter tests; GUI verification pending | **Implemented single-target path**; batch host API remains proposed |
| `plugins/smart-controller/src/lib.rs` | Plugin manifest, authoring/preview entry point, apply gate | AniCraft plugin command/settings workflow | Adapter | Host owns plugin lifecycle and undo transaction | Plugin tests; GUI smoke test pending | **Implemented first slice**; do not create parallel plugin framework |
| `docs/` | Host contracts and milestone specifications | Existing M3/M5/M7 specifications | Reference | Documentation only | Review against source and tests | **Maintain**; keep proposed APIs clearly marked as proposals |

### Internal duplication checks

Before adding an implementation:
1. Search the registry and source tree for an existing helper or contract.
2. Prefer extending the owning module over adding a second implementation in a plugin adapter.
3. Keep host calls at the adapter boundary; pure core modules must remain testable without AniCraft.
4. Preserve the dependency-boundary tests. A dependency shortcut is not an acceptable substitute for an explicit semantic contract.
5. If two modules legitimately need the same behavior, extract a shared pure abstraction only when its ownership and semantics are stable.

## 2. AniCraft capability reuse registry

AniCraft is the source of truth for application and document behavior. Forma Deform should adapt to these capabilities, not reproduce them.

| Capability | Provider / known interface | Forma Deform strategy | Do not build here | Verification gate |
|---|---|---|---|---|
| Bone identity and rig model | AniCraft canonical rig model / stable IDs | Adapter | A competing bone ID system or rig document | Verify stable-ID and deletion behavior against host source |
| Bone transforms and coordinate spaces | AniCraft transform APIs | Adapter | A second transform hierarchy | Confirm local/world conventions and numerical behavior |
| Constraints and transform inheritance | AniCraft constraint system | Reuse / Adapter | General-purpose constraint solver | Demonstrate a concrete missing constraint before research |
| Asset Warp geometry and pins | AniCraft `warp.info`, `warp.pin_move` | Adapter | Duplicate warp storage or mutation engine | Re-query target identity before mutation; test stale/missing pins |
| Timeline, frames, and playhead | AniCraft timeline | Adapter | Animation timeline or clip engine | Use host frame semantics and explicit frame context |
| Document persistence and migrations | AniCraft document/extension boundary | Adapter | Sidecar database or private document store | Host-approved extension schema, save/load, migration, undo/redo |
| Undo/redo and command transactions | AniCraft command/plugin transaction | Reuse | A plugin-owned undo stack | Verify rollback and one coherent undo step |
| Rendering, stage, panels, tool lifecycle | AniCraft UI/runtime | Reuse / Adapter | Parallel renderer, dock system, or plugin framework | Verify supported native extension points |
| Evaluation scheduling and cache invalidation | AniCraft evaluator/runtime | Adapter | A second playback scheduler or global cache | Implement live evaluation only against a verified host hook |
| Export and runtime playback | AniCraft | Reuse | Export/player pipeline | Host-supported extension point only |

## 3. External source candidates

These are research leads from the source-level audit in `DEVELOPMENT.md`. Repository-level license labels are preliminary metadata, not legal clearance for individual files or transitive dependencies.

| Capability / candidate | Candidate area | Proposed strategy | License and dependency gate | Correctness / test gate | Benchmark gate | Status |
|---|---|---|---|---|---|---|
| Smart Bone action sampling — [Fanimate](https://github.com/toonittmedia/fanimate) | Action sampling, stable bone IDs, pose snapshots/mirroring | Reference first; consider a narrow port | Apache-2.0 reported in repository metadata; verify exact file headers, revision, dependencies, notices | Map to canonical controller IDs and AniCraft time; add deterministic sampling tests | Only if sampling cost is material | **Proposed** |
| Constraint ordering and transforms — [OpenBone](https://github.com/UXCentury/open-bone) | Ordering, inheritance, local/world transforms, CCD IK | Reference or narrow adapter; prefer AniCraft constraints | MIT reported; inspect package and file dependency closure | Compare transform conventions and solver edge cases with AniCraft | Required before adopting a solver | **Proposed; host capability first** |
| Weight interpolation/vector deformation — [LibreToon](https://github.com/alvillafm/LibreToon) | Weight interpolation, vector deformation, rig/artwork bindings | Benchmark, then selective port or reimplementation | MIT reported; verify current license and file-level provenance | Golden fixtures, degenerate geometry, boundary and determinism tests | Compare against AniCraft native deformation | **Research** |
| FABRIK, RBF, soft selection, springs — [OSATURA](https://github.com/chiuta/osatura) | Isolated algorithm modules | Selective port or independent reimplementation | Core reported MPL-2.0; other components AGPL-3.0-or-later; isolate exact files and obligations | Algorithm-specific tests; confirm the selected file is in the intended license boundary | RBF vs AniCraft MLS; soft-selection latency | **Research; no blanket reuse** |
| Plastic / ARAP mesh methods — [OpenToonz](https://github.com/opentoonz/opentoonz) | Plastic deformation, ARAP, surface handles | Algorithmic reference first | BSD-3-Clause reported for project code; third-party code has separate terms | Validate numerical stability, mesh quality and boundary conditions | Required against simpler/native alternatives | **Research** |
| Multi-axis keyforms and deformers — [Aether Canvas](https://github.com/syuumaimikan/Aether-Canvas) | `crates/aether-rig/src/keyform.rs`: Cartesian N-D grid, non-uniform key spacing, multilinear and Catmull–Rom weights | **Reference only**; implement a narrow native typed-pose grid | GitHub repository license field is null and default tree has no LICENSE/NOTICE file; workspace Cargo.toml declares `MIT OR Apache-2.0`, but file-level provenance and legal notices remain unverified | Reuse the grid concepts only; Forma Deform requires typed scalar/vector/angle semantics, strict pose binding validation, and bounded dimensions | Benchmark after linear semantics and limits are fixed | **Blocked for code reuse; reference permitted** |

## 4. Adoption record (required for every port or new dependency)

Create one record per proposed adoption. Do not mark a row **Approved** until every required field is completed.

| Field | Required evidence |
|---|---|
| Registry ID and capability | Link to the row above and the exact product gap |
| Source identity | Repository URL, immutable commit/revision, exact file paths, and upstream version |
| Reuse decision | Reuse / adapter / port / reimplement / reference / build, with rationale |
| License and provenance | File-level license, copyright notices, attribution requirements, patent terms if applicable, and compatibility with this repository |
| Dependency closure | Direct and transitive dependencies, their licenses, build impact, maintenance state, and whether a smaller extraction is possible |
| Existing alternatives | AniCraft capability and internal module searched; explain why neither is sufficient |
| Semantic differences | Coordinate systems, units, boundary conditions, interpolation rules, error handling, determinism |
| Tests | Existing upstream tests reviewed; local unit, integration, regression, and malformed-input tests added |
| Performance | Benchmark setup, representative data sizes, baseline, target, and measured results when relevant |
| Decision and owner | Proposed / approved / implemented / rejected; reviewer and decision date |
| Follow-up | Upstream attribution, notices, update strategy, and any deferred risks |

### Status definitions

- **Proposed:** candidate identified; no code adoption authorized.
- **Research:** comparison or legal/dependency investigation underway.
- **Approved:** all adoption-record gates reviewed and explicitly accepted.
- **Implemented:** approved strategy is present in the repository with tests and required notices.
- **Rejected:** documented reason not to adopt.
- **Blocked:** a required host contract, license answer, test, or benchmark is missing.

## 5. Change-control checklist

For every new algorithm, dependency, or host adapter, the pull request should answer:

- [ ] Did we search this registry and the source tree for an existing implementation?
- [ ] Is the capability owned by AniCraft, Forma Deform, or an external library?
- [ ] Is the proposed reuse strategy explicit and appropriately narrow?
- [ ] Are exact upstream files, revisions, licenses, notices, and dependency licenses recorded?
- [ ] Are host APIs verified from source or supported documentation rather than assumed?
- [ ] Are deterministic, boundary, malformed-input, and regression tests included?
- [ ] Is benchmark evidence included where performance is a reason for adoption?
- [ ] Are duplicate implementations and unnecessary dependencies avoided?
- [ ] Are proposed host changes clearly separated from plugin-only changes?

## 6. Maintenance

Update this registry when:
- a new core module or host adapter is added;
- an external algorithm is investigated, approved, rejected, or ported;
- AniCraft adds or changes a capability that removes the need for plugin-owned code;
- a dependency, license, or benchmark decision changes.

The registry is a decision aid, not a mandate to create abstractions or dependencies. Prefer the smallest implementation that satisfies a verified requirement.
