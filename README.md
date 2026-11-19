# Forma Deform

**Forma Deform** is a reusable 2D deformation project with an AniCraft plugin suite. The original TypeScript rectangle/timeline demo remains a separate geometry experiment.

## AniCraft plugin suite

The branch feat/smart-controller-v1 contains three separate command plugins:

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **Form Deform** — deterministic bone-angle-driven Asset Warp pin deformation using an authored neutral position and per-degree gains.
- **Smart Controller** — general-purpose neutral-centered input mapping with Linear/Smoothstep easing and three geometry anchors.

Each plugin has its own crate and manifest. Shared native logic is isolated under crates/. The AniCraft guest SDK is vendored with pinned upstream provenance.

See [plugin overview](plugins/README.md), [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md), and [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md).

## Important host boundaries

The current AniCraft command-plugin API supports on-demand bone and warp queries and undoable warp-pin edits. It does not expose durable arbitrary plugin metadata, continuous playback/scrubbing evaluation, or a verified command to set a nested symbol instance's displayed frame. The suite does not claim those capabilities. Apply operations are explicit document edits; calculation-only mode is the default.

## Verification

Run the complete Rust suite and build all three WASM plugins:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The TypeScript demo retains its Bun-based scripts. GitHub Actions checks both projects.
