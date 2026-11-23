# Build and smoke-test the historical AniCraft adapters

This guide is for developers validating the repository's retained ABI v1 command adapters. It is not a recommendation to deploy them as the final native Forma Deform workflow.

Build them from the repository root with a current stable Rust toolchain:

```powershell
rustup target add wasm32-unknown-unknown
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The generated modules are:

| Adapter | WASM file |
|---|---|
| Bone Query | `target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm` |
| Form Deform | `target/wasm32-unknown-unknown/release/anicraft_form_deform_plugin.wasm` |
| Smart Controller | `target/wasm32-unknown-unknown/release/anicraft_smart_controller_plugin.wasm` |

Install a module in AniCraft through **Commands → Install Plug-in…** only when running the corresponding development smoke test.

## Suggested smoke tests

Prepare a project with an armature, at least one posed bone, a shape with an Asset Warp mesh, and a known pin. Record the numeric bone, element, and pin IDs returned by AniCraft.

1. **Bone Query:** run the read-only inspection. Confirm bone IDs, keyed angle, spring fields, and optional shape-point data against the host project.
2. **Form Deform:** first run with Apply disabled. Check the absolute target derived from the authored neutral position and per-degree gains. Only then test Apply and confirm AniCraft Undo restores the previous pin position.
3. **Smart Controller V1:** run the Slider preview using either the on-demand bone input or an explicit `manual_value`. Inspect normalized input and the calculated action frame in the log. The current V1 command is preview-only: it does not apply a target pin, set the timeline, or provide continuous playback evaluation.

The Smart Controller crate also contains a canonical, read-only preview path and target-capability planning helpers. Those helpers are not equivalent to a complete native authoring workflow and are not the same as a host target setter.

## Important boundaries

- Form Deform and Smart Controller evaluation remain independent.
- Form Deform's legacy adapter calculates one pin target per invocation; it does not provide the modern FDM topology/binding/evaluation workflow.
- Smart Controller's active V1 command is a Slider preview. Do not expect an Apply option in that command.
- The adapters do not store durable plugin-owned bindings in the AniCraft document.
- No adapter runs continuously during playback or scrubbing.
- Smart Controller's SymbolFrame core calculation is not visually applied because the current host API does not expose a verified symbol-instance frame setter.
- CI tests and WASM builds do not replace an in-app smoke test.

See [Legacy Adapter Status and Retirement Plan](../docs/LEGACY_ADAPTER_RETIREMENT.md) before proposing adapter removal.
