# Smart Controller development plan

**Status:** Active plan, revised 2026-11-19  
**Host:** Mujshanan/AniCraft  
**V1 objective:** Slider mapping plus deterministic, on-demand preview.

## 1. Architecture boundary

- smart-controller-core owns host-independent Rust model, validation, normalization, endpoint-aware frame mapping, transform-value calculation, and preview output. It has no AniCraft, WASM, browser, or UI dependency.
- plugins/smart-controller is a thin AniCraft command-plugin adapter. It deserializes JSON parameters, validates them through the core, reads a driver bone through the verified bone.info command when manual input is absent, invokes pure preview calculation, and logs JSON output.

The verified SDK entry point is ac::plugin!(MANIFEST, run) with fn run(p: &serde_json::Value) -> Result<(), String>. No speculative prelude, context object, or plugin error type is used.

The former single-file core remains in legacy.rs to preserve the three-anchor binding API used by the Form Deform integration test. New logic is split into model.rs, validation.rs, mapping.rs, and preview.rs.

## 2. V1 scope — Slider + deterministic preview

- Read an evaluated driver angle on demand with bone.info, or accept manual_value as a first-class deterministic input.
- Normalize between two finite rotation endpoints. Reversed endpoints are valid and reverse the mapping direction.
- Map normalized input to an action frame with explicit endpoint semantics.
- Default to Exclusive [start, end), where the end frame is never selected because cyclic actions commonly duplicate the first pose at the end.
- Support Inclusive [start, end] when the last pose is distinct.
- Emit a JSON preview record through the plugin log.
- Do not change the timeline, modify bones, or write document data.

### Endpoint policy

For a non-degenerate frame interval:
- Exclusive: start + min(floor(normalized × span), span - 1).
- Inclusive: start + round(normalized × span).

Both clamp normalized input to [0, 1]. A degenerate mapping helper returns start; Slider validation rejects end <= start.

## 3. Deferred scope

- V2 — Transform: expose the core transform mapping through an adapter only after host transform read/write commands and semantics are verified.
- V3 — Presets: define versioned preset/binding persistence only after a supported storage boundary is established.
- V4 — MCP-driven interactive updates: investigate authenticated external-control updates. This is not a claim that the current command-plugin ABI can run continuously during playback.
- Continuous playback evaluation and durable document-owned bindings remain host-extension decisions unless the host contract changes.

## 4. Source layout

    crates/smart-controller-core/
      src/lib.rs          # public re-exports
      src/legacy.rs       # compatibility API for Form Deform contract
      src/model.rs        # Params, Mode, EndpointPolicy
      src/validation.rs   # mode-aware validation
      src/mapping.rs      # normalization and mapping
      src/preview.rs      # PreviewOutput and pure preview functions
      tests/mapping_tests.rs
      tests/validation_tests.rs
    plugins/smart-controller/src/lib.rs  # SDK entry point and thin adapter

## 5. Verification criteria

- cargo test -p smart-controller-core
- cargo test -p anicraft-smart-controller-plugin
- cargo test -p smart-controller-core -p anicraft-smart-controller-plugin
- cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin

Native tests cover endpoint policy, reversed angle ranges, mode-aware validation, explicit manual input, and the retained legacy cross-plugin math contract. An AniCraft in-app smoke test is still required to verify the installed plugin and real bone.info response.

## 6. Non-goals

Do not invent timeline or bone-write commands. Do not interpret guessed JSON document paths as an API. Do not silently substitute zero for a failed bone query. Do not claim live playback, durable binding storage, or in-app success before those capabilities are verified.
