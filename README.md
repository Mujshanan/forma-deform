# Forma Deform

**Forma Deform** is a reusable 2D deformation project with an AniCraft plugin suite. The original TypeScript rectangle/timeline demo remains a separate geometry experiment.

## Current AniCraft integrations

The current implementation contains three separate command plug-ins:

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **Form Deform** — deterministic bone-angle-driven Asset Warp pin deformation using an authored neutral position and per-degree gains.
- **Smart Controller** — general-purpose neutral-centered input mapping with Linear/Smoothstep easing and three geometry anchors.

Each current plug-in has its own crate and manifest. Shared native logic is isolated under crates/. The AniCraft guest SDK is vendored with pinned upstream provenance.

**Smart Controller product direction:** the command plug-in is a prototype/compatibility implementation, not the intended final UI. The target is a first-party native AniCraft editor tool, using AniCraft's existing Tools and Properties conventions and exposing the same registered command contracts through AniCraft's existing MCP server. This requires coordinated host-side work in [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft). See the [native-tool and MCP proposal](docs/SMART_CONTROLLER_NATIVE_TOOL_PROPOSAL.md) and [updated Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md).

See [plugin overview](plugins/README.md), the [unified build and smoke-test guide](plugins/INSTALL.md), and the [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md).

## Important host boundaries

The current AniCraft command-plug-in API supports on-demand bone and warp queries and undoable warp-pin edits. It does not expose durable arbitrary plug-in metadata, continuous playback/scrubbing evaluation, or a verified command to set a nested symbol instance's displayed frame. The suite does not claim those capabilities. Apply operations are explicit document edits; calculation-only mode is the default.

The native Smart Controller proposal requires first-party AniCraft integration for its tool UI, durable controller definitions, shared commands, and eventual frame-evaluation hook. The proposal does not claim those host capabilities are implemented.

## Verification

Run the complete Rust suite and build all three WASM plug-ins:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The TypeScript demo retains its Bun-based scripts. GitHub Actions checks both projects.
