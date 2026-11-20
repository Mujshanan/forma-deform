# AniCraft Smart Controller development plan

**Status:** Unified plugin architecture, implementation in progress  
**Host:** Mujshanan/AniCraft  
**Plugin identity:** AniCraft Smart Controller (`org.mujshanan.smart-controller`)

## 1. Product scope

One installed AniCraft command plugin exposes distinct controller modes:

- **Slider:** maps a driver bone's evaluated angle to an action-frame preview.
- **Deform:** calculates an absolute Asset Warp pin target from an authored neutral angle/position and per-degree gains, with optional spring swing and explicit undoable Apply.

These are mode-specific workflows inside one plugin, not separate installed plugins. Each mode has a separate typed parameter DTO and dispatch path.

## 2. Architecture boundaries

- `smart-controller-core` owns host-independent Slider mapping, validation, normalization, endpoint handling, and preview calculations.
- `form-deform-core` owns host-independent deformation domain types, bone transforms, weighted mesh deformation, pin references, validation, and mathematical evaluation.
- `plugins/smart-controller` composes both pure cores and owns the AniCraft manifest, ABI entry point, host calls, DTOs, and host/core translation.
- `anicraft-plugin-sdk` owns generic JSON/WASM ABI transport and host communication. It contains no Smart Controller policy or deformation math.

Runtime data flow is calculation in the selected pure core → plugin adapter serialization/ABI through the SDK → AniCraft plugin runtime. Cargo dependency direction remains plugin → each core and plugin → SDK. Neither core depends on the SDK, host, or the other core.

The former standalone Form Deform command adapter has been retired. Its pure calculation crate remains independent and is now consumed by the unified Smart Controller plugin.

## 3. Slider mode

- Read an evaluated driver angle on demand with `bone.info`, or accept `manual_value` as a deterministic input.
- Normalize between two finite rotation endpoints. Reversed endpoints are valid.
- Map normalized input to an action frame with explicit endpoint semantics.
- Default to Exclusive [start, end), where the end frame is not selected.
- Support Inclusive [start, end] when the last pose is distinct.
- Emit a JSON preview record through the plugin log.
- Do not change the timeline, modify bones, or write document data.

Transform mode remains deferred until verified host transform read/write commands and semantics are available.

## 4. Deform mode

- Read the selected bone's evaluated angle and optional spring swing from `bone.info`.
- Verify the selected element and pin through `warp.info`.
- Evaluate the absolute pin target using `form-deform-core::evaluate_deformation`.
- Keep spring swing opt-in; requesting unavailable swing is an error.
- Default to calculation-only. Apply explicitly sends `warp.pin_move`, producing an undoable document edit.
- Always calculate from authored neutral values; never accumulate from the currently deformed pin position.

The pure `deform_mesh` linear-blend-skinning API is groundwork only. Current host responses do not expose complete bind mesh data, skin weights, or evaluated bone world transforms, so the adapter must not fabricate inputs or claim to run that solver on live AniCraft geometry.

## 5. Host limitations and non-goals

- Both modes are on-demand command executions, not playback/scrubbing evaluators.
- No verified command currently sets a nested symbol instance's displayed frame.
- Bindings are not durably persisted in the AniCraft document.
- Do not invent timeline, bone-write, geometry-read, or metadata-persistence APIs.
- Do not silently replace failed host reads with zero or other fallback values.

## 6. Source layout

```text
crates/
  smart-controller-core/   # Slider mapping and three-anchor binding math
  form-deform-core/        # pure bone/pin/mesh deformation math
  anicraft-plugin-sdk/     # generic host ABI and transport
plugins/
  bone-query/              # read-only inspection command
  smart-controller/        # unified Slider + Deform command adapter
```

## 7. Verification

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin`
- `cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin`

Native tests cover each pure core and the unified adapter's mode DTOs, host response decoding, mode-specific field isolation, and command serialization. AniCraft in-app smoke testing is still required for real host behavior.
