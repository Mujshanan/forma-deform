# Forma Deform — Development and Architecture

**Project role:** standalone deformation authoring Studio with independent deformation engines and an optional future AniCraft adapter.  
**Status:** core capabilities exist; the initial standalone Studio shell is implemented, while feature editors, geometry loading, engine adapters, and project persistence remain pending.  
**Canonical scope:** deformation authoring, controller preview, point arrays and bindings, cages/curves/tessellation, validation, and deterministic deformation evaluation.  
**Non-goals:** a general animation editor, general drawing/painting application, or a runtime dependency on AniCraft.

## 1. Product boundary

Forma Deform answers: **how should geometry deform under explicitly authored deformation models and inputs?**

Forma Deform owns:
- Pure deformation algorithms and deformation-specific data structures.
- Smart Controller model, normalization, mapping, easing, validation, and preview.
- Pin/offset authoring, point-array and binding specifications, cages, Bézier-derived inputs, and tessellation controls.
- Corrective poses, additive deformation contributions, and deformation presets, subject to explicit composition semantics.
- Deterministic evaluation of its own data against explicit inputs.
- The standalone Studio shell, focused project lifecycle, viewport, selection, command dispatch, and undo/redo for Studio-authored changes.

AniCraft remains a future optional host/integration target and owns its own authoritative document, bones, transforms, Asset Warp, timeline, rendering, persistence, undo/redo, command registry, plugin lifecycle, evaluation scheduling, cache invalidation, and export. The Studio must not import or launch the AniCraft application to operate.

The Studio may define a versioned project/document format for its standalone workflow. A future AniCraft adapter must map supported data through a verified host document/extension boundary; it must not silently mutate host document structures or introduce a runtime dependency on AniCraft.

## 2. Accepted semantic and integration constraints

1. Bone IDs and transforms come from AniCraft's canonical rig model. Forma Deform does not create a competing bone identity system.
2. Coordinate-space conversions follow AniCraft's documented transform conventions.
3. Smart Controller binding evaluation and Form Deform pin evaluation are independent.
4. Smart Controller V1 is authoring/preview-first. Preview is on demand and does not imply continuous playback callbacks.
5. A controller definition and a deformation binding are distinct concepts. Bindings refer to stable controller IDs and stable host target identities.
6. No implicit multi-source composition. Combining Smart Controller outputs with Form Deform pin-deformation outputs requires an explicit future semantic decision.
7. No duplicate document model, timeline, renderer, undo stack, cache system, plugin framework, or asset library.
8. Do not claim durable plugin-owned binding persistence until an AniCraft host-side storage contract is implemented and verified.
9. Apply/bake operations are explicit, undoable commands. Preview is non-mutating.
10. No PR merge or host-repository change is implied by this plan.
11. CCC-01.5 is verified at the source/test-contract level: Smart Controller preview and Form Deform pin evaluation remain independent; the former cross-path integration test has been replaced with an isolated Smart Controller preview assertion, and the Smart Controller crate no longer depends on Form Deform for tests. AniCraft pin behavior remains host-owned and unchanged.

## 3. Feature allocation matrix

**Priority:** P0 = foundation; P1 = core workflow; P2 = advanced; P3 = later extension.  
**Disposition:** Own = implement here; Host = AniCraft owns; Consume = use an existing host capability; Adapter = bridge only; Research = benchmark before deciding.

| ID | Feature | Priority | Forma Deform disposition | AniCraft / other-plugin boundary |
|---|---|---:|---|---|
| C01 | Canonical bone identity and input model | P0 | Adapter | AniCraft owns stable bone IDs and rig data |
| C02 | Bone transforms and coordinate spaces | P0 | Adapter | AniCraft owns transforms and conversion rules |
| C03 | Transform constraints and inheritance | P1 | Consume / Adapter | Use AniCraft constraints; add no duplicate solver without a demonstrated gap |
| C04 | Angle/range mapping and neutral normalization | P0 | Own | Core controller mapping |
| C05 | Multi-axis parameter mapping | P2 | Own | Controller capability; not a new host parameter system |
| C06 | Pose-set and keyform interpolation | P1 | Own | Interpolate controller/deformation keyforms, not AniCraft timeline frames |
| C07 | Additive blend shapes and corrective poses | P2 | Own | Deformation-space contributions; composition rules must be explicit |
| C08 | Smart Bone action mapping | P1 | Adapter / Own mapping | Consume action samples from the 360 Studio Smart Bone/action model |
| C09 | Slider and joystick controllers | P1 | Own | Controller authoring and preview |
| C10 | Multiple target bindings | P1 | Own | Bind to stable host targets through contracts |
| C11 | Deterministic controller dependency evaluation | P1 | Own | AniCraft schedules; Forma evaluates its controller graph |
| C12 | Deformation target abstraction | P0 | Own | Host adapter resolves target identity and access |
| C13 | Pin and vector deformation | P0 | Own selectively | Reuse AniCraft Asset Warp where it already satisfies the need |
| C14 | Deformation validation and diagnostics | P0 | Own | Host reports capability/target errors; plugin reports model errors |
| C15 | Pure, repeatable evaluation | P0 | Own | AniCraft owns lifecycle, revision and cache scheduling |
| E01 | Lattice/cage deformation | P2 | Own | New algorithm only if not covered by native warp |
| E02 | Rotation/pivot deformation | P2 | Own | Deformation primitive |
| E03 | RBF pin-warp alternative | P2 | Research | Benchmark against AniCraft MLS before adopting |
| E04 | ARAP mesh deformation | P3 | Research | Benchmark quality, performance and numerical stability |
| E05 | Bone skinning and weight interpolation | P2 | Consume / Adapter | Prefer AniCraft's native weighted deformation; add alternative only if justified |
| E06 | Path constraints | P3 | Adapter | Prefer AniCraft constraints; do not duplicate general constraint infrastructure |
| E07 | Pose capture, mirroring and libraries | P2 | Own deformation presets | General pose/symbol library may belong to AniCraft/360 Studio |
| E08 | Character-turn mapping | P2 | Consume | 360 Studio owns view/turn semantics |
| E09 | Mesh tracing and automatic mesh generation | P3 | Adapter / Research | 360 Studio owns view-preparation workflow; AniCraft owns asset identity |
| E10 | Soft-selection and weight editing | P2 | Own | Deformation authoring tools |
| E11 | Reusable deformation presets | P2 | Own | Presets reference host targets by stable IDs |
| L01 | Secondary-motion physics | P3 | Own only as deformation behavior | Use host time/evaluation services; avoid a second animation runtime |
| L02 | Spring and inertia controls | P3 | Own parameters / Consume host data | AniCraft may already provide bone spring state |
| L03 | Animation clips and crossfades | — | Not core | AniCraft timeline/animation system |
| L04 | Procedural expression drivers | P3 | Own narrowly | Must not become a duplicate general-purpose graph engine |
| L05 | Runtime and external player integration | — | Not core | AniCraft runtime/export |
| L06 | Automatic rig generation | — | Not core | 360 Studio may propose a rig; AniCraft owns rig representation |

### 3.1 Feature groups that are deliberately outside this repository

- Character view sets, turn angles, front/side/three-quarter view organization, view correspondence, and turn-sheet generation belong to **AniCraft 360 Studio**.
- Smart Bone authoring, symbol rigging, Switch Groups, reusable symbol assets, and the character asset library belong to **AniCraft 360 Studio**.
- General bones, timeline, keyframes, document storage, rendering, and undo belong to **AniCraft**.

## 3.2 FDM v2 authored persistence and composition

FDM owns a versioned authored payload schema (`forma-deform-document` v1), deformation definition IDs/revisions, typed parameters, explicit reference geometry and coordinate descriptors, plus typed composition graphs. AniCraft still owns document embedding, save/load, dirty state and undo/redo. Runtime snapshots, evaluated results and cache entries are never authoritative persisted data.

Composition is independent by default. Sequential, parallel and weighted-blend operators require explicit graph nodes and inputs; the graph validator rejects cycles, missing references, orphan nodes, duplicate IDs, incompatible value signatures, target/reference/coordinate mismatches and invalid blend weights. Smart Controller output is not a graph source under CCC-01.5. See `docs/M1_FDM_PERSISTENCE_AND_COMPOSITION_SCHEMA.md`.

## 4. Standalone Studio UI

Forma Deform Studio is a standalone application. It uses the visual language and interaction conventions intended for AniCraft, but it contains only deformation authoring tools and does not depend on the AniCraft application.

### 4.1 Shared visual language boundary

- Use a compact dark workspace, consistent typography and spacing, neutral panel surfaces, warm accent for primary actions/active tools, and clear focus/selection/diagnostic states.
- Define theme tokens and reusable controls/panel primitives behind a host-independent UI boundary.
- The shared UI design kit must be independently consumable by both applications; do not import AniCraft application modules into Forma Deform.
- The AniCraft 360 Studio repository is currently a proposal containing documentation only. It is not an implemented component library.
- Do not claim parity with actual AniCraft components until the real AniCraft application source and design system are identified and inspected.

### 4.2 Standalone workspace layout

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Forma Deform Studio · File · Edit · View · Tools · Help       Commands      │
├────────────────┬──────────────────────────────────────┬─────────────────────┤
│ Deform tools   │ Viewport                             │ Inspector           │
│ Select         │ Artwork / selected target            │ Target & capability │
│ Pin            │ Pins / offsets / handles             │ Pin / driver        │
│ Point Array    │ Array points and binding overlays     │ Array & binding     │
│ Bind           │ Cage vertices / edges / faces         │ Cage / topology     │
│ Cage           │ Bézier paths and handles              │ Curve               │
│ Bézier         │ Tessellation preview overlay          │ Tessellation        │
│ Tessellate     │ Validation / stale-state overlays     │ Diagnostics         │
├────────────────┴──────────────────────────────────────┴─────────────────────┤
│ Contextual status · selection details · preview/validation messages         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.3 Interaction model

- Selecting a target populates the Inspector; do not add an unrelated general-purpose scene editor.
- Tools appear only when the selected target supports them.
- Pin/offset, point-array/binding, and cage/curve/tessellation features use their own focused inspectors.
- Preview is explicit, deterministic, and non-mutating.
- Persistent changes go through Studio commands and participate in Studio undo/redo.
- Binding freshness, topology validity, stable IDs, and authored-versus-derived geometry remain core-owned contracts.
- No general animation timeline, keyframe editor, audio/video tools, or unrelated asset library.
- A future AniCraft adapter is optional and isolated. It must not alter standalone launchability.

### 4.4 Feature groups

| Group | UI responsibility |
|---|---|
| Pin & Offset Drivers | Pin placement, neutral/rest position, offsets, driver source/range/mapping, preview values |
| Point Arrays & Binding | Array parameters, source/cage selection, bind/unbind/rebind, freshness and outside-cage diagnostics |
| Cages & Bézier | Topology-aware cage editing, stable control-point identities, authored path/handle editing |
| Vector Tessellation | Non-destructive sampling preview, tolerance/budget controls, capped/validation diagnostics |

### 4.5 Implementation sequence

| Step | Deliverable |
|---|---|
| UI-01 | Standalone workspace/product contract |
| UI-02 | Source and UI architecture audit |
| UI-03.1 | Initial standalone shell, focused tool rail, inspector, status, viewport pan/zoom | Implemented; verification pending |
| UI-03.2 | Viewport and application-boundary hardening | Next |
| UI-04 | Reusable theme tokens and foundational primitives | Planned |
| UI-05 | Pin, point-array/binding, cage/Bézier/tessellation feature panels | Planned; core contracts remain authoritative |
| UI-06 | Project persistence, undo/redo, and UX regression coverage | Planned |
| UI-07 | Optional AniCraft adapter after host APIs are verified | Future, not a runtime dependency |

See [UI-01 workspace contract](docs/UI_01_DEFORMATION_AUTHORING_WORKSPACE.md) and [UI-02 source/UI architecture audit](docs/UI_02_SOURCE_AND_UI_ARCHITECTURE_AUDIT.md). The first shell implementation is recorded in commit [ec44889](https://github.com/Mujshanan/forma-deform/commit/ec44889fdb5ced812099618f9b67663109a03257).

## 5. External feature and code-reuse audit

The repositories below were inspected at source level for selected modules. This is a candidate audit, not a full dependency, test, benchmark, provenance, or legal review.

| Reference | Useful area | Reuse recommendation | License / caution |
|---|---|---|---|
| [Fanimate](https://github.com/toonittmedia/fanimate) | Smart Bone actions, stable bone IDs, pose snapshots/mirroring, evaluator scratch buffers | Reuse concepts; selectively port small isolated algorithms only after dependency review | Repository metadata reported Apache-2.0; preserve notices and verify file-level provenance |
| [OpenBone](https://github.com/UXCentury/open-bone) | Constraint ordering, transform inheritance, local/world transforms, CCD IK | Prefer algorithmic reference or a narrow port; AniCraft already owns general rig infrastructure | Repository metadata reported MIT; inspect package and file dependencies |
| [LibreToon](https://github.com/alvillafm/LibreToon) | Weight interpolation, vector deformation, rig/artwork bindings, character-turn data | Strong candidate for isolated Rust algorithm comparison and possible reuse | Repository metadata reported MIT; retain notices and verify current license |
| [OSATURA](https://github.com/chiuta/osatura) | FABRIK, RBF pin warp, soft selection, spring behavior | Benchmark RBF against AniCraft MLS; port only self-contained core components | MPL-2.0 core; other components AGPL-3.0-or-later. Keep file boundaries and obligations clear |
| [OpenToonz](https://github.com/opentoonz/opentoonz) | Plastic deformation, ARAP mesh methods, surface handles | Algorithmic reference first; C++ transplant is not the default | BSD-3-Clause project code; third-party components have separate terms |
| [Aether Canvas](https://github.com/syuumaimikan/Aether-Canvas) | Parameter/keyform grids, multi-axis interpolation, additive blend shapes, warp and rotation deformers, driver evaluation | High-value design reference; isolate generic algorithms only after license and dependency verification | License was not confirmed in the source audit: do not copy code until verified |

### 5.1 Reuse decision matrix

| Component/capability | Candidate source | Reuse mode | Decision gate |
|---|---|---|---|
| Range normalization and easing | Aether Canvas; Fanimate | Independent implementation or narrow port | Define semantics and tests first |
| Multi-axis keyform interpolation | Aether Canvas | Algorithmic reference; possible isolated extraction | Verify license, edge cases, and deterministic interpolation |
| Smart Bone action sampling | Fanimate | Adapter/design reference | Must map to canonical controller IDs and AniCraft time |
| Constraint ordering / transforms | OpenBone | Reference or narrow adapter | Confirm AniCraft has a genuine missing capability |
| Vector/weight deformation | LibreToon | Benchmark and selective port | Compare output quality and numerical behavior |
| RBF pin warp | OSATURA | Benchmark alternative | Compare against current MLS; license boundary review |
| ARAP mesh deformation | OpenToonz | Algorithmic reference | Performance/quality benchmark and source-specific license review |
| Soft selection | OSATURA | Selective algorithm port or reimplementation | Verify core file is MPL-covered and isolate dependencies |
| Physics/springs | OSATURA; Aether Canvas | Later reference | Must integrate with AniCraft time/evaluation rather than add a second runtime |

**Reuse policy:** do not copy a subsystem because it is written in Rust or has a permissive repository-level license. Verify the exact file license, copyright headers, dependency licenses, tests, numerical behavior, and whether the algorithm duplicates AniCraft. Prefer small, isolated, well-tested algorithms over whole frameworks.

## 6. Code and ownership architecture

```text
forma-deform
├── model
│   ├── controller definitions
│   ├── binding specifications
│   ├── deformation targets
│   └── versioned plugin-owned schemas
├── controller
│   ├── input normalization
│   ├── range and multi-axis mapping
│   ├── easing and keyform interpolation
│   └── dependency validation/evaluation
├── deformation
│   ├── pin/vector adapters
│   ├── lattice and rotation
│   ├── blend shapes/correctives
│   └── optional mesh algorithms
├── preview
│   ├── deterministic evaluation
│   └── diagnostic output
└── host
    └── AniCraft command/document/evaluator adapters
```

The module sketch is conceptual. Do not create empty crates or force the current repository into this exact structure before a concrete implementation need exists.

## 7. Milestones and gates

| Milestone | Scope | Exit criteria |
|---|---|---|
| M0 — Contract audit | Confirm bone, warp, command, storage, undo, and evaluator APIs | Documented supported/unsupported capabilities with source references |
| FD-M1 — Pure controller core | Stable IDs, normalization, validation, deterministic preview, and the initial built-in one-dimensional mapping subset (see section 10) | Core implementation and CI-tested; joystick and multi-input mapping remain deferred |
| FDM-v2 M1 — Host geometry/evaluation contracts | Stable host target identity, immutable source snapshots, evaluated geometry input/output, time/dependency stamps, persistence, cache invalidation, and transactional apply/bake boundaries | FDM-side contracts and acceptance matrix reviewed; AniCraft conformance explicitly tracked separately |
| FD-M2 — Plugin preview | Read verified AniCraft inputs and produce preview results | On-demand preview is repeatable and does not mutate document |
| FD-M3 — Native authoring | Typed command authoring and host-extension requirements | Uses verified AniCraft command/settings capabilities; host-dependent features remain explicit |
| FD-M4 — Persistent bindings | Versioned binding model stored through host-owned document extension | Core payload plus host save/load, migration, undo and invalidation tests pass |
| FD-M5 — Deformation integration | Target adapters and explicit apply/bake commands | Stable target identity and atomic undo; GUI and host transaction tests pass |
| FD-M6 — Host evaluation integration | Live evaluator only if AniCraft exposes a supported hook | Scrub/playback determinism, invalidation and performance tests |
| FD-M7 — Advanced algorithms | Multi-axis keyforms, lattice, RBF/ARAP research, soft selection | Benchmarks justify each addition |

**Milestone naming:** `FD-M1` is the Smart Controller one-dimensional mapper described in section 10. `FDM-v2 M1` is the separate host-geometry/evaluation contract phase. They are not the same milestone. FD-M2 through FD-M5 are the Smart Controller preview/authoring/persistence/apply track; they do not, by themselves, prove FDM-v2 M1 host conformance.

Do not skip M0 to claim capabilities that the host does not expose. M6 is not a prerequisite for a useful authoring/preview tool.

## 8. Testing requirements

- Pure unit tests for normalization, mapping, easing, keyform interpolation, dependency validation, and deformation math.
- Determinism tests: identical model and explicit inputs produce identical outputs.
- Immutability tests: preview does not mutate authored geometry, keyframes, or binding definitions.
- Stable identity tests for bone, controller, and target references.
- Adapter tests against verified AniCraft command responses.
- Save/load and migration tests when host persistence exists.
- Undo/redo tests for every persistent edit and apply/bake command.
- Numerical edge cases: zero-length ranges, reversed ranges, non-finite inputs, degenerate geometry, cyclic dependencies, missing targets, and boundary values.
- Benchmarks before adopting RBF, ARAP, or new skinning algorithms.

## 9. Completion and change control

A feature is not complete because a UI control exists. It must have a defined model, deterministic semantics, validation, tests, and a verified host boundary where applicable.

This document is an architecture and development plan. It does not authorize changes to AniCraft, external repositories, or PR merges.

## 10. FD-M1 — Canonical one-dimensional Smart Controller mapper

**Implementation area:** `crates/smart-controller-core/src/mapping/canonical.rs`  
**Public API:** `smart_controller_core::mapping::CanonicalControllerMapper`  
**Pose lookup:** `smart_controller_core::mapping::PoseLibrary`

FD-M1 supplies a first-party implementation of the existing `ControllerMapper` contract. The evaluator continues to own input snapshot preflight, dependency order, unavailable-output propagation, and contribution collection. The mapper owns only the pure transformation from one resolved driver input to typed outputs keyed by canonical BindingIds.

### 10.1 Supported mappings

| Mapping | Input semantics | Output semantics |
|---|---|---|
| `Range1D` | Scalar or Angle input normalized through explicit minimum / neutral / maximum | Clamped signed domain [-1, 1], linear interpolation around neutral |
| `ThreeAnchorRange` | Same explicit three-point range | Symmetric Linear or Smoothstep easing on either side of neutral |
| `DirectedRange1D` | Scalar or Angle input normalized through directed start / end; reversed endpoints are supported | Unit domain [0, 1]; anchor targets use the equivalent signed domain |
| `Discrete` | Scalar input clamped to [-1, 1] | Nearest equally spaced authored value; midpoint ties choose the higher index; exactly one output binding is required |
| `PoseSet1D` | Scalar input clamped to [-1, 1] | Ordered pose IDs resolve through the runtime PoseLibrary; adjacent poses interpolate evenly by BindingId |

Angle input and Angle property output use radians and preserve wrapped/unwrapped semantics. Discrete and PoseSet1D require Scalar input.

### 10.2 Target mapping

- **SymbolFrame:** interpolate negative, neutral and positive frame anchors, then round to the nearest integer frame.
- **FrameRange:** map the unit input through the configured endpoint policy; the default remains Exclusive.
- **GeometryDeform:** interpolate each vector component between the three anchors. RelativeDelta emits the difference from the neutral anchor; the existing contribution/composition stage applies the authored base.
- **Property:** multiply the signed mapped value by the configured scale. Scalar input produces Scalar output; Angle input produces Angle output.
- **PoseSet1D:** every referenced pose must be registered, all poses must have compatible typed values and angle semantics, and each pose must contain exactly the controller's declared binding IDs.

### 10.3 Explicit limits and ownership

- The initial mapper requires exactly one declared driver input. Multi-input controllers are rejected rather than choosing an implicit input.
- `PoseLibrary` is runtime-only. It is not a persistence mechanism and does not extend CanonicalDocument schema v1.
- `JoystickFivePose` is explicitly unsupported by this one-dimensional mapper. It must be implemented by a dedicated two-dimensional pose-field mapper, not approximated as a scalar range.
- Mapper outputs remain uncomposed contributions. The mapper does not write host targets, mutate AniCraft documents, schedule playback evaluation, or combine with Form Deform pin evaluation.
- Mapping-domain failures are converted by the existing evaluator into `SC_MAPPING_DOMAIN` diagnostics and unavailable controller outputs.

### 10.4 Verification scope

The mapper integration tests cover three-anchor endpoints and clamping, smoothstep symmetry, reversed directed ranges, frame endpoint policy, symbol-frame interpolation/rounding, discrete quantization and typed outputs, pose interpolation by binding identity, relative geometry deltas, Angle semantics, missing pose references, exact pose binding sets, unsupported joystick mapping, and ambiguous multi-input rejection.

This milestone completes the initial pure one-dimensional mapping layer only. It does not establish durable pose storage, native authoring UI, continuous playback evaluation, general multi-axis interpolation, or Smart Controller/Form Deform composition.

## 11. FDM v2 accepted architecture baseline

**Architecture status:** ADM-001 through ADM-007 are accepted. CCC-01.5 remains an accepted integration constraint. These decisions supersede the older readiness text that described the ADM topics as unresolved; they do not retroactively change implementation status or prove that host integration exists.

| Decision | Accepted architecture |
|---|---|
| ADM-001 — Geometry ownership | Hybrid: AniCraft owns canonical artwork, source topology, identities, transforms, hierarchy, and native Asset Warp; FDM owns additional deformation-specific geometry. Exchange immutable, revisioned snapshots through an adapter. |
| ADM-002 — Lattice topology | FDM supports multiple explicit topology types, initially regular structured lattices and irregular cages; generated and directly authored topology; explicit mappings; stable control-point IDs; revisioned structural changes. |
| ADM-003 — Nested deformation | Parent deformation propagates to descendants; child deformation does not affect ancestors or siblings; local-rest-space authoring; explicit mappings and acyclic dependencies. |
| ADM-004 — Multiple-source composition | Sources evaluate independently by default; sequential, parallel, and compatible weighted composition require an explicit deterministic composition graph. |
| ADM-005 — Animation ownership | AniCraft owns timeline, tracks, keyframes, time, sampling, scheduling, and persistence; FDM owns deformation-specific animatable properties and specialized interpolation. |
| ADM-006 — Baking representation | Retained, non-destructive bake into AniCraft-native geometry; provenance, source restoration, frame-range support where available, and transactional undo/redo. |
| ADM-007 — Runtime representation | Persistent data, runtime state, and results are separate; AniCraft owns scheduling/snapshots/cache lifecycle/application; FDM owns prepared evaluators and deformation runtime models. |

### 11.1 CCC-01.5 remains a specific restriction

Smart Controller binding evaluation and Form Deform pin-deformation evaluation remain independent. Smart Controller V1 remains preview-only, and the existing pin-deformation formula/behavior is preserved. ADM-004 does not authorize combining these outputs. Such composition requires a separate explicit decision.

### 11.2 Authoritative architecture and M0 audit

- [FDM v2 Architecture and Integration Specification](docs/FDM_V2_ARCHITECTURE.md) — consolidated accepted decisions, responsibilities, invariants, lifecycle, and open implementation questions.
- [FDM v2 AniCraft Host Capability and Integration Audit](docs/FDM_V2_HOST_CAPABILITY_AUDIT.md) — evidence-linked M0 audit. Status: audit and ownership matrix complete; CCC-01.5 discrepancy resolved and the Smart Controller changes passed the full GitHub Actions CI workflow, including workspace tests and all three release WASM plugin builds.

The architecture specification is the authoritative consolidation of accepted semantics. The host audit records actual AniCraft capabilities and gaps; it distinguishes inspected source evidence from assumptions and previously documented but not reverified behavior.

### 11.3 FDM v2 M1 contract package and status

The FDM v2 M1 contract package is split into three focused documents:

1. [Stable target identity and revisioned geometry snapshots](docs/M1_TARGET_GEOMETRY_CONTRACT.md) — implemented FDM-side opaque identity, Path2D v1 snapshot, canonical fingerprint and source freshness primitives.
2. [Evaluation and host lifecycle contracts](docs/M1_EVALUATION_LIFECYCLE_CONTRACT.md) — evaluated geometry input/output, explicit time sampling, dependency stamps, cache invalidation, persistence ownership, and transactional apply/bake boundaries.
3. [Host conformance and acceptance test matrix](docs/M1_HOST_CONFORMANCE_TEST_MATRIX.md) — FDM automated gates and AniCraft host conformance tests with explicit evidence rules.

The contract documents and the FDM-side evaluation request/result DTOs now define and validate the source snapshot, model, time, external dependency and output-result boundary. The remaining FDM-side work includes a persistent deformation-model schema/migration and a concrete composition-graph model. The host document identity/revision store, timeline sampler, persistent extension, cache lifecycle, and native transaction implementation do not yet exist in AniCraft. The AniCraft conformance tests remain blocked pending a separately reviewed and authorized host implementation.

### 11.4 Next implementation sequence

1. Review and accept the complete FDM v2 M1 contract and DTO package.
2. Complete the FDM-authored deformation persistence schema, migration rules and explicit composition-graph validation.
3. Separately authorize and implement AniCraft document/target identity, revision, snapshot and coordinate conformance.
4. Implement host persistence and explicit transaction/bake commands only against verified host APIs.
5. Validate the FD-M2 through FD-M5 Smart Controller plugin chain as a related but separate workflow; do not count it as proof of FDM v2 M1 host conformance.
6. Continue with host evaluation scheduling only if AniCraft exposes a supported hook.

**Governance:** Accepted architecture decisions are not blanket authorization to modify AniCraft, implement every proposed topology/algorithm, or merge pull requests. Preserve this boundary in issues, PRs, and subsequent milestones.
