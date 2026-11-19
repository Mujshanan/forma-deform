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
| Enumerate arbitrary document elements | No typed SDK helper found in inspected SDK | Not established; do not assume |
| Read a bone's evaluated angle/pose | No typed SDK helper or runtime callback found | Not established |
| Persist arbitrary plug-in binding data inside the document | Guide says plug-ins belong to AniCraft, not documents; no custom metadata API found in inspected SDK | Not supported by current documented plug-in contract |
| Read/write plug-in-owned files | Sandbox explicitly forbids file access | Not supported |
| Network or clock access | Sandbox explicitly forbids both | Not supported |
| Non-destructive command preview | Settings dialogs document live stage preview with Cancel restoring the document; accepted result is one undo step | Supported for the plug-in command's command-driven changes, subject to host behavior |
| Continuous custom evaluation during playback/scrubbing | No Smart Controller evaluator hook in inspected SDK | Not supported by current documented contract |
| Atomic command run | Guide states a command plug-in run is one undo step and failed runs roll back | Supported for a single plug-in run |

## Verified SDK surface

The guest SDK exposes:

- `command(id, params)` — invoke an existing AniCraft command.
- `document()` — obtain compact document context.
- `selected_elements(paths)` — obtain selected active-layer elements at the playhead.
- `log(message)` — report diagnostics.
- Exporter-only `movie()` and `frame(index)` helpers.
- `plugin!`, `exporter!`, and `ease!` ABI macros.

The SDK source also documents that native builds use host-call stand-ins that return errors, allowing pure logic to be tested with native `cargo test`.

The command plug-in example reads selection data and invokes existing commands such as `element.select` and `modify.transform`. This demonstrates command orchestration, not a persistent controller runtime.

## Consequences for Forma Deform v1

1. **Native core first is viable.** Keep all model, validation, normalization, easing, and target mapping in a normal Rust library with no host imports.
2. **A WASM command plug-in is viable.** The adapter can expose authoring and explicit-angle preview actions using verified existing commands.
3. **Document persistence is a host gap.** The plug-in cannot write arbitrary files, and the documented contract provides no arbitrary document metadata API. Durable bindings need a small first-party AniCraft extension (for example, a versioned document-model field plus engine commands) or another explicitly approved persistence design.
4. **Bone-driven input is a host gap.** The inspected plug-in surface does not provide an evaluated bone-angle read or callback. V1 can preview a caller-supplied angle; reading the live controller requires host support.
5. **Visual preview is conditional.** The settings dialog can preview command-driven changes and roll them back on Cancel. We still need to verify that the exact target operations required by Smart Controller can be expressed safely with existing commands. If not, return calculated preview data only.
6. **No continuous runtime claim.** A command plug-in is invoked as a command; it is not a custom evaluator in AniCraft's frame-evaluation pipeline.

## Host extension candidates (not yet implemented)

If durable document storage and live controller input are required, the smallest likely host extension is:

- A versioned document-owned collection of controller bindings.
- Engine commands to add, update, remove, and inspect bindings.
- Stable target identity validation.
- A read-only query returning the evaluated bone input at an explicit frame.
- Later, a pure extension to document/frame evaluation that consumes bindings and produces effective target values without rewriting authored keyframes.

These are proposals only. They require separate implementation planning and review in the AniCraft repository. This audit does not authorize changes to AniCraft.
