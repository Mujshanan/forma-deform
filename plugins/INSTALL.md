# Build and install the AniCraft plug-in suite

This file is the operational build/install runbook. The project architecture, scope, current status, and staged roadmap are maintained only in the root [DEVELOPMENT.md](../DEVELOPMENT.md).

The repository builds three separate command plug-ins. Build them from the repository root with a current stable Rust toolchain:

```powershell
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The three modules are:

| Plug-in | WASM file |
|---|---|
| Bone Query | `target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm` |
| Form Deform | `target/wasm32-unknown-unknown/release/anicraft_form_deform_plugin.wasm` |
| Smart Controller | `target/wasm32-unknown-unknown/release/anicraft_smart_controller_plugin.wasm` |

Install each module in AniCraft through **Commands → Install Plug-in…**. They appear as separate commands.

## Suggested smoke-test project

Prepare one project containing:
- An armature with at least one posed bone and a known numeric bone ID.
- A shape on the stage with an Asset Warp mesh and at least one pin.
- The element ID and pin ID returned by AniCraft's Warp Mesh command.

Test in this order:
1. Run Bone Query and verify the reported armature/bone identity and pose data.
2. Run Form Deform with Apply disabled; verify the calculated target. Enable Apply only after the result is correct, then confirm Undo restores the previous pin position.
3. Run Smart Controller with Apply disabled and explicit manual input first; inspect negative, neutral and positive cases. Test host bone input separately after confirming the host response.

## Important boundaries

- Form Deform calculates an absolute target from its authored neutral position; do not use the currently deformed pin as the neutral base.
- Apply is an explicit document edit, not a persistent live preview.
- The current command plug-ins do not store durable controller bindings in the AniCraft document.
- The plug-ins do not run continuously during playback or scrubbing.
- A calculated SymbolFrame value is not proof that the host visually applies that frame.
- Native unit tests and WASM builds are separate from in-app smoke testing; report each independently.
