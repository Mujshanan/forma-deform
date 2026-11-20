# Form Deform

Form Deform is an AniCraft command adapter over a host-independent deformation engine.

## Architecture boundary

- `form-deform-core` owns domain types, bone transforms, weighted mesh deformation, pin references, validation, and mathematical evaluation.
- `anicraft-form-deform-plugin` owns the AniCraft manifest, ABI entry point, host calls, Serde DTOs, and translation between host payloads and core inputs/outputs.
- `anicraft-plugin-sdk` owns the generic JSON/WASM ABI transport only. It contains no Form Deform rules or geometry math.

The core has no Serde, JSON, SDK, or WASM dependency. It can be compiled and tested as an ordinary Rust library.

## Core model

- `BonePose` / `BoneTransform`: stable bone identity and evaluated 2D world transform.
- `MeshVertex` / `BoneWeight`: bind-space vertex and explicit normalized skin influences.
- `DeformMesh` / `MeshPin`: mesh vertices and stable pin-to-vertex references.
- `deform_mesh`: linear-blend skinning using evaluated bone transforms; validates influence IDs, weights, and pin indices.
- `DeformRequest` / `DeformResult`: deterministic absolute pin target evaluation from authored neutral data.

The current `bone.info` and `warp.info` host responses used by this plugin expose bone angles and pin identifiers, but do not provide the complete bind mesh, skin weights, or evaluated bone world transforms needed to run `deform_mesh` on live AniCraft geometry. The adapter therefore does not fabricate those fields or claim to apply the core mesh solver to host geometry. Integrating live mesh deformation requires an explicit host API/DTO contract that supplies those values. Until then, the active command path uses the core's authored pin-target evaluator and the host's documented pin-move command.

## Current command workflow

1. Select a controller bone ID, warped element ID, and pin ID.
2. Enter the authored neutral angle and neutral stage-space pin position.
3. Enter X/Y displacement gains in pixels per degree.
4. Run without Apply to inspect the computed absolute target.
5. Enable Apply to send the explicit `warp.pin_move` command.

Every evaluation is derived from authored neutral values; it does not accumulate from the displayed pin position. Spring swing is included only when explicitly requested and present.

## Build and tests

```sh
cargo test -p form-deform-core
cargo test -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
```
