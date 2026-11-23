# Forma Deform — Development and Architecture

**Project role:** AniCraft deformation and rig-control tool/plugin.  
**Status:** prototype and architecture definition; native tool integration is staged and must not be represented as complete. The repository status below distinguishes default-branch content from open or branch-local work.  
**Canonical scope:** deformation, controller authoring/preview, target bindings, and deformation evaluation.  
**Non-goal:** a second animation application or a duplicate of AniCraft host infrastructure.

## 1. Product boundary

Forma Deform answers: **how should artwork deform when bones, pins, parameters, and authored controllers change?**

It owns:
- Pure deformation algorithms and deformation-specific data structures.
- Smart Controller model, normalization, mapping, easing, validation, and preview.
- Binding specifications between controller inputs and deformation targets.
- Corrective poses, additive deformation contributions, and deformation presets.
- Deterministic evaluation of its own data against explicit host inputs.

AniCraft owns the authoritative document, bones, transforms, Asset Warp, timeline, rendering, persistence, undo/redo, command registry, plugin lifecycle, evaluation scheduling, cache invalidation, and export.

The plugin may define versioned schemas for its own authored data, but persistence must be implemented through a supported AniCraft document/extension boundary. Do not invent an independent file store or silently mutate host document structures.

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

## 4. UI proposal

The UI is a native AniCraft tool context using existing stage, properties, timeline, command, and dockable-panel infrastructure.

### 4.1 Layout

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ AniCraft menus / contextual tool options / command palette               │
├──────────────┬───────────────────────────────────────┬───────────────────┤
│ Tool strip   │ Stage                                 │ Properties        │
│ Select       │ Selected artwork                      │ Target            │
│ Pin          │ Deformation mesh                       │ Source input      │
│ Handle       │ Neutral/rest markers                   │ Range / neutral   │
│ Soft select  │ Preview displacement                   │ Mapping / easing  │
│ Bind         │                                        │ Target gains      │
│ Preview      │                                        │ Diagnostics       │
├──────────────┴───────────────────────────────────────┴───────────────────┤
│ Existing AniCraft timeline / layers / playhead                           │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Interaction model

- Selecting a target populates the Properties panel; no duplicate scene/layer tree.
- Pin and handle tools appear only for compatible targets.
- Binding is a guided action: choose source, choose target, capture/confirm neutral, configure range/mapping, validate, preview, then explicitly save/apply.
- Preview displays source input, normalized input, mapping result, and target displacement.
- Preview must not write keyframes, change authored pin positions, or modify the document.
- Apply/bake is an explicit host command with one coherent undo operation.
- Timeline scrubbing uses AniCraft's playhead. V1 must not simulate playback by repeatedly issuing commands and then describe that as a native evaluator.
- Diagnostics should identify missing source IDs, stale targets, invalid ranges, unsupported target types, cycles, and host capability gaps.

### 4.3 UI evolution

| Stage | UI capability |
|---|---|
| Prototype | Command/MCP-driven inspection and on-demand preview |
| Native authoring tool | Tool strip, target inspector, binding editor, diagnostics |
| Persistent model | Binding list, stable IDs, versioned authored settings |
| Host evaluator integration | Live evaluated overlay and playback only after AniCraft exposes a verified evaluator hook |

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
| [Flare](https://github.com/Flare-Animate/Flare) | Rust animation-suite architecture; vector drawing/editing, timeline/keyframes, motion and shape tweens, easing, symbols/instances, Moho project import, CLI/MCP workflows | High-value architecture and algorithm reference. Evaluate isolated tween/easing, vector-geometry, symbol/playback, and import-model components only where Forma Deform has a demonstrated gap; do not import the editor, timeline, document model, renderer, or plugin framework | BSD-3-Clause is stated by the current repository README. Early alpha and actively evolving; verify the exact source file, dependency tree, notices, provenance, and current license before any code port. `original/` is archived C++ reference material, not the default integration target |

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
| Scalar animation curves and cubic Bézier handles | Flare `flare-tween` | Candidate isolated algorithm reference/port for controller response curves | Only if Forma Deform needs curve behavior beyond its current mapper; keep AniCraft timeline ownership and compare semantics/tests first |
| Polyline correspondence and arc-length resampling | Flare `flare-tween::interpolate_sequence` | Strong candidate for future stroke/contour correspondence; not a cage deformer | Validate orientation, degenerate segments, point limits, overshoot and topology policy against Forma's requirements |
| Vector contour morphing | Flare `flare-eval::morph` | High-value algorithmic reference; selective extraction may be justified for vector-shape deformation | Handles contour matching/splitting/alignment, not arbitrary point-to-cage deformation; audit dependencies and numerical behavior before porting |
| Vector path data model | Flare `flare-document::Shape` / `PathSegment` | Data-model reference only | Do not import Flare's document model; use AniCraft-owned path/shape identity and lifecycle |
| 3D triangle mesh and cage representation | Flare experimental `viewspace::BaseMesh` | Data-structure reference only; cage algorithm not reusable yet | `BaseMesh::apply_cage` is an explicit `NotImplemented` stub; do not mistake control-point storage for a working cage deformer |
| 3D weighted skinning | Flare `flare-mannequin::skin` | Algorithmic reference only | Specialized private 3D mannequin implementation with at-most-two joint influences; not a generic 2D deformation API |
| 2D rig forward/inverse kinematics | Flare `flare-rig` | Do not adopt; host-boundary reference only | AniCraft owns canonical bones, transforms and rig evaluation; avoid duplicate rig/IK systems |
| Flare animation/tweening and vector model | Flare | Architecture reference; selective algorithm extraction only after source audit | Identify a concrete Forma Deform gap; inspect the current Rust implementation, API/dependency footprint, tests, numerical behavior, and file-level license. Do not duplicate AniCraft timeline/document/rendering infrastructure or import the whole editor |

**Reuse policy:** do not copy a subsystem because it is written in Rust or has a permissive repository-level license. Verify the exact file license, copyright headers, dependency licenses, tests, numerical behavior, and whether the algorithm duplicates AniCraft. Prefer small, isolated, well-tested algorithms over whole frameworks.


### 5.2 Flare-specific source audit

**Repository:** [Flare-Animate/Flare](https://github.com/Flare-Animate/Flare)  
**Audited revision:** `5105e8c85d1e03546e6ce55884fce68055a19c8a` (the `main` tree observed during this audit).  
**License baseline:** the root `LICENSE` and workspace package metadata state BSD-3-Clause. This is not a substitute for checking bundled third-party material, generated files, or file-specific provenance before copying.

#### Implemented Rust components worth studying

| Source | Verified implementation | Forma Deform assessment |
|---|---|---|
| `crates/foundation/flare-tween` | Pure scalar curves with keyframes, cubic Bézier time/value handles, automatic/aligned/free handle policies, constant/linear/Bezier and easing families, constant/linear/cyclic/cyclic-offset extrapolation, bounded validation and tests. Its crate manifest depends only on `thiserror`; the library describes the implementation as clean-room. | **Best isolated candidate** for controller response curves if our required semantics exceed the current mapper. It is not a timeline and must not take ownership of AniCraft frames or playback. |
| `flare-tween::interpolate_sequence` | Interpolates 2D stroke point sequences. Equal-count strokes pair by order; unequal counts are resampled by arc length to a common count. Supports reversing the target sequence, easing, finite-input checks and a 4096-point bound. | **Strong future candidate** for stroke/contour correspondence or vector-shape transitions. It is not a cage, mesh, or arbitrary source-to-target binding algorithm. |
| `crates/foundation/flare-eval/src/morph.rs` | Matches vector contours, converts line segments to cubic pieces, splits the longest cubic pieces to equalize counts, aligns closed-contour winding/start positions, collapses unmatched contours, then interpolates geometry and paint. It applies explicit contour/segment/search limits. | **High-value algorithmic reference** for a future vector-contour deformation/morph feature. It has meaningful topology assumptions and is not a general-purpose FFD solver. Review its full dependency and test surface before any extraction. |
| `crates/foundation/flare-document/src/shape.rs` | Immutable/shared vector shape path represented by `MoveTo`, `LineTo`, `CubicTo`, and `Close`, with fill, gradient and stroke metadata. | Useful path representation reference only. Forma Deform should bind to AniCraft's authoritative shape/path identities rather than adopt a second document model. |
| `crates/foundation/flare-mannequin/src/skin.rs` | Builds a specialized 3D tube/ring surface with triangle connectivity, vertex normals and up to two joint influences per vertex; its skinning routine moves vertices from rest to posed joint frames. | Interesting skinning mathematics, but the module is private and tied to Flare's 3D mannequin skeleton and rotation model. Not a drop-in 2D mesh skinning crate. |
| `crates/foundation/flare-rig` | 2D hierarchical bone model, forward kinematics, pose interpolation, two-bone IK and bounded CCD IK. | Do not reuse as a competing rig system. AniCraft owns canonical bones and rig evaluation; consult only for isolated mathematical edge cases if a real host gap is demonstrated. |

#### Important non-implementation: the experimental cage mesh

Flare's `crates/foundation/flare-core/src/experimental/viewspace/mesh.rs` defines a 3D `BaseMesh` with vertices, triangular faces, UVs, semantic regions, depth values, control points, optional bone hierarchy and feature anchors. It validates the mesh and computes vertex normals/bounds.

However, `BaseMesh::apply_cage` currently returns `ViewSpaceError::NotImplemented`. Its source comment describes future mean-value or Green-coordinate weights (and mentions linear blend skinning as a possible v1), but those are TODO design notes, not implemented algorithms. The experimental module's own status table lists `mesh-cage` as a stub. Do not cite this as a working cage deformer or reuse its control-point structure as proof of a completed solver.

The Moho vector importer is also not a suitable curve-deformation source: its own module documentation says it reads indexed points/curves but does not read Bézier handles, so converted paths become polylines.

#### Reuse decisions for Forma Deform

1. **Inspect `flare-tween` first** if we need richer scalar controller curves. Its dependency footprint is small and its algorithms are separate from the editor. Before porting, compare its interpolation/easing and extrapolation semantics with our existing Smart Controller mapper and tests.
2. **Keep `interpolate_sequence` as a candidate** for future stroke correspondence. Its arc-length resampling is useful where point counts differ, but it assumes ordered contours and does not solve topology correspondence generally.
3. **Use `flare-eval::morph` as a reference** if we add vector-contour morphing. Do not confuse shape tweening with cage deformation, and do not import the broad `flare-eval` crate merely to obtain this algorithm.
4. **Do not reuse Flare's experimental cage implementation**; the actual cage operation is unimplemented. Forma Deform still needs its own defined cage topology, point-location/binding contract, deformation weights and evaluation solver (or another verified source).
5. **Do not import Flare's rig, timeline, document, renderer, UI, CLI, MCP server or application crates.** Those responsibilities either belong to AniCraft or are outside Forma Deform's scope.
6. **Treat mannequin skinning as research only.** Its private 3D implementation is specialized and would require a deliberate 2D adaptation, not a mechanical port.

#### Licensing and extraction gate

The audited Flare workspace declares BSD-3-Clause and the standalone `flare-tween` crate has a narrow dependency footprint. This makes selected source worth further consideration, but does not grant blanket approval to copy the repository. Before any port or vendoring:
- pin the exact Flare commit and exact source files;
- inspect file headers, authorship/provenance and any generated or imported material;
- audit the transitive dependency licenses for the chosen files/crate;
- preserve BSD-3-Clause notices and disclaimers where required;
- run source-specific tests and add Forma Deform regression/property tests;
- compare edge cases, numerical stability, performance and semantics against our own requirements.

**Audit disposition:** Flare is a useful algorithmic reference, not a deformation-engine dependency. The strongest candidates are the standalone `flare-tween` curve/sequence algorithms and the isolated contour-morph algorithm. No code is approved for copying, vendoring, or runtime dependency use by this assessment.
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
| M1 — Pure controller core | Stable IDs, normalization, validation, deterministic preview, and the initial built-in one-dimensional mapping subset (see section 10) | Core implementation and CI-tested; joystick and multi-input mapping remain deferred |
| M2 — Plugin preview | Read verified AniCraft inputs and produce preview results | On-demand preview is repeatable and does not mutate document |
| M3 — Native authoring UI | Tool context, binding editor, target inspector, diagnostics | Uses AniCraft panels/commands; keyboard and undo behavior verified |
| M4 — Persistent bindings | Versioned binding model stored through host-owned document extension | Save/load, migration, undo and invalidation tests pass |
| M5 — Deformation integration | Target adapters and explicit apply/bake commands | Stable target identity and atomic undo |
| M6 — Host evaluation integration | Live evaluator only if AniCraft exposes a supported hook | Scrub/playback determinism, invalidation and performance tests |
| M7 — Advanced algorithms | Multi-axis keyforms, lattice, RBF/ARAP research, soft selection | Benchmarks justify each addition |

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


## 11. Repository status and current evidence (23 November 2026)

This section records the state of the active repository work as distinct from the default-branch source baseline. A PR being open, merged into another feature branch, or green on a parent commit does not mean its work is present on `main`.

### 11.1 Default branch and legacy demo retirement

The latest recorded default-branch commit at this snapshot is `e182e447a9e6b320507982b98069dc90e5b6e323` (`docs: add Flare source-level reuse audit`). The default-branch README previously described the experimental Bun/TypeScript rectangle/timeline demo.

The demo-retirement work is real but branch-scoped:

- PR #52 retired the obsolete TypeScript rectangle/timeline demo, its tests and HTML/CSS shell, package/TypeScript configuration, and Bun CI steps; it also repaired Rust compilation errors and reported workspace tests plus all three release WASM builds passing on its exact head.
- PR #52 merged into the LEG-01.4 branch, not directly into default `main`.
- PR #54 carried the retirement into the FDM-04 branch chain and added the adaptive Path2D sampling benchmark/quality gate. Its reported CI passed on exact head `7aba19b31dbd030d7cd32b83832aed5ee32002cc`.
- Therefore, do not claim that the default branch has completed the retirement until the relevant branch chain is integrated into `main`. The three historical Rust/WASM adapters remain separate from the demo retirement and are not deleted by it.

### 11.2 FDM core and output-contract work

- PR #60 (FDM-05.4) is an open integration-readiness/output-contract hardening change. Its reported exact-head CI covers workspace tests, the adaptive-sampling benchmark smoke test and all three release WASM builds. It is not merged.
- PR #47 remains the FDM-03 integration/regression branch. Its description records V1 persistent-binding tests, V2 operator-engine integration and V3 cage-contract reconciliation; branch lineage and required follow-up verification must be checked before treating FDM-03 as complete.
- PR #54 is the canonical FDM-04.4 benchmark/quality-gate candidate in its branch chain; superseded PR #53 must not be substituted.
- Do not infer that any open FDM branch is part of the default branch or that a green parent commit validates a later PR head.

### 11.3 Adapter retirement and formatting

- PR #49 records the LEG-01.4 adapter disposition and retirement gates. It is documentation-only and remains open.
- The old Form Deform command adapter is a retirement candidate only after native FDM parity. Bone Query remains diagnostic tooling for now; the vendored ABI SDK is retired last. Smart Controller core compatibility/migration modules are not to be removed as part of command-adapter retirement.
- PR #68 records FDM-FMT-01.1 only: a 69-file rustfmt-diff baseline, crate/ownership grouping, and six overlapping files deferred pending stable lineage. It does not format Rust files.
- **FDM-FMT-01.2 status: BLOCKED, 0/36 files formatted.** The scoped target is the 36-file `smart-controller-core` group. The exact manifest was recovered from this plan. The local execution container has no `cargo`/`rustfmt` and cannot reach GitHub directly. A temporary isolated GitHub Actions runner was attempted on `fix/fdm-fmt-01-2-ci-runner`, but run [#45206450353](https://github.com/Mujshanan/forma-deform/actions/runs/45206450353) completed with zero jobs; no formatting artifact was produced. The temporary runner branch was reset to its original maintenance-baseline commit; the maintenance branch and PR #68 were not modified. No source file was changed, and exact-file diff review, scoped formatter check, crate tests, and the separate maintenance commit remain outstanding. The existing PR #68 CI run [#45171640134](https://github.com/Mujshanan/forma-deform/actions/runs/45171640134) failed at the workspace-wide `cargo fmt --all -- --check` step, so later checks were skipped.
- **FDM-FMT-01.3 status: NOT STARTED.** Its 17 non-overlapping `form-deform-core` files remain queued until FDM-FMT-01.2 has been formatted, verified, tested, and committed separately.
- Preserve the maintenance branch `chore/fdm-fmt-01-workspace-formatting-remediation` and the existing PR #68 lineage. Do not substitute a repository-wide formatting run, include the six deferred PR #61 overlap files, or mix formatting with behavior changes.

### 11.4 Native AniCraft integration and S0 evidence gate

The current FDM-06.x work is a sequence of open documentation, source-audit, specification and planning PRs. The AniCraft source baseline is pinned to `2df7775907d1458ce653e7e6314ec8a5d7d565f5`; re-pin and re-audit if the host changes.

The sequence has established or proposed:

- Host-owned document/target identity, versioned extension persistence, immutable evaluation snapshots, mutation ownership, invalidation and stale-result acceptance contracts.
- A source-pinned host conformance plan and target-identity/revision lifecycle audit.
- A readiness finding that runtime implementation is **not ready** while public mutable `Session.doc`/`Document` APIs, settlement side effects, identity-ledger/history gaps and atomic publication evidence remain unresolved.
- A mutation-boundary/state-ownership specification and a file-by-file host transaction/persistent-state implementation plan.
- A command registry and workspace-consumer audit, including 395 unique command IDs, 52 command-spec sources, and the identified consumer families.

The latest S0 addendum is PR #80, stacked after PR #79. It records additional source-located evidence, including all 36 inline callback entries missed by the first locator, first-order effect classes, a first-pass effect graph for the 131 `mutates=false` command entries, and the independently verified uploaded AniCraft archive/source baseline. The addendum also records that a `mutates=false` outer `Session::execute` path can still settle persistent cut/floating state without the normal snapshot/history path, and that nested execution and Publisher callbacks have different execution contexts.

**S0 remains OPEN and is not ready for closure.** The current evidence does not establish the complete transitive callback/effect graph, compiler-backed verification of all relevant workspace consumers, or successful host CI on the pinned source baseline. The command inventory and planned test IDs are not substitutes for implemented tests. Do not state that S0 is closed, do not begin runtime implementation on the basis of these audit documents, and do not treat any PR as implementation or merge authorization.

### 11.5 Non-negotiable semantic and change-control invariants

1. Smart Controller binding evaluation and Form Deform pin evaluation remain independent under CCC-01.5; no implicit multi-source composition.
2. Native Asset Warp remains host-owned and unchanged by the current FDM scope.
3. Preview remains on demand and non-mutating; continuous playback evaluation is not established.
4. AniCraft owns document state, mutation transactions, identity, persistence, history, scheduling and result publication.
5. FDM runtime implementation remains gated on closure of the required evidence/design blockers and separate explicit authorization.
6. Existing PR branches and their lineage must not be rewritten or merged without explicit authorization.
7. A CI result is valid only for the exact commit SHA and workflow run it names; do not transfer it to a later head or a different branch.

### 11.6 Immediate evidence sequence

1. Finish the callback-level transitive effect graph and record unresolved edges explicitly.
2. Reconcile the original 36-row callback manifest with the current source-located entries.
3. Complete compiler-backed consumer verification across the relevant AniCraft workspace surfaces.
4. Obtain successful host CI evidence on the pinned baseline (or re-pin and repeat the audit if the baseline changes).
5. Submit the complete package for formal S0 closure review. Until that review accepts the evidence, S0 remains OPEN.
