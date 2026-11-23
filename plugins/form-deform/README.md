# Form Deform

**Status:** Legacy on-demand command adapter; retirement candidate after native FDM host integration reaches the documented parity gate.

Form Deform drives one AniCraft Asset Warp pin from one armature bone angle. It is a focused deformation tool, separate from the general-purpose Smart Controller.

## Workflow

1. Create an armature and pose its controller bone.
2. Add an Asset Warp mesh and pin to the target shape.
3. Note the bone, element, and pin IDs.
4. Set the authored neutral bone angle and the pin's neutral stage-space position.
5. Set X/Y displacement gains in stage pixels per degree.
6. Run Form Deform with Apply pin position disabled to inspect the calculated target.
7. Enable Apply pin position to write the absolute pin position through the host's warp.pin_move command. The host command is undoable.

The calculation is non-accumulating: every run derives an absolute target from the authored neutral pin position, not from the pin's current displayed position. Configure the neutral position and gains as part of the deformation setup.

## Spring policy

By default, the input uses the constrained keyed bone angle. Include spring swing explicitly adds the separately reported spring swing. Head/tail coordinates are not used as pose inputs.

## Limits

- One bone drives one warp pin per invocation.
- The host is queried at the current playhead.
- This is an on-demand command, not a live playback/scrubbing evaluator.
- The plugin does not create warp meshes or pins; use AniCraft's Asset Warp tools first.
- Applying the result changes the document through warp.pin_move; use the explicit Apply option and normal undo workflow.
- No durable plugin-owned binding data is stored in the AniCraft document.

## Build

```sh
cargo test -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
```
