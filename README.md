# Forma Deform

A reusable 2D deformation project with an AniCraft plugin suite. The original TypeScript rectangle/timeline demo remains a separate geometry experiment.

## AniCraft plugin suite

The branch `feat/smart-controller-v1` contains two command plugins:

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **AniCraft Smart Controller** — one unified command with Slider mode for bone-angle-to-action-frame mapping and Deform mode for bone-driven Asset Warp pin targets.

The former standalone Form Deform plugin has been consolidated into Smart Controller. The independent `form-deform-core` crate remains responsible for pure deformation math; `smart-controller-core` remains responsible for Slider mapping. The Smart Controller adapter composes both with the generic AniCraft SDK.

The guest SDK and host runtime boundary is specified in the versioned [AniCraft Plugin ABI v1](docs/PLUGIN_ABI_V1.md). The SDK implementation is in `crates/anicraft-plugin-sdk`; plugins must use it rather than declaring raw imports or exports themselves.

See [plugin overview](plugins/README.md), the [build and smoke-test guide](plugins/INSTALL.md), [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md), and [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md).

## Important host boundaries

The current AniCraft command-plugin API supports on-demand bone and warp queries and undoable warp-pin edits. It does not expose durable arbitrary plugin metadata, continuous playback/scrubbing evaluation, or a verified command to set a nested symbol instance's displayed frame. The suite does not claim those capabilities. Apply operations are explicit document edits; calculation-only mode is the default.

## Verification

Run the complete Rust suite and build both WASM plugins:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The TypeScript demo retains its Bun-based scripts. GitHub Actions checks both projects.
