# Build and install the AniCraft plugin suite

The repository builds three separate command plugins. Build them from the repository root with a current stable Rust toolchain:

```powershell
rustup target add wasm32-unknown-unknown
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The three modules are:

| Plugin | WASM file |
|---|---|
| Bone Query | target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm |
| Form Deform | target/wasm32-unknown-unknown/release/anicraft_form_deform_plugin.wasm |
| Smart Controller | target/wasm32-unknown-unknown/release/anicraft_smart_controller_plugin.wasm |

Install each module in AniCraft through **Commands → Install Plug-in…**. They appear as separate commands.

## Suggested smoke-test project

Prepare one project containing:

- An armature with at least one posed bone and a known numeric bone ID.
- A shape on the stage with an Asset Warp mesh and at least one pin.
- The element ID and pin ID returned by AniCraft's Warp Mesh command.

Then test in this order:

1. Run Bone Query. Confirm the armature, bone ID, keyed angle, and spring fields are reported correctly. Turn on shape-point reporting if the rig has shape skins.
2. Run Form Deform with Apply disabled. Enter the known bone/element/pin IDs, neutral angle, neutral pin stage position, and small per-degree gains. Confirm the reported target is stable when rerun at the same frame. Enable Apply only after the calculation is correct; confirm Undo restores the previous pin position.
3. Run Smart Controller with bone input enabled. Set a valid input range and three output anchors. First use Apply disabled and check negative, neutral, and positive poses. Then enable Apply and verify the target pin moves to the calculated absolute stage position and Undo restores it.

For an isolated controller test, disable Read controller bone at playhead and provide an explicit input angle. Keep spring swing disabled for the first pass.

## Important

- Form Deform and Smart Controller calculate absolute target positions from authored neutral/base values; do not use the currently deformed pin position as the neutral base.
- Apply is a document edit, not a persistent live preview. It is undoable.
- The plugins do not store their parameters as durable document-owned bindings.
- No plugin runs continuously during playback or scrubbing.
- Smart Controller's SymbolFrame core calculation is not visually applied because the current host API does not expose a verified symbol-instance frame setter.
- Native unit tests and WASM builds are covered by GitHub Actions; in-app smoke testing still requires AniCraft.
