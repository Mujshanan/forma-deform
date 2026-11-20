# AniCraft Smart Controller

A single AniCraft command plugin with two currently supported controller modes. Both modes are on-demand; neither evaluates continuously during playback or scrubbing.

## Modes

### Slider

Maps a driver bone's evaluated angle to an action-frame preview.

1. Select the driver armature/layer and driver bone name.
2. Select the action layer and configure its start/end frame range.
3. Set minimum and maximum driver rotation angles.
4. Choose Exclusive (default) or Inclusive endpoint policy.
5. Run using the host bone input, or provide `manual_value` for deterministic tests.
6. Inspect normalized input and calculated output frame in the plugin log.

Manual input overrides the host query. A missing or malformed bone response is an error; it is never silently replaced with zero.

- Exclusive maps [start, end). The end frame is reserved as the wrap point and is never selected.
- Inclusive maps [start, end]. Both endpoint frames can be selected.
- Reversed rotation endpoints are valid and reverse the mapping direction.
- Transform mode remains deferred until verified host transform read/write APIs are available.

### Deform

Drives one Asset Warp pin from a bone angle using authored neutral position and per-degree gains.

1. Select a controller bone ID, warped element ID, and pin ID.
2. Enter the authored neutral angle and neutral stage-space pin position.
3. Enter X/Y displacement gains in pixels per degree.
4. Optionally include spring swing (only when the host provides it).
5. Run with Apply disabled to inspect the absolute target.
6. Enable Apply to send the explicit `warp.pin_move` command.

Every evaluation is derived from authored neutral values; it does not accumulate from the displayed pin position. The edit is undoable.

## Architecture boundaries

- `smart-controller-core` owns Slider mapping, validation, normalization, endpoint handling, and preview calculations.
- `form-deform-core` owns Deform domain types, bone transforms, weighted mesh deformation, pin references, validation, and mathematical evaluation. It has no Serde, JSON, SDK, or WASM dependency.
- `anicraft-smart-controller-plugin` composes both pure cores and owns the manifest, ABI entry point, host calls, Serde DTOs, and translation between host payloads and core inputs/outputs.
- `anicraft-plugin-sdk` owns generic JSON/WASM ABI transport only. It contains no Smart Controller rules or deformation math.

The runtime data path is: pure mode-specific core calculation → plugin adapter/SDK serialization and ABI → AniCraft plugin runtime. The plugin crate depends on both independent core crates and the SDK; neither core depends on the SDK or host runtime.

The mesh skinning API in `form-deform-core` is pure groundwork. Current `bone.info` and `warp.info` responses do not expose complete bind mesh data, skin weights, or evaluated bone world transforms, so Deform mode does not claim to apply that solver to live host geometry. It currently evaluates an authored absolute pin target and calls the documented pin-move command.

## Current host limitations

- Both modes are on-demand command executions, not playback/scrubbing evaluators.
- The plugin does not set the AniCraft timeline or write bone properties.
- Bindings are not durably persisted in the AniCraft document.
- Live mesh deformation requires a verified host contract supplying geometry and evaluated bone transforms.

## Build and test

```sh
cargo test -p smart-controller-core
cargo test -p form-deform-core
cargo test -p anicraft-smart-controller-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```
