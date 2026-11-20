# Smart Controller development plan

**Status:** V1 implemented; V4 MCP feasibility prototype in progress, revised 2026-11-20  
**Host:** Mujshanan/AniCraft  
**V1 objective:** Slider mapping plus deterministic, on-demand preview.

## 1. Architecture boundary

- smart-controller-core owns host-independent Rust model, validation, normalization, endpoint-aware frame mapping, transform-value calculation, and preview output. It has no AniCraft, WASM, browser, or UI dependency.
- plugins/smart-controller is a thin AniCraft command-plugin adapter. It deserializes JSON parameters, validates them through the core, reads a driver bone through the verified bone.info command when manual input is absent, invokes pure preview calculation, and logs JSON output.
- V4 is an external Python feasibility tool using AniCraft's existing MCP stdio bridge. It does not expand the plugin ABI or modify AniCraft source.

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

## 3. V4 scope — MCP-driven interactive global-playhead prototype

V4 tests whether an external process can read an artist-authored driver-bone angle and move AniCraft's global scene playhead to the mapped frame through the live MCP bridge.

Verified command path:
- anicraft_commands discovers the commands supported by the running host.
- bone.info {layer, frame} reads the evaluated keyed angle for a stable numeric bone ID.
- timeline.goto {frame} moves the current scene's global playhead.
- anicraft_document reports the current playhead; it is not the source of the evaluated bone angle.

The prototype lives at tools/v4_mcp_slider.py. It performs MCP stdio initialization, command discovery, bone read, V1 mapping, playhead update, and CSV latency logging. It supports one-shot operation and watch mode. Watch mode samples only while the UI is at a configured fixed source frame and debounces two stable angle samples before issuing a goto.

**Important limitation:** timeline.goto does not set an individual layer's frame or a nested graphic symbol instance's local displayed frame. The scene has one global playhead. After a mapped goto, the artist must return to the fixed source frame to adjust the driver for another sample. This is quasi-interactive global timeline scrubbing, not a seamless Rigby2D-style nested-symbol Slider and not frame-synchronized playback.

The prototype deliberately does not call bone.pose: that command writes an authored keyed pose and would turn the experiment into a script-driven pose editor rather than reading the artist's driver input. It also does not write document-owned controller bindings.

### V4 acceptance status

- [x] MCP stdio client and live bridge invocation implemented.
- [x] Runtime command discovery for bone.info and timeline.goto implemented.
- [x] V1-compatible Slider mapping and CSV latency logging implemented.
- [x] Local mapping, parsing, and protocol-simulator checks.
- [ ] Live registry discovery against the user's running AniCraft build.
- [ ] At least 10 consecutive live bone-to-frame updates.
- [ ] Median live round-trip latency below 500 ms.
- [ ] Live UI demonstration or recording.

See docs/V4_MCP_FINDINGS.md for the evidence, limitations, run instructions, and pending live acceptance criteria. Do not report V4 as fully accepted until the live checks are recorded.

## 4. Deferred scope

- V2 — Transform: expose the core transform mapping through an adapter only after host transform read/write commands and semantics are verified.
- V3 — Presets: define versioned preset/binding persistence only after a supported storage boundary is established.
- Native continuous playback evaluation and durable document-owned bindings remain host-extension decisions.
- A true nested-symbol Slider requires a host command to set the instance's local displayed frame or a reviewed native evaluation hook.

## 5. Source layout

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
    tools/v4_mcp_slider.py               # external MCP feasibility prototype
    tests/test_v4_mcp_slider.py           # V4 mapping and response-contract tests
    docs/V4_MCP_FINDINGS.md               # evidence and live-test findings

## 6. Verification

Rust:
- cargo test -p smart-controller-core
- cargo test -p anicraft-smart-controller-plugin
- cargo test -p smart-controller-core -p anicraft-smart-controller-plugin
- cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin

V4 Python:
- python -m py_compile tools/v4_mcp_slider.py
- python -m unittest discover -s tests -p "test_v4_mcp_slider.py" -v

The V4 checks do not replace live AniCraft bridge testing.

## 7. Non-goals

Do not invent timeline or bone-write commands. Do not interpret guessed JSON document paths as an API. Do not silently substitute zero for a failed bone query. Do not claim live playback, durable binding storage, or in-app success before those capabilities are verified.
