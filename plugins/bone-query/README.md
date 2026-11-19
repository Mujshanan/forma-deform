# Bone Query

**Status:** Read-only command-plugin implementation; host integration still needs testing.

Bone Query calls AniCraft's bone.info command and reports the armature data exposed by Mujshanan/anicraft:main. It does not modify the document.

## Reported data

- **Query context:** evaluated frame and optional armature-layer filter.
- **Armature:** layer ID, name, pose-keyframe count, root instance IDs, member element IDs, mixed-armature flag, spring-enabled flag, and first shape ID.
- **Bone identity and hierarchy:** bone ID/name, parent and child element IDs, optional parent-bone ID, and pin state.
- **Pose:** joint angle in degrees, optional rotation limits, and spring swing reported separately. The informational angle-plus-swing value is their sum; it does not replace either source value.
- **Geometry:** evaluated head and tail coordinates, plus joint slide coordinates. Missing geometry is shown as not evaluated rather than as zero.
- **Joint translation:** X/Y enabled state and translation limits, or free/disabled state.
- **Spring:** optional strength and damping percentages.
- **Shape armatures:** shape IDs, their bone IDs, skin-point counts, and per-bone bound-point counts.
- **Optional skin details:** point positions, bone influence weights, and whether each binding is manual.

The report distinguishes from/to element IDs from parent-bone IDs. Roots and members are host IDs, not array positions.

## Parameters

| Parameter | Default | Purpose |
|---|---|---|
| Frame | Current playhead | Optional 0-based frame passed to bone.info |
| Armature layer ID | All armatures | Optional layer filter passed to bone.info |
| Include shape point weights | Off | Requests detailed point positions and influences |
| Maximum points per shape | 30 | Limits displayed point details per shape (1–100) |

The plugin uses the document playhead when Frame is left blank. The host remains responsible for validating and clamping the requested frame.

## Build

~~~sh
rustup target add wasm32-unknown-unknown
cargo test -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
~~~

The output is:

~~~
target/wasm32-unknown-unknown/release/anicraft_bone_query_plugin.wasm
~~~

Install the WASM file through AniCraft's **Commands → Install Plug-in…** workflow.

## Host limitations

- This is an on-demand command plugin, not a persistent bone observer. It does not automatically refresh during playback or scrubbing.
- Command plugins run in a fresh sandboxed instance for each invocation.
- Shape-point details are requested from the host only when enabled; output is capped independently for each shape.
- The plugin depends on the bone.info fields currently implemented in the Mujshanan AniCraft fork. It should not be assumed compatible with upstream Storytold AniCraft revisions that expose a different response schema.

## Verification status

The implementation and fixture tests have been updated against the fork's current cmds_bone.rs response. Compilation, native test execution, WASM compilation, and an in-app run have **not yet been verified**.
