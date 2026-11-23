# Bone Query

**Status:** Retained read-only development and diagnostic command adapter.

Bone Query calls AniCraft's `bone.info` command and reports the armature data exposed by Mujshanan/anicraft:main. It does not modify the document. It is a diagnostic probe, not a production dependency of the deformation engine.

## Reported data

- **Query context:** evaluated frame and optional armature-layer filter.
- **Armature:** layer ID, name, pose-keyframe count, root element IDs, member element IDs, mixed-armature flag, spring-enabled flag, and first shape ID.
- **Bone identity and hierarchy:** bone ID/name, parent/child endpoint IDs (the child may be an internal joint-node ID for a bone inside a shape), optional parent-bone ID, and pin state.
- **Pose:** joint angle in degrees, optional rotation limits, and spring swing reported separately. The informational angle-plus-swing value is their sum; it does not replace either source value.
- **Geometry:** head and tail coordinates for the keyed pose at the requested frame, plus joint slide values along the parent instance's local X/Y axes. Spring swing is reported separately; these endpoints are not spring-offset geometry. Missing geometry is shown as not evaluated rather than as zero.
- **Joint translation:** X/Y enabled state and translation limits, or free/disabled state.
- **Spring:** optional strength and damping percentages.
- **Shape armatures:** shape IDs, their bone IDs, skin-point counts, and per-bone bound-point counts.
- **Optional skin details:** point positions, bone influence weights, and whether each binding is manual.

The report distinguishes from/to endpoint IDs from parent-bone IDs. For a bone inside a shape, its child endpoint is an internal joint-node ID rather than an element ID. Roots and members are host IDs, not array positions.

## Parameters

| Parameter | Default | Purpose |
|---|---|---|
| Query a specific frame | Off | Enables the explicit frame override |
| Frame | 0 | 0-based frame; used only when Query a specific frame is enabled |
| Filter by armature layer | Off | Enables the layer filter |
| Armature layer ID | 0 | Host layer ID; used only when Filter by armature layer is enabled |
| Include shape point weights | Off | Requests detailed point positions and influences |
| Maximum points per shape | 30 | Limits displayed point details per shape (1–100) |

Both filters are opt-in. By default, the plugin queries the current playhead and all armatures. The explicit toggles matter because AniCraft supplies a default value for every declared manifest parameter; an unset integer parameter would otherwise resolve to 0 and unintentionally force frame/layer 0.

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

Install the WASM file through AniCraft's **Commands → Install Plug-in…** workflow only for a development diagnostic session.

## Host limitations

- This is an on-demand command plugin, not a persistent bone observer. It does not automatically refresh during playback or scrubbing.
- Command plugins run in a fresh sandboxed instance for each invocation.
- Shape-point details are requested from the host only when enabled; output is capped independently for each shape.
- The plugin depends on the `bone.info` fields implemented in the Mujshanan AniCraft fork. It should not be assumed compatible with upstream Storytold AniCraft revisions that expose a different response schema.

## Verification boundary

The repository CI workflow is configured to run native workspace tests and build this adapter for `wasm32-unknown-unknown`. Those automated checks do not prove that the plugin has been installed or smoke-tested in AniCraft. Record a successful in-app smoke test separately before describing host integration as verified.
