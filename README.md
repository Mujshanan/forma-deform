# Forma Deform

A Rust deformation library and AniCraft Smart Controller plugin suite. The deformation core is host-independent; the AniCraft plugin is an adapter connecting selected calculations to AniCraft's command-plugin API.

## Feature maturity

| Area | Maturity | Scope |
|---|---|---|
| form-deform-core | Implemented and tested | Pure Rust 2D transforms, weighted mesh deformation, authored-angle-to-pin-target calculation, validation, and opt-in diagnostics |
| smart-controller-core | Implemented and tested | Slider validation and bone-angle-to-action-frame preview mapping |
| AniCraft Smart Controller | Experimental integration | On-demand Slider preview and Deform pin-target calculation; explicit Deform apply issues an undoable Asset Warp pin edit |
| Live deformation and playback | Not implemented | No continuous playback/scrubbing evaluation or live mesh-skinning integration |
| Persistence and authoring UI | Not implemented | Bindings are plugin parameters; arbitrary plugin metadata is not persisted in the AniCraft document |

Treat the AniCraft integration as experimental until the host exposes the data and lifecycle APIs needed for a complete animation controller. The pure cores can be reused independently.

## Supported geometry and shape types

The deformation core is topology-agnostic. It accepts 2D mesh vertices, each with one or more bone influences, plus optional pin records. It does not parse or create shape primitives.

- Supported input: 2D weighted mesh vertices and caller-supplied triangle topology for diagnostics.
- Pin records: caller-provided ID, vertex index, and target position.
- Not directly represented: rectangles, ellipses, Bézier paths, vector strokes, text, and arbitrary drawing objects.
- AniCraft integration: Deform mode targets an existing Asset Warp element and one of its existing pins. This is a host-level pin operation, not general-purpose live mesh deformation.

A closed loop may be represented by repeating the first vertex at the end of the vertex list. Vertex order and pin indices are preserved. The core does not automatically close, triangulate, or validate polygon winding.

## Bone and control-point semantics

### Bones

- A BonePose contains a caller-supplied ID and a BoneTransform.
- BoneTransform contains a 2D translation and rotation in degrees.
- A point is rotated about the coordinate origin, then translated: x' = tx + x cos(θ) − y sin(θ), y' = ty + x sin(θ) + y cos(θ).
- Each mesh vertex is transformed by every bone in its influence list. The positions are combined using linear-blend-skinning (LBS) weights.
- Weights must be finite, non-negative, and sum to 1 within 1e-6. Missing bones and malformed weights are errors.
- The core accepts already evaluated transforms. It does not calculate bone hierarchies, parent/child transforms, inverse-bind matrices, constraints, or spring simulation.

### Pins / control points

A MeshPin contains an ID, vertex_index, and target. The index refers to the corresponding input vertex. The core validates that the index exists and the target is finite, then returns the pin record unchanged.

**Important:** pins are currently references/metadata, not deformation constraints. LBS does not move vertices to satisfy pin targets or solve a constraint system. The separate evaluate_deformation function calculates an absolute target position from an authored neutral angle/position and per-degree gains. AniCraft Deform mode uses that result for an Asset Warp pin; it does not feed the mesh LBS result into AniCraft.

## Coordinate-system conventions

- Core Vec2 values are plain finite f64 coordinates. The core does not assign units, stage dimensions, origin location, or screen-axis direction.
- Bone rotations and angle inputs are in degrees. The transform formula above uses the conventional mathematical 2D rotation formula.
- AniCraft Deform parameters label neutral X/Y positions and gains in stage pixels and pixels per degree. The adapter passes the calculated target to AniCraft's warp.pin_move command without an additional axis flip or unit conversion.
- Stage origin and positive-Y direction therefore follow the AniCraft host convention. Do not assume generic core coordinates are screen pixels or that positive Y points down. Other host adapters must explicitly convert coordinates at the boundary.

## WASM ABI version

AniCraft plugins target wasm32-unknown-unknown and use **AniCraft Plugin ABI v1** (ac_abi_version() == 1). The wire format is UTF-8 JSON. The guest SDK in crates/anicraft-plugin-sdk owns ABI exports, allocation, pointer/length packing, and synchronous host request/response transport.

See [AniCraft Plugin ABI v1](docs/PLUGIN_ABI_V1.md) for required exports/imports, runtime limits, status codes, ownership, and compatibility rules. ABI version is independent of the plugin manifest version and SDK crate version.

## Determinism guarantees

- Core calculations have no ambient randomness, global mutable state, clock dependency, or host calls.
- For identical inputs in the same build/runtime, the calculation path is repeatable; property tests cover repeatability and identity behavior.
- Non-finite values, invalid weights, missing bones, and non-finite results are rejected rather than returned as successful invalid output.
- **No cross-platform bit-for-bit floating-point guarantee is made.** Trigonometric results and compiler/runtime behavior may differ in least-significant bits. Consumers needing reproducible serialized output should define a numeric tolerance or quantization policy.
- AniCraft execution is on-demand. Each Deform evaluation uses authored neutral values and does not accumulate displacement from the currently displayed pin position.

## Limitations

- Mesh deformation is linear-blend skinning only. There is no iterative solver, cage deformation, lattice/FDD, physics, collision, or constraint solving.
- The mesh API does not enforce pins as fixed or movable constraints; pin targets are carried through unchanged.
- Current AniCraft host queries do not provide complete bind mesh data, skin weights, or evaluated bone world transforms. The mesh LBS API is therefore not wired to live AniCraft geometry.
- Slider mode calculates and logs a frame preview; it does not set the timeline or write bone properties.
- Deform mode queries a bone angle and Asset Warp pin, calculates an absolute stage-pixel target, and optionally calls the undoable warp.pin_move command. It is not a playback/scrubbing evaluator.
- Bindings are not durably persisted in the AniCraft document. The host does not expose durable arbitrary plugin metadata.
- Transform mode remains deferred until verified host transform read/write APIs are available.
- Diagnostics are opt-in. Degenerate-triangle and zero-length-bone warnings require caller-supplied triangle indices and bone segment endpoints. Native elapsed time is reported only when requested; WASM timing should be measured by the host adapter. Solver metrics are absent because LBS is not iterative.

## Build, test, and run

Install Rust and the wasm32-unknown-unknown target for plugin builds.

### Native development

~~~sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
~~~

### Build the AniCraft plugins

~~~sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
~~~

The generated modules are under target/wasm32-unknown-unknown/release/. Install/load the relevant module using the AniCraft host's plugin workflow, then invoke the command plugin from AniCraft. The exact installation path and UI are host-version-specific; this repository does not provide a standalone AniCraft runtime.

### Run core tests independently

~~~sh
cargo test -p form-deform-core
cargo test -p smart-controller-core
cargo test -p anicraft-smart-controller-plugin
~~~

## Minimal AniCraft integration example

Keep host JSON and AniCraft commands outside the pure core. This illustrates the Deform-mode boundary: query the host, convert the response to core inputs, calculate, and only then issue an explicit document edit.

~~~rust
use anicraft_plugin_sdk as ac;
use form_deform_core::{evaluate_deformation, BoneAngle, DeformRequest, DeformSpec, Vec2};
use serde_json::json;

fn deform_pin(
    bone_id: u64,
    element_id: u64,
    pin_id: u64,
    neutral_angle_deg: f64,
    neutral_stage_px: Vec2,
    gain_px_per_degree: Vec2,
    apply: bool,
) -> Result<(), String> {
    let bones = ac::command("bone.info", json!({}))?;
    let bone = bones["armatures"].as_array().into_iter().flatten()
        .flat_map(|armature| armature["bones"].as_array().into_iter().flatten())
        .find(|bone| bone["id"].as_u64() == Some(bone_id))
        .ok_or_else(|| format!("bone {bone_id} not found"))?;
    let angle = bone["angle"].as_f64().ok_or("bone angle is missing")?;
    let swing = bone["swing"].as_f64();

    let warp = ac::command("warp.info", json!({"element": element_id}))?;
    if warp["warped"] != true
        || !warp["pins"].as_array().into_iter().flatten()
            .any(|pin| pin["pin"].as_u64() == Some(pin_id))
    {
        return Err(format!("Asset Warp pin {pin_id} not found"));
    }

    let result = evaluate_deformation(DeformRequest {
        spec: DeformSpec {
            neutral_position: neutral_stage_px,
            neutral_angle_degrees: neutral_angle_deg,
            gain_per_degree: gain_px_per_degree,
        },
        bone_angle: BoneAngle {
            keyed_degrees: angle,
            spring_swing_degrees: swing,
        },
        include_spring_swing: false,
    }).map_err(|error| error.to_string())?;

    ac::log(&format!(
        "Calculated pin target: ({:.2}, {:.2}) stage px",
        result.target_position.x, result.target_position.y
    ));

    if apply {
        ac::command("warp.pin_move", json!({
            "element": element_id,
            "pin": pin_id,
            "x": result.target_position.x,
            "y": result.target_position.y
        }))?;
    }
    Ok(())
}
~~~

This is an adapter-level example, not a complete plugin manifest/entry point. A production adapter should deserialize host responses into typed DTOs, validate required fields, expose an explicit apply/preview choice, and test against the host's documented command contract. The actual entry point and ABI exports are in plugins/smart-controller.

## Feature requests versus reusable-library proposals

Use a **feature request** when the desired change is primarily AniCraft behavior or user experience: new plugin parameters, a command/mode, UI workflow, host query/edit integration, persistence, or playback behavior. Include the user workflow, expected host operations, failure/undo behavior, and required AniCraft host API. If the host lacks that API, identify the dependency rather than proposing a plugin-only workaround.

Propose a **reusable library feature** when the capability is a host-independent algorithm or data model useful across applications. Define:
- domain inputs and outputs without AniCraft JSON, SDK, command IDs, or UI types;
- explicit units, coordinate conventions, numeric tolerances, and error behavior;
- deterministic behavior and compatibility expectations;
- tests, representative workloads, and performance requirements;
- why the behavior belongs in a reusable core rather than an adapter.

Keep host serialization, ABI calls, persistence, UI, and document mutations in the plugin adapter. A reusable-library proposal must not add AniCraft dependencies to form-deform-core; an AniCraft feature request should not duplicate core math in the plugin.

## Rust core crates

- **forma-document-core** — versioned project documents, rectangle geometry, hit testing, keyframes, and timeline interpolation.
- **form-deform-core** — bone transforms, weighted mesh deformation, pin references, structured errors, and opt-in diagnostics.
- **smart-controller-core** — pure Slider validation and input-to-target mapping.
- **anicraft-plugin-sdk** — generic versioned guest ABI and host communication boundary.

## AniCraft plugin suite

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **AniCraft Smart Controller** — one unified command with Slider mode for bone-angle-to-action-frame preview and Deform mode for bone-driven Asset Warp pin targets.

The former standalone Form Deform plugin has been consolidated into Smart Controller. The Smart Controller adapter composes the independent cores with the generic AniCraft SDK.

## Diagnostics

Call deform_mesh_with_diagnostics with DeformDiagnosticsOptions. Metrics collection is opt-in. Metrics report vertex/bone counts, native elapsed nanoseconds, and interpolation mode. Solver metrics are absent because the current implementation is LBS, not an iterative solver. On wasm32-unknown-unknown, elapsed time is None; time the call in the host adapter if required.

Triangle degeneracy and zero-length-bone warnings require callers to supply triangle indices and bone segment endpoints. The core does not infer missing topology or bone lengths. Structured errors expose a stable machine-readable DeformErrorCode plus the typed source error.

## Verification

~~~sh
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
~~~
