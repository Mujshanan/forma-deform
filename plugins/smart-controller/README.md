# Smart Controller

**Status:** First on-demand command-plugin implementation.

Smart Controller is the general-purpose input-to-target mapper. It uses the shared smart-controller-core crate for validation, neutral-centered normalization, easing, and three-anchor interpolation.

## V1 workflow

1. Create an AniCraft armature and note a controller bone ID.
2. Add an Asset Warp mesh and pin to the target shape; note the element and pin IDs.
3. Set the input minimum, neutral, and maximum.
4. Set negative, neutral, and positive X/Y output anchors.
5. Set the neutral pin base position in stage coordinates.
6. Run with Apply disabled to inspect the calculated output.
7. Enable Apply to write the absolute pin position through warp.pin_move (undoable).

The default Relative Delta mode subtracts the neutral output anchor before adding it to the neutral pin base. Absolute Pose mode uses the interpolated anchor as an offset from that base. Both modes are deterministic and do not accumulate from the pin's current position.

The input can come from the live bone angle at the playhead or an explicit angle for testing. Spring swing is excluded by default and can be included explicitly. Linear and Smoothstep easing are supported.

## Boundaries

- One binding is configured per invocation; this v1 adapter does not yet persist a binding set in the document.
- It can calculate geometry targets and apply a verified Asset Warp pin position.
- The shared core can also calculate SymbolFrame targets, but the current AniCraft command API has no verified command for visually setting a nested symbol instance's displayed frame. The adapter therefore does not pretend to apply that target.
- It is an on-demand command, not a playback/scrubbing evaluator.
- Apply is an explicit document edit; it is not a non-destructive live preview. Use AniCraft undo to revert it.
- No automatic binding coordination between Form Deform and Smart Controller is claimed yet; both use the same verified warp-pin command boundary and remain separately callable.

## Build

```sh
cargo test -p anicraft-smart-controller-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```
