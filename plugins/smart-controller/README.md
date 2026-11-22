# Smart Controller

**V1 status:** canonical, on-demand host preview and explicit single- or multi-target current-playhead apply, plus the original Slider preview. The canonical path reads verified AniCraft inputs and produces a deterministic target preview plan. It does not run during playback/scrubbing.

## Native authoring dialog (FD-M3 first slice)

AniCraft renders command plug-in settings as a native dialog. The Smart Controller now offers two workflows:

- **Geometry Controller** (default): author one Manual or Bone Rotation input, a three-point range/easing, and one GeometryDeform target with negative/neutral/positive vector anchors. The plug-in assembles and validates the canonical document internally, then runs the read-only M2 preview.
- **Legacy Slider**: retains the compact legacy frame-mapping preview. The native settings schema has no free-form text parameter, so the legacy bone-name field is not user-editable in this dialog; the adapter uses the documented default bone name, Head.

The dialog uses only AniCraft-supported parameter types (number, int, bool, choice, color), stays within the host's 32-parameter limit, and does not ask users to paste canonical JSON. Settings are ephemeral; they are not saved as durable bindings. Cancel must leave the document unchanged. The M4 core payload now defines a versioned storage envelope, but it is not connected to AniCraft document storage yet.

The first slice intentionally supports one input, one controller, and one geometry target. It is not yet a persistent docked inspector or a multi-controller authoring workspace.

## Persistent bindings (FD-M4 in progress)

The host-independent core now exposes `PersistentBindingDocument` with strict schema versioning, canonical graph validation, atomic controller upsert/removal, and collection revision tracking. This is a payload contract only. The current command plug-in cannot write document metadata or files; durable save/load and undo/redo require a reviewed AniCraft host extension. See [FD-M4 specification](../../docs/FD-M4-PERSISTENT-BINDINGS.md).

## Target application (FD-M5 first slice)

The Smart Controller now supports an explicit **Target action** choice:

- **Preview** (default) remains read-only.
- **Apply current frame** re-queries `warp.info`, verifies the exact element and pin IDs, then issues one `warp.pin_move` command for the composed Vector2 output.
- **Apply all current frame** requires 1–512 resolved, host-verified GeometryDeform targets with unique `(element, pin)` identities. It preflights every target and payload before issuing any writes, then applies the prepared `warp.pin_move` calls inside AniCraft's enclosing `plugin.run` transaction.
- A preflight failure issues no writes. A commit-time command failure propagates so AniCraft can roll back the complete run. A successful batch is one undo step.

**Bake range** evaluates each requested frame using frame-specific bone inputs, verifies and prepares every pin write before mutation, then commits the entire range inside one `plugin.run` transaction. The inclusive frame range is limited to 512 frames and 512 total target writes. Manual inputs remain constant across the range. The proposed `warp.pin_move_batch` host command remains an optional lower-overhead API, not a prerequisite for this plug-in-side batch. See [FD-M5 design](../../docs/FD-M5-DEFORMATION-INTEGRATION.md).

## Canonical preview (FD-M2)

Set `canonical_document_json` to a JSON-encoded canonical Smart Controller document (schema `smart-controller`, version `1`). The plugin then:

1. Validates the canonical document before querying the host.
2. Collects only external inputs referenced by enabled controllers.
3. Uses the supplied `bone_layers_json` map to resolve canonical bone IDs to AniCraft armature/layer IDs. It never guesses a layer from selection or array position.
4. Queries the read-only `bone.info` command once per required layer for the current playhead.
5. Parses bone rotation by stable numeric bone ID, converting host degrees to canonical radians. Spring swing and wrapped-angle semantics are explicit parameters.
6. Evaluates the canonical controller graph with `CanonicalControllerMapper`, runs the independent composition pass, and builds a target preview.
7. For geometry targets only, queries read-only `warp.info` and verifies the exact element and pin IDs.
8. Logs target values, capability boundaries, unresolved targets, and evaluator diagnostics.

No target setter is called. The output is a preview plan, not a rendered stage overlay and not an apply/bake operation.

### Command parameters

| Parameter | Default | Purpose |
|---|---|---|
| `canonical_document_json` | empty | Versioned canonical document JSON; non-empty enables the canonical path |
| `manual_values_json` | `{}` | JSON object mapping InputId to serialized TypedValue |
| `bone_layers_json` | `{}` | Explicit JSON object mapping canonical bone ID strings to numeric AniCraft layer IDs |
| `bone_angle_policy` | `keyed_pose_only` | Either `keyed_pose_only` or `keyed_pose_plus_spring_swing` |
| `angle_semantics` | `unwrapped` | Either `wrapped` or `unwrapped` |

Manual values use the canonical tagged value representation, for example:

~~~json
{"jaw-driver":{"kind":"scalar","value":0.5}}
~~~

A bone rotation input uses its stable host bone ID in the document and requires an explicit layer mapping:

~~~json
{"42":7}
~~~

### Minimal canonical document example

This example maps a bone's rotation to a calculated property value. The property target is calculation-only: the plugin reports the value but does not call an unverified generic property setter.

~~~json
{
  "schema": {"name": "smart-controller", "version": 1},
  "inputs": [{
    "id": "head-rotation",
    "source": {"kind": "bone_transform", "bone_id": "42", "channel": "rotation"},
    "dimension": "angle",
    "fallback": {"kind": "reject"}
  }],
  "controllers": [{
    "id": "head-controller",
    "enabled": true,
    "inputs": ["head-rotation"],
    "dependencies": [],
    "mapping": {"kind": "range_1d", "minimum": -0.7853981634, "neutral": 0.0, "maximum": 0.7853981634},
    "bindings": [{
      "id": "mouth-width-output",
      "target_id": "property:mouth:width",
      "target": {"kind": "property", "target_id": "mouth", "property": "width", "scale": 1.0, "additive": false},
      "mode": "absolute_pose",
      "base_value": null
    }],
    "parent_scoped": false
  }]
}
~~~

For this example, set `bone_layers_json` to `{"42":7}` if bone 42 belongs to AniCraft layer 7. The document and both maps are supplied to the plugin command as strings.

### Input and target boundaries

- Supported host input: `Manual` and `BoneTransform` rotation.
- Unsupported host inputs fail closed: `LegacyScalar`, translation, and scale require a future explicit provider.
- Controller-output inputs are resolved by the core dependency graph and are not queried from AniCraft.
- Bone IDs and layer IDs are numeric host identities; mappings are explicit.
- Bone angle is read at the current playhead. This is an on-demand command, not a playback observer.
- GeometryDeform targets are verified through `warp.info`; `warp.pin_move` is called only for explicit current-frame apply actions.
- SymbolFrame, FrameRange, and Property targets are calculated only because a matching host application command has not been verified.
- PoseSet1D runtime pose records are not supplied by the current plugin invocation. Durable pose storage is not available in the documented AniCraft extension boundary.
- JoystickFivePose and multi-input mapping remain unsupported by the one-dimensional mapper.
- Smart Controller composition remains separate from Form Deform pin evaluation.

## Legacy Slider preview

When `canonical_document_json` is empty, the plugin preserves the original compact Slider workflow:

1. Select the driver layer/armature and driver bone name.
2. Configure the action layer and frame range.
3. Set minimum and maximum driver rotation angles.
4. Choose Exclusive (default) or Inclusive endpoint policy.
5. Run with the host bone input, or provide `manual_value` for deterministic testing.
6. Inspect normalized input and calculated output frame in the plugin log.

Manual input overrides the host query. A missing or malformed bone response is an error; it is never silently replaced with zero.

## Endpoint policy

- Exclusive maps [start, end). The end frame is reserved as the wrap point and is never selected; useful when the last frame duplicates the first pose.
- Inclusive maps [start, end]. Both endpoint frames can be selected when the last pose is distinct.
- Degenerate mapping helper intervals return start, although Slider validation requires end > start.
- Rotation endpoints may be reversed.

## Build and test

~~~sh
rustup target add wasm32-unknown-unknown
cargo test -p smart-controller-core
cargo test -p anicraft-smart-controller-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
~~~

The output is:

~~~text
target/wasm32-unknown-unknown/release/anicraft_smart_controller_plugin.wasm
~~~

Install the WASM file through AniCraft's **Commands → Install Plug-in…** workflow.

## Verification status

Native test execution, WASM compilation, and an in-app run must pass before this milestone is marked complete. The plugin is intentionally on-demand; no live playback evaluator, durable binding persistence, generic property setter, or symbol-frame setter is claimed. The bounded timeline-range bake described above is implemented as an explicit command-plugin operation; GUI verification of keyed frames, undo/redo, Cancel, and rollback remains pending.
