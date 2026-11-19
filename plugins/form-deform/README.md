# Form Deform

**Status:** Directory reserved; implementation not started.

Form Deform will own shape-deformation logic driven by bone changes. It will be developed as a separate plugin from Bone Query and Smart Controller.

Planned responsibilities:

- Inspect a target shape and its existing deformation/warp data.
- Define a bone-to-shape deformation relationship.
- Calculate shape deformation from bone pose data.
- Keep authored rest geometry separate from evaluated deformation.
- Preview and commit only through verified AniCraft commands and undo semantics.

The first milestone will establish a native, deterministic deformation core before adding a WASM command adapter. Continuous playback/scrubbing evaluation is not available through the current command-plugin contract and must not be simulated by repeatedly running a command.
