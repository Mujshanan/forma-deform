# Bone Query

**Status:** First command-plugin implementation.

Bone Query is a read-only AniCraft command plugin. It queries the documented `bone.info` command at the current playhead frame and reports:

- Armature layer ID and name, whether springs are enabled, and whether the armature is mixed.
- Bone ID, name, parent bone, stage-space head/tail, keyed/constrained angle, limits, pin state, and bound-point count.
- Spring swing separately from the keyed/constrained angle. The reported displayed angle is their sum; this is informational and does not replace either source field.
- Optional per-shape deformation-point positions and bone weights.

The plugin does not modify bones, keyframes, weights, or the document.

## Parameters

| Parameter | Default | Purpose |
|---|---:|---|
| Include shape point weights | Off | Requests detailed point positions and influences from `bone.info` |
| Maximum points per shape | 30 | Limits point detail in the output report |

The query always uses the current playhead frame. Bone IDs and shape IDs are AniCraft's numeric host IDs, not array positions.

## Build

From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
```

The output is:

```
target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm
```

Install that WASM file through AniCraft's **Commands → Install Plug-in…** workflow.

## Host limitation

This is an on-demand command plugin, not a persistent bone observer. It reports the pose when invoked and does not automatically refresh during playback or scrubbing.
