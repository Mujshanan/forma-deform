# Build and install the AniCraft plugin suite

The repository builds two separate command plugins. Build them from the repository root with a current stable Rust toolchain:

```powershell
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The two modules are:

| Plugin | WASM file |
|---|---|
| Bone Query | target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm |
| AniCraft Smart Controller | target/wasm32-unknown-unknown/release/anicraft_smart_controller_plugin.wasm |

Install each module in AniCraft through **Commands → Install Plug-in…**. Smart Controller exposes Slider and Deform modes from one installed command.

## Suggested smoke-test project

Prepare one project containing:
- An armature with posed bones and known bone IDs/names.
- A shape on the stage with an Asset Warp mesh and at least one pin.
- The element ID and pin ID returned by AniCraft's Warp Mesh command.

Then test in this order:

1. Run Bone Query. Confirm armature, bone IDs, keyed angles, and spring fields are reported correctly.
2. Run Smart Controller in Slider mode with Apply-independent preview inputs. Test negative, neutral, and positive driver angles; verify Exclusive and Inclusive endpoint behavior.
3. Run Smart Controller in Deform mode with Apply disabled. Enter the known bone/element/pin IDs, neutral angle, neutral pin stage position, and small per-degree gains. Confirm the reported target is stable when rerun at the same frame.
4. Enable Apply only after the Deform calculation is correct; confirm the pin moves to the calculated absolute stage position and Undo restores its previous position.

For isolated deterministic tests, provide the manual input value in Slider mode. Keep spring swing disabled for the first Deform pass.

## Important

- Deform mode calculates absolute target positions from authored neutral values; do not use the currently deformed pin position as the neutral base.
- Apply is a document edit, not a persistent live preview. It is undoable.
- The plugin does not store parameters as durable document-owned bindings.
- No plugin runs continuously during playback or scrubbing.
- Smart Controller's SymbolFrame core calculation is not visually applied because the current host API does not expose a verified symbol-instance frame setter.
- Native unit tests and WASM builds are covered by GitHub Actions; in-app smoke testing still requires AniCraft.
