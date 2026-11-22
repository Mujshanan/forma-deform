# FD-M2 — AniCraft plugin preview integration

**Implementation branch:** `feat/fd-m2-plugin-preview`  
**Scope:** on-demand command-plugin preview only.

## Implemented

- The Smart Controller command plugin now has a canonical-preview path selected by a non-empty `canonical_document_json` parameter. The existing legacy Slider preview remains available when that parameter is empty.
- Canonical documents are deserialized and validated before host input collection.
- Manual inputs are supplied as tagged `TypedValue` JSON keyed by stable `InputId`.
- Bone inputs require an explicit canonical bone ID → AniCraft layer ID map. No selected-bone, layer-index, or array-position inference is used.
- The input provider queries the documented read-only `bone.info` command at the current playhead, caches both successful and failed responses per layer for the invocation, and parses bone rotations by numeric host bone ID.
- Host degrees are converted to canonical radians. Inclusion of spring swing and wrapped/unwrapped angle semantics are explicit command parameters.
- The canonical mapper, dependency evaluator, and independent composition pass produce a target preview plan.
- Geometry targets are checked against the exact element and pin IDs returned by read-only `warp.info`.
- The plugin logs target values, target capability, host verification, unresolved outputs, and evaluator diagnostics.
- No target mutation, keyframe write, timeline command, or playback evaluation is performed.

## Explicit limitations

- Only Manual and BoneTransform Rotation inputs have host providers. LegacyScalar and Translation/Scale fail closed.
- PoseSet1D runtime pose records are not currently provided to the plugin; no durable pose store is introduced.
- JoystickFivePose and multi-input mappings remain unsupported by the one-dimensional mapper.
- Geometry pin mutation is not invoked. The documented `warp.pin_move` command remains outside this read-only milestone because it must run through AniCraft's preview/undo transaction.
- Property, FrameRange, and SymbolFrame outputs are calculated and reported only; no generic property setter or symbol-frame setter is claimed.
- No live playback/scrubbing hook, persistent binding storage, native authoring UI, or Smart Controller/Form Deform composition is added.

## Verification gates

Run from the repository root:

~~~sh
cargo fmt --all -- --check
cargo test -p smart-controller-core
cargo test -p anicraft-smart-controller-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
~~~

Then install the built WASM in the supported AniCraft fork and verify:

1. A valid canonical document with a Manual input produces the same preview on repeated invocations.
2. A BoneTransform Rotation input reads the expected numeric bone ID from the explicitly mapped layer.
3. A missing bone, layer mapping, malformed response, or unsupported channel fails closed and is surfaced in diagnostics.
4. A GeometryDeform target is marked verified only when `warp.info` returns the exact element and pin IDs.
5. Repeated preview leaves the document, authored geometry, keyframes, and timeline unchanged.

M2 is not complete until the native tests, WASM build, and in-app checks above have been run successfully. The repository changes establish the integration path but do not claim those runtime checks have already passed.
