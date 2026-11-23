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


## 7. Native M2 host contracts — 2026-11-23

The read-only host/core contracts are specified in [SMART_CONTROLLER_HOST_CONTRACTS.md](contracts/SMART_CONTROLLER_HOST_CONTRACTS.md), with the corresponding AniCraft API contract in [AniCraft's host contract](https://github.com/Mujshanan/anicraft/blob/main/docs/SMART_CONTROLLER_HOST_CONTRACT.md).

### Contract A — target identity and capability
- Geometry targets are addressed by document-scoped numeric `ElementId` + `PinId`; the adapter verifies the exact `warp.info` response.
- AniCraft's actual pin response key is `pins[].pin` (not `id`). The adapter validates the actual schema and distinguishes Supported, Unsupported, and Unavailable.
- Supported geometry pins remain preview/undo-only; no `warp.pin_move` command is issued.
- AniCraft currently has no stable document UUID. Durable target bindings remain blocked until a collision-safe document scope is host-owned.

### Contract B — deterministic input snapshots
- Bone inputs are sampled at an explicit zero-based frame, and the adapter rejects a `bone.info` response for a different frame.
- Keyed constrained angle and spring swing remain separate; inclusion of spring swing and wrapped-angle semantics are explicit policies.
- Snapshot values remain invocation-scoped; failures do not become zero values, and the core applies authored fallback policy.

The adapter implementation adds explicit frame pinning and host-schema conformance tests. Neutral capture, binding controls, persistence, target mutation, playback, and CCC-01.5 composition changes remain out of scope. The contract suite must pass CI and an AniCraft in-app smoke test before host conformance is claimed.
