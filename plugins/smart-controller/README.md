# Smart Controller

**Adapter status:** Transitional ABI v1 command adapter. The active `run()` entry point remains the V1 Slider preview command. The Rust core and additional adapter modules contain broader capabilities, but those foundations are not a complete native AniCraft workflow.

The host-independent Rust core owns controller models, validation, normalization, mapping, migration, and deterministic evaluation. The adapter translates selected host inputs and read-only target inspection into core types.

## V1 command workflow

1. Select the driver layer/armature and driver bone name.
2. Select the action layer and configure its start/end frame range.
3. Set minimum and maximum driver rotation angles.
4. Choose Exclusive (default) or Inclusive endpoint policy.
5. Run using the host bone input, or provide `manual_value` for deterministic tests.
6. Inspect normalized input and calculated output frame in the plugin log.

Manual input overrides the host query. A missing or malformed bone response is an error; it is never silently replaced with zero.

## Endpoint policy

- Exclusive maps [start, end). The end frame is reserved as the wrap point and is never selected; useful when the last frame duplicates the first pose.
- Inclusive maps [start, end]. Both endpoint frames can be selected when the last pose is distinct.
- Degenerate mapping helper intervals return start, although Slider validation requires end > start.
- Rotation endpoints may be reversed; +45° to -45° maps +45° to normalized 0 and -45° to normalized 1.

## Boundaries of the active V1 command

- V1 is an on-demand preview command, not a playback/scrubbing evaluator.
- The command does not set the AniCraft timeline or write bone properties.
- `action_layer_id` is validated as configuration metadata; no host action/timeline API is assumed.
- Transform calculations are pure-core groundwork; the active V1 command rejects Transform mode.
- Bindings are not durably persisted in the AniCraft document.
- The active V1 command does not apply geometry pins or generic target properties.

## Additional adapter foundations

The adapter has isolated modules for:

- Parsing the verified `bone.info` response by numeric bone ID, with explicit spring-swing and angle-wrapping choices.
- Assessing target capability and verifying exact geometry pin identity through read-only `warp.info`.
- Delegating legacy BindingSet / Params migration to `smart-controller-core` while retaining its migration report.
- Evaluating a canonical document through an injected mapper and producing deterministic, read-only target preview plans.

The canonical preview path accepts manual values keyed by canonical InputId and an explicit bone-ID → AniCraft layer-ID map. Its input provider supports Manual and BoneTransform Rotation inputs. The evaluator resolves ControllerOutput dependencies and performs the independent composition pass using an injected ControllerMapper. The target preview planner returns composed values paired with canonical target mappings and capability assessments in TargetId order.

These helpers are descriptive/read-only foundations. They do not issue target setters or mutate the document. Geometry pin application still requires a host preview/undo transaction; symbol-frame and generic property setters remain unverified. The concrete Pose2D/joystick mapper is a separate core prerequisite and must be supplied through the mapper boundary.

## Compatibility and migration

The core's `legacy.rs` and `migration.rs` modules preserve older binding formats and explicit migration/reverse-export behavior. They are not the retired adapter runtime and must not be deleted as part of adapter retirement without a separate compatibility decision and migration test plan.

## Build and test

```sh
cargo test -p smart-controller-core
cargo test -p anicraft-smart-controller-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

See [Legacy Adapter Status and Retirement Plan](../../docs/LEGACY_ADAPTER_RETIREMENT.md) for the adapter's eventual replacement gates.
