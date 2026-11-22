# Forma Deform — Development and Architecture

**Project role:** AniCraft deformation and rig-control tool/plugin.  
**Status:** prototype and architecture definition; native tool integration is staged and must not be represented as complete.  
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


## 11. FD-M5 — Target application first slice

**Status:** single- and multi-target current-playhead apply plus bounded timeline-range bake implemented in the Smart Controller adapter using AniCraft's frame-aware `bone.info`, `warp.info`, and `warp.pin_move` commands. GUI smoke test remains pending.

### Supported operations

1. **Preview** remains the default and is read-only.
2. **Apply current frame** requires exactly one resolved, host-verified GeometryDeform target, re-queries `warp.info`, and issues one `warp.pin_move`.
3. **Apply all current frame** accepts 1–512 unique, resolved, host-verified GeometryDeform targets. It freshly verifies every target and prepares all payloads before issuing any writes.
4. All writes in the multi-target path execute inside AniCraft's `plugin.run` transaction. A nested command failure propagates for whole-run rollback; a successful batch is one undo step.

### Undo and batch boundary

A plug-in-side batch over existing `warp.pin_move` commands is implemented. The proposed `warp.pin_move_batch` remains an optional host-owned API for lower call overhead and reusable native callers; it is not required for the current plug-in batch and does not exist yet. Its payload, validation rules, limits and host test plan are specified in [FD-M5 deformation integration](docs/FD-M5-DEFORMATION-INTEGRATION.md).

### Timeline-range bake

- `bake_range` evaluates each inclusive frame using frame-specific bone rotation inputs.
- The entire range is preflighted before writes; range and total writes are each capped at 512.
- Prepared pin writes include explicit frame IDs and commit in one `plugin.run` transaction.

### Verification and acceptance

- [x] Source-level host contract reviewed against AniCraft's `cmds_warp.rs`, `cmds_plugin.rs`, plug-in policy and atomic-run tests.
- [x] Forma Deform adapter parses `pins[].pin` (not `pins[].id`).
- [x] Multi-target preflight rejects duplicate, unresolved, unverified, unsupported and over-limit batches.
- [ ] Run in the AniCraft GUI: single/multi apply, verify keying, one-step Undo and Redo.
- [ ] Force a commit-time failure and verify whole-run rollback.
- [ ] Verify Cancel leaves the document unchanged.
- [ ] Run range bake in AniCraft and verify every frame, one-step Undo/Redo and full rollback.

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
