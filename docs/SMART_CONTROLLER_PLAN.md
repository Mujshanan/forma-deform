# AniCraft Smart Controller development plan

**Status:** V1 command modes and runtime contract implemented; adapter-boundary test seam added; CI validation pending  
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

**V1 boundary decision:** the three-anchor `ControllerBinding` / `BindingSet` API in `smart-controller-core::legacy` is retained as an experimental, host-independent core model. The plugin adapter does not call it. It is not the runtime representation of Slider or Deform, is not durably persisted in AniCraft, and is explicitly excluded from V1 acceptance. Do not describe its CRUD/JSON helpers as plugin binding persistence.

## 3. V1 runtime contract

### Slider

The manifest accepts `driver_layer_id`, `driver_bone_name`, `action_layer_id`, `min_rotation_deg`, `max_rotation_deg`, `action_start_frame`, `action_end_frame`, `endpoint_policy`, and optional `manual_value`.

- If `manual_value` is supplied, it is the driver input and the adapter does not query the host.
- Otherwise the adapter calls `bone.info` with the selected driver layer and resolves the named bone's evaluated `angle`. Missing/malformed results fail explicitly.
- The pure Slider core validates finite, distinct rotation endpoints and an increasing frame range, normalizes the angle, and maps it to a frame. Reversed rotation endpoints are supported.
- `endpoint_policy` defaults to Exclusive [start, end); Inclusive [start, end] is also supported.
- The adapter logs the serialized preview result. It issues no timeline, bone-write, or document-mutation command.
- `action_layer_id` is currently required by parameter validation but is not queried from the host or used to mutate/select an action. The frame preview is calculated solely from the supplied start/end frame values. Do not claim that the action layer is verified or driven.

### Deform

The manifest accepts `bone_id`, `element_id`, `pin_id`, `neutral_angle`, `neutral_x`, `neutral_y`, `gain_x`, and `gain_y`; `include_spring_swing` and `apply` default to false.

1. Query `bone.info`, decode the typed armature/bone response, and resolve the requested bone ID and keyed angle. Spring swing is optional.
2. Query `warp.info` for the requested element; require an Asset Warp mesh and the requested existing pin.
3. Evaluate the absolute target from the authored neutral position and angle plus the per-degree gain. Spring swing contributes only when explicitly enabled and present.
4. Log the effective angle and absolute stage-pixel target.
5. If `apply` is false, perform calculation only. If true, send `warp.pin_move` with exactly the selected element, pin, and calculated X/Y target. The host edit is undoable.

All host reads and writes stay in the adapter. Missing IDs, malformed required DTO fields, non-warped elements, absent pins, and invalid core inputs return errors. Repeated evaluation starts from authored neutral values, not the currently displayed pin position.

### Mode dispatch

The default mode is `slider`. `deform` selects the Deform workflow. `transform` is explicitly rejected by the adapter until verified host transform read/write APIs exist; unknown modes are errors.

## 4. Experimental ControllerBinding core API

`smart-controller-core::legacy` contains a separate three-anchor model with an input range, easing curve, absolute/relative evaluation, and SymbolFrame/GeometryDeform targets. `BindingSet` provides in-memory CRUD helpers and versioned JSON serialization.

This model is retained for future evaluation only:
- the V1 plugin adapter does not call it;
- its modes are not the plugin's Slider/Deform mode dispatcher;
- its JSON serialization is not AniCraft document persistence;
- it does not cause timeline or geometry host commands to run.

It is excluded from V1 acceptance. Reconsider it only through a separate product/API decision and after the required host command and persistence contracts are verified.

## 5. Host limitations and non-goals

- Both modes are on-demand command executions, not playback/scrubbing evaluators.
- No verified command currently sets a nested symbol instance's displayed frame.
- Bindings are not durably persisted in the AniCraft document.
- Do not invent timeline, bone-write, geometry-read, or metadata-persistence APIs.
- Do not silently replace failed host reads with zero or other fallback values.

## 6. Source layout

```text
crates/
  smart-controller-core/   # Slider mapping + experimental, non-runtime three-anchor model
  form-deform-core/        # pure bone/pin/mesh deformation math + diagnostics
  forma-document-core/     # authoritative project document, geometry, and timeline model
  anicraft-plugin-sdk/     # generic host ABI and transport
plugins/
  bone-query/              # read-only inspection command
  smart-controller/        # unified Slider + Deform command adapter
```

## 7. Verification

- `cargo test --workspace`
- `cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin`
- `cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin`

Adapter tests execute the same `run_with_host` mode dispatcher used by production `run()`, with a fake host that captures commands and logs. They cover Slider manual/bone inputs, endpoint and reversed-range behavior, Deform preview/apply, exact pin-move payload, optional spring swing, malformed/missing host data, command failures, and repeat evaluation. SDK tests cover malformed JSON and oversized plugin parameters. AniCraft in-app smoke testing is still required for real host behavior.
