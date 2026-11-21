# Smart Controller

**V1 status:** Slider mapping and deterministic, on-demand preview.

The host-independent Rust core performs validation, normalization, endpoint-aware frame mapping, and preview calculation. The AniCraft adapter only resolves the driver input through the verified bone.info command, calls the core, and logs JSON output.

## V1 workflow

1. Select the driver layer/armature and driver bone name.
2. Select the action layer and configure its start/end frame range.
3. Set minimum and maximum driver rotation angles.
4. Choose Exclusive (default) or Inclusive endpoint policy.
5. Run using the host bone input, or provide manual_value for deterministic tests.
6. Inspect normalized input and calculated output frame in the plugin log.

Manual input overrides the host query. A missing or malformed bone response is an error; it is never silently replaced with zero.

## Endpoint policy

- Exclusive maps [start, end). The end frame is reserved as the wrap point and is never selected; useful when the last frame duplicates the first pose.
- Inclusive maps [start, end]. Both endpoint frames can be selected when the last pose is distinct.
- Degenerate mapping helper intervals return start, although Slider validation requires end > start.
- Rotation endpoints may be reversed; +45° to -45° maps +45° to normalized 0 and -45° to normalized 1.

## Boundaries

- V1 is an on-demand preview command, not a playback/scrubbing evaluator.
- The plugin does not set the AniCraft timeline or write bone properties; no unverified command IDs are used.
- action_layer_id is validated as configuration metadata; no host action/timeline API is assumed.
- Transform calculations are pure-core groundwork for V2; the V1 adapter rejects Transform mode.
- Bindings are not durably persisted in the AniCraft document.

## Build and test

    cargo test -p smart-controller-core
    cargo test -p anicraft-smart-controller-plugin
    cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin


## Unified adapter foundations (Phase 5)

The adapter now has isolated modules for:

- Parsing the verified `bone.info` response by numeric bone ID, with explicit spring-swing and angle-wrapping choices.
- Assessing target capability and verifying exact geometry pin identity through read-only `warp.info`.
- Delegating legacy BindingSet / Params migration to `smart-controller-core` while retaining its migration report.

These modules are foundations only. The existing V1 Slider command still uses its original preview path. Geometry pin mutation is not issued by the new adapter; the documented `warp.pin_move` path requires a host preview/undo transaction. Symbol-frame display, generic property application, playback evaluation and durable document persistence remain unverified or unsupported.
