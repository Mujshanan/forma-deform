# Forma Deform

A reusable 2D deformation core and AniCraft plugin suite. Geometry/document/timeline behavior and deformation math are implemented authoritatively in Rust.

## Rust core crates

- **forma-document-core** — versioned project documents, rectangle geometry, hit testing, keyframes, and timeline interpolation. This replaces the former TypeScript-only demo logic.
- **form-deform-core** — bone transforms, weighted mesh deformation, pin targets, structured errors, and opt-in diagnostics.
- **smart-controller-core** — pure Slider binding validation and input-to-target mapping.
- **anicraft-plugin-sdk** — generic versioned guest ABI and host communication boundary.

The cores are host-independent. UI, WASM, persistence, and host-specific behavior belong in adapters; do not reimplement geometry or deformation algorithms in TypeScript.

## AniCraft plugin suite

The branch `feat/smart-controller-v1` contains two command plugins:

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **AniCraft Smart Controller** — one unified command with Slider mode for bone-angle-to-action-frame mapping and Deform mode for bone-driven Asset Warp pin targets.

The former standalone Form Deform plugin has been consolidated into Smart Controller. The Smart Controller adapter composes the independent cores with the generic AniCraft SDK.

The guest SDK and host runtime boundary is specified in the versioned [AniCraft Plugin ABI v1](docs/PLUGIN_ABI_V1.md). The SDK implementation is in `crates/anicraft-plugin-sdk`; plugins must use it rather than declaring raw imports or exports themselves.

## Deformation diagnostics

Call `deform_mesh_with_diagnostics` with `DeformDiagnosticsOptions`. Metrics collection is opt-in. When enabled, metrics report vertex/bone counts, elapsed nanoseconds on native targets, and the interpolation mode. Solver metrics are `None` because the current implementation is linear-blend skinning and does not run an iterative solver. On `wasm32-unknown-unknown`, elapsed time is `None`; time the call in the host adapter if required.

Triangle degeneracy and zero-length-bone warnings require callers to supply triangle indices and bone segment endpoints respectively. The core does not guess missing topology or bone lengths. Structured errors expose a stable machine-readable `DeformErrorCode` plus the typed source error.

## Important host boundaries

The current AniCraft command-plugin API supports on-demand bone and warp queries and undoable warp-pin edits. It does not expose durable arbitrary plugin metadata, continuous playback/scrubbing evaluation, or a verified command to set a nested symbol instance's displayed frame. The suite does not claim those capabilities. Apply operations are explicit document edits; calculation-only mode is the default.

## Verification

Run the complete Rust suite and build both WASM plugins:

```sh
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```
