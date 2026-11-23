# Native Host Integration Traceability

**Checked:** 2026-11-23  
**Repositories:** [Forma Deform](https://github.com/Mujshanan/forma-deform) and [AniCraft](https://github.com/Mujshanan/anicraft)  
**Purpose:** Distinguish the native AniCraft implementation from the historical WASM plugin probes and record which integration seams are actually connected.

## Executive finding

The project history is: three separate proof-of-concept plugins → one consolidated Forma Deform product/codebase → development toward a native AniCraft tool. The three plugin crates still present in the Forma Deform workspace are not three separately shipped products.

There is concrete native Smart Controller UI work in AniCraft branch [`feat/smart-controller-ui-01-03`](https://github.com/Mujshanan/anicraft/tree/feat/smart-controller-ui-01-03), but it is a partial, unmerged UI slice. It adds a toolbar tool, transient Stage handles, and a basic Properties section for the selected transient handle. The branch does not yet connect authored controller persistence, source-angle resolution, the controller core, target application, or live preview. AniCraft `main` does not contain this native Smart Controller work.

Therefore:
- It is inaccurate to describe Forma Deform as only a proposed native tool or only a WASM plugin suite.
- It is also inaccurate to describe the native Smart Controller workflow as complete.
- The right status is **consolidated product with reusable core and plugin probes, plus a partially implemented native AniCraft UI branch whose document/evaluation integration remains incomplete**.

## Product and code ownership

| Area | Owner | Evidence / boundary |
|---|---|---|
| Controller/deformation domain rules and deterministic evaluation | Forma Deform core | `smart-controller-core` and `form-deform-core` in the Forma Deform workspace |
| Native toolbar, Stage interaction, basic handle Properties section and transient UI state | AniCraft | Native UI files on `feat/smart-controller-ui-01-03` |
| Authoritative document schema and persistence | AniCraft | Not implemented for Smart Controller in the inspected feature branch |
| Commands, undo/redo and transactional edits | AniCraft | Existing command/session architecture; no Smart Controller authored-state commands in the inspected branch |
| Evaluated rig input, evaluation scheduling and target application | AniCraft host adapter + Forma Deform core | Not connected in the inspected feature branch |
| Historical WASM command adapters | Forma Deform workspace | `plugins/bone-query`, `plugins/form-deform`, `plugins/smart-controller`; independently buildable probes, not separate shipped products |

## Evidence in AniCraft `main`

The current `main` tree contains the existing native host capabilities:
- Asset Warp UI and command path: [`src/warp_tool.rs`](https://github.com/Mujshanan/anicraft/blob/main/src/warp_tool.rs), [`crates/engine/src/cmds_warp.rs`](https://github.com/Mujshanan/anicraft/blob/main/crates/engine/src/cmds_warp.rs), and the document warp model [`crates/doc/src/warp.rs`](https://github.com/Mujshanan/anicraft/blob/main/crates/doc/src/warp.rs).
- Existing bone and bind tools: [`src/bone_tool.rs`](https://github.com/Mujshanan/anicraft/blob/main/src/bone_tool.rs) and [`src/bind_tool.rs`](https://github.com/Mujshanan/anicraft/blob/main/src/bind_tool.rs).
- A general WASM plugin runtime and registry: [`crates/plugins/src/registry.rs`](https://github.com/Mujshanan/anicraft/blob/main/crates/plugins/src/registry.rs) and [ADR 0017](https://github.com/Mujshanan/anicraft/blob/main/docs/adr/0017-wasm-plugins.md).

The recursive `main` source tree did not contain `src/smart_controller_tool.rs`, `crates/controller`, or other Smart Controller-specific native paths. The root [AniCraft workspace manifest](https://github.com/Mujshanan/anicraft/blob/main/Cargo.toml) also does not declare an `anicraft-controller` application dependency.

These existing systems are reusable host infrastructure, but their existence alone does not establish a Forma Deform integration path.

## Evidence in AniCraft `feat/smart-controller-ui-01-03`

The inspected branch was 17 commits ahead of `main` at commit [`59ed1f4871b543ec78da13f331dae7f0a99beedf`](https://github.com/Mujshanan/anicraft/commit/59ed1f4871b543ec78da13f331dae7f0a99beedf). The branch had no associated pull request returned by the GitHub pull-request query at the time of inspection.

| File | Observed implementation | What it proves / does not prove |
|---|---|---|
| [`src/main.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/src/main.rs) | Declares `mod smart_controller_tool` | UI module is compiled into this branch's application |
| [`src/app.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/src/app.rs) | Adds `Tool::SmartController`, labels/hint, transient handle collection and selected index | Native tool identity and transient state exist |
| [`src/stage.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/src/stage.rs) | Draws handle overlay and routes Smart Controller clicks to the tool module | Stage interaction exists |
| [`src/panels.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/src/panels.rs) | Adds a basic Properties section for the selected transient handle | A small handle-properties surface exists, not a connected driver/target binding inspector |
| [`src/smart_controller_tool.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/src/smart_controller_tool.rs) | Draws transient handles and a draft inspector; tests nearest-handle and draft validation | UI scaffold exists; it does not persist definitions or evaluate a controller |
| [`crates/controller/src/lib.rs`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/crates/controller/src/lib.rs) | Adds a pure scalar-to-frame mapper from a `BoneId` and nested `InstancePath` | A small native mapping prototype exists, but not its host adapter or runtime |
| [`crates/controller/README.md`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/crates/controller/README.md) | Lists persistence, source-angle resolution, target application, and authoring UI as not implemented | Accurate for functional controller authoring; a transient Stage/handle UI shell exists but does not complete that workflow |
| [`docs/SMART_CONTROLLER_UI_ACCEPTANCE.md`](https://github.com/Mujshanan/anicraft/blob/feat/smart-controller-ui-01-03/docs/SMART_CONTROLLER_UI_ACCEPTANCE.md) | Defines create/select/edit/preview/save/reopen workflow and automated/manual acceptance | This is a completion checklist, not evidence that the workflow passes |

### Important implementation gaps

1. **No durable controller model in the document.** The UI uses transient `Vec<Vec2>`-style handle state and an in-memory draft; it does not serialize controller definitions.
2. **No controller commands.** No create/update/delete controller command or undo/redo contract was found in this branch's change set.
3. **No native core adapter.** The app root manifest does not depend on `anicraft-controller`; `src/smart_controller_tool.rs` does not call it.
4. **No evaluated source resolver.** The UI does not obtain a bone's evaluated angle and pass it into the mapping core.
5. **No connected binding inspector.** A driver/target draft UI helper is present in the tool module, but the active Properties section only edits transient handle coordinates; the draft helper is not wired into the application panel.
6. **No target application or live preview/reset transaction.** The core returns a requested nested symbol frame, but no host path applies it to player/evaluation state. The tool hint explicitly says binding and preview are not connected.
7. **No completion evidence.** The acceptance document requires command tests, serialization round trips, undo/redo, preview immutability, save/reopen, and a manual native Windows smoke test. A source tree or compile alone does not satisfy that gate.

The native prototype's target is a nested symbol instance frame. Forma Deform's deformation core also addresses geometry targets and pin deformation. Keep those semantics explicit; do not silently conflate Smart Controller symbol-frame mapping with Form Deform pin evaluation.

## Documentation reconciliation performed in Forma Deform

The following documentation changes are made on the dedicated branch `quality/native-host-traceability-docs`:

- `README.md`: records the three-plugin → unified product → native tool evolution, and accurately describes the unmerged native UI slice.
- `DEVELOPMENT.md`: clarifies the product boundary and assigns native UI/document ownership to AniCraft while keeping deterministic domain logic in Forma Deform.
- `docs/ANICRAFT_SDK_AUDIT.md`: preserves the original WASM SDK audit and adds a dated note distinguishing that audit from the newer native host branch.
- This report: records the exact host source paths, current branch state, gaps, and acceptance requirements.

## Recommended next integration sequence

1. Reconcile the stale `crates/controller/README.md` statement on the AniCraft feature branch: the UI view exists, but is only transient scaffolding.
2. Decide whether the native `anicraft-controller` prototype is the intended canonical Smart Controller core or whether it should adapt the Forma Deform `smart-controller-core`. Avoid two competing controller models.
3. Add a versioned document-owned controller schema and explicit create/update/delete commands, with stable IDs and undo/redo tests.
4. Add a host adapter that resolves evaluated bone input, calls the chosen pure core, and applies output through a temporary preview/evaluation layer without rewriting authored keyframes.
5. Add preview reset/immutability tests and save/close/reopen serialization tests.
6. Run the native AniCraft Windows smoke test required by the acceptance document and record the exact host commit and fixture.
7. Only then describe the native Smart Controller workflow as integrated. Keep Smart Controller output and Form Deform pin-deformation output independent unless an explicit composition design is approved.

## Scope and limitations

This is a source-level traceability review of the referenced GitHub branches and files, not a local AniCraft build, runtime test, or manual GUI smoke test. It does not modify the AniCraft repository or imply that its feature branch is merged. The user's stated product history is reflected in the product narrative; implementation status is based on the inspected source evidence above.
