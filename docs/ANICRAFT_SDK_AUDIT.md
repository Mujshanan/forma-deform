# AniCraft Plugin SDK Capability Audit

**Audited:** 2026-11-19  
**Repository inspected:** [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft), branch `main`  
**Purpose:** Establish the actual v1 boundary for Forma Deform Smart Controller.

## Sources inspected

- `examples/plugins/sdk/src/lib.rs` — guest-side ABI v1 helpers.
- `examples/plugins/jitter/src/lib.rs` — command plug-in example.
- `docs/guide/plugins.md` — user-facing plug-in capabilities and sandbox rules.
- `docs/guide/reference/commands.md` — registered command reference.
- `AGENTS.md` — AniCraft architecture and command/evaluation rules.

## Capability matrix

| Requirement | Current evidence | V1 conclusion |
|---|---|---|
| WASM command plug-in | SDK exports `plugin!` and ABI v1 entry points | Supported |
| Call existing engine commands | SDK `command(id, params)` | Supported, but every command ID and parameter schema must be verified |
| Read compact document context | SDK `document()` | Supported for stage, scene, frame, layers and selection summary |
| Read selected elements | SDK `selected_elements(paths)` | Supported for selected elements on active layer at playhead |
| Enumerate elements | The raw `elements` request supports `selected: false` and returns visible elements on the active layer at the playhead; it is not a whole-document inventory | Supported for the active layer only |
| Read a bone's evaluated angle/pose | The read-only `bone.info` command returns armature bones, joint angles, limits, and spring swing at an explicit frame; the plug-in command policy permits the `bone` namespace | Supported on demand through a command query; not a continuous callback |
| Inspect geometry deformation pins | `warp.info {element?, frame?}` returns mesh pins and their IDs/positions; the `warp` namespace is allowed | Supported on demand |
| Move a geometry pin | `warp.pin_move {element?, pin, x?, y?, dx?, dy?, frame?}` changes the document | Supported as a command; use only within command-dialog preview/undo semantics |
| Set a nested graphic symbol's displayed frame | No direct instance-frame setter found in the inspected command reference | Not established; core can calculate frame, visual preview requires host support or further command research |
| Persist arbitrary plug-in binding data inside the document | Guide says plug-ins belong to AniCraft, not documents; no custom metadata API found in inspected SDK | Not supported by current documented plug-in contract |
| Read/write plug-in-owned files | Sandbox explicitly forbids file access | Not supported |
| Network or clock access | Sandbox explicitly forbids both | Not supported |
| Non-destructive command preview | Settings dialogs preview command-driven changes on stage; Cancel restores the document, while OK commits one undo step | Supported for commands with suitable target operations |
| Continuous custom evaluation during playback/scrubbing | No Smart Controller evaluator hook in inspected SDK | Not supported by current documented contract |
| Atomic command run | Guide states a command plug-in run is one undo step and failed runs roll back | Supported for a single plug-in run |

## Verified SDK surface

The guest SDK exposes:

- `command(id, params)` — invoke an existing AniCraft command.
- `document()` — obtain compact document context.
- `selected_elements(paths)` — obtain selected active-layer elements at the playhead. The underlying ABI also accepts an `elements` request with `selected: false` to enumerate visible elements on the active layer.
- `log(message)` — report diagnostics.
- Exporter-only `movie()` and `frame(index)` helpers.

The command reference documents `bone.info {frame?, layer?, points?}` as a read-only query returning armatures and their bones, including joint angle and spring swing at the requested frame. The plug-in command policy explicitly permits the `bone` namespace. Therefore an on-demand preview can query a bone's angle through the command interface; there is still no continuous evaluator callback.
- `plugin!`, `exporter!`, and `ease!` ABI macros.

The SDK source also documents that native builds use host-call stand-ins that return errors, allowing pure logic to be tested with native `cargo test`. The SDK crate itself is currently a local path dependency inside AniCraft's `examples/plugins` workspace, rather than a separately published dependency; the independent Forma Deform repository must choose a pinned vendoring/submodule strategy or a separately distributable SDK package before building its adapter.

The command plug-in example reads selection data and invokes existing commands such as `element.select` and `modify.transform`. The host's `plugin_may_call` policy allows the `bone` and `warp` namespaces. The command reference documents `bone.info`, `warp.info`, and `warp.pin_move`; the last one changes the document and therefore must be used only through the host's preview/undo transaction when previewing. This demonstrates useful on-demand operations, not a persistent controller runtime.

## Consequences for Forma Deform v1

1. **Native core first is viable.** Keep all model, validation, normalization, easing, and target mapping in a normal Rust library with no host imports.
2. **A WASM command plug-in is viable.** The adapter can expose authoring and explicit-angle preview actions using verified existing commands.
3. **Document persistence is a host gap.** The plug-in cannot write arbitrary files, and the documented contract provides no arbitrary document metadata API. Durable bindings need a small first-party AniCraft extension (for example, a versioned document-model field plus engine commands) or another explicitly approved persistence design.
4. **Bone-angle input is available on demand.** The read-only `bone.info` command returns bone angles at an explicit frame and is allowed by the plug-in command policy. The adapter can query this command for preview; it cannot subscribe to continuous pose changes.
5. **Geometry preview has a plausible command path.** `warp.info` exposes mesh pins and `warp.pin_move` moves a pin at a frame. This is a document mutation, but command-dialog preview can roll it back on Cancel. A geometry target must identify the specific stable pin, not merely the containing element. Exact command parameter handling still needs adapter tests.
6. **Symbol-frame preview remains unverified.** The inspected command reference exposes symbol loop/type operations but no direct command for temporarily setting a graphic symbol instance's displayed frame. The core can calculate the desired frame, but visual application needs a verified host operation or a small host extension.
7. **No continuous runtime claim.** A command plug-in is invoked as a command; it is not a custom evaluator in AniCraft's frame-evaluation pipeline.

## Host extension candidates (not yet implemented)

If durable document storage and live controller input are required, the smallest likely host extension is:

- A versioned document-owned collection of controller bindings.
- Engine commands to add, update, remove, and inspect bindings.
- Stable target identity validation.
- A read-only query returning the evaluated bone input at an explicit frame.
- Later, a pure extension to document/frame evaluation that consumes bindings and produces effective target values without rewriting authored keyframes.

These are proposals only. They require separate implementation planning and review in the AniCraft repository. This audit does not authorize changes to AniCraft.


## 6. Frame-script capability check

The current AniCraft plug-in API also supports `kind: "frame-script"`, which is distinct from command plug-ins:

- A frame script runs when playback enters a keyframe that names its entry point.
- Every invocation starts a fresh WebAssembly instance.
- It can read `player` state (scene/frame, playback state, clips, pointer and timeline) and queue timeline actions such as `set_property` or `goto_and_stop`.
- It cannot call `command`, `document`, or `elements`; those requests are explicitly refused.
- It therefore cannot call `bone.info` or `warp.info`, and cannot implement a bone-angle-driven Smart Controller. It is also keyframe-entry driven rather than a persistent per-frame controller evaluator.

Frame scripts are useful for playback interactivity, but they do not close the live Smart Controller runtime gap. The v1 scope remains an on-demand command plug-in.

## 7. SDK packaging decision

The AniCraft guest SDK is a small source crate under `examples/plugins/sdk`, but it is a local path dependency in AniCraft's sample-plugin workspace and is not a separately published crate. A Cargo git dependency cannot select that nested package by subdirectory.

For this repository, vendor the audited SDK source as `crates/anicraft-plugin-sdk` with a standalone manifest. Pin provenance to the exact upstream AniCraft commit in `crates/anicraft-plugin-sdk/UPSTREAM.md`. This avoids pulling the entire AniCraft repository as a submodule and keeps native host-call stubs available for testing. Update the vendored copy only through an explicit SDK diff review.

Pinned upstream source commit: [bcfea816478f2a00ab172cefe67f0bd5696b3886](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886).


## 8. Exact target command contracts

The current AniCraft command reference and engine implementations were checked for the v1 operations:

- bone.info is read-only. Parameters: frame?, layer?, and points?. Each bone record includes a numeric stable bone ID, angle in degrees, limits, and swing in degrees. The implementation reports the constrained keyed pose angle separately from the spring contribution. If Smart Controller input is intended to follow the fully sprung displayed pose, the adapter must use angle + swing; this choice must be explicit and covered by tests.
- warp.info is read-only. Parameters: element? and frame?. It returns the numeric element ID and pins with numeric pin IDs, current stage-space at coordinates, and artwork-space rest coordinates.
- warp.pin_move is mutating. Parameters: element?, required pin, either both x and y or neither, optional dx and dy, and frame?. Absolute x/y coordinates are stage-space; dx/dy are stage-space deltas. The command keys the pin at the requested frame and participates in AniCraft's document undo transaction.

The core's string IDs are adapter-neutral identifiers; the current AniCraft adapter must parse and validate decimal numeric host IDs rather than assuming arbitrary strings are accepted by host commands. A host ID is not interchangeable with an array index or current selection position.
