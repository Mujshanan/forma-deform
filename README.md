# Forma Deform

**Forma Deform** is a reusable 2D deformation project with an AniCraft plugin suite. The original TypeScript rectangle/timeline demo remains a separate geometry experiment.

## AniCraft plugin suite

The branch `feat/smart-controller-v1` contains three separate command plugins:

- **Bone Query** — read-only armature, bone, pose, spring, joint-translation, and skin-weight inspection.
- **Form Deform** — deterministic bone-angle-driven Asset Warp pin deformation using an authored neutral position and per-degree gains.
- **Smart Controller** — general-purpose neutral-centered input mapping with Linear/Smoothstep easing and three geometry anchors.

Each plugin has its own crate and manifest. Shared native logic is isolated under `crates/`. The AniCraft guest SDK is vendored with pinned upstream provenance.

See the [plugin overview](plugins/README.md), [unified build and smoke-test guide](plugins/INSTALL.md), [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md), and [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md).

## V4 MCP slider prototype

V4 adds an external Python experiment that connects to an open AniCraft window through its MCP bridge. It reads a driver angle with `bone.info`, applies the same Slider endpoint semantics as V1, and moves the **global scene playhead** with `timeline.goto`.

- Prototype: [tools/v4_mcp_slider.py](tools/v4_mcp_slider.py)
- Findings and limitations: [docs/V4_MCP_FINDINGS.md](docs/V4_MCP_FINDINGS.md)
- Mapping tests: `python -m unittest discover -s tests -p "test_v4_mcp_slider.py" -v`

Start AniCraft with remote control enabled, then run the prototype from this repository:

```sh
anicraft --control 9878
python tools/v4_mcp_slider.py --layer ARMATURE_LAYER_ID --bone BONE_ID --source-frame 0 --watch --cycles 20
```

Replace the IDs with values from Bone Query or `bone.info`. The watch mode samples only while the UI is at the fixed source frame; after each mapped goto, return the playhead to that source frame to make the next driver adjustment.

**V4 is a feasibility prototype, not a production Smart Bone/Slider feature.** `timeline.goto` changes the global scene playhead. It does not set a nested graphic symbol instance's local displayed frame. The prototype does not write driver poses, authored keyframes, or persistent controller bindings. Live latency and 10-cycle acceptance still require testing against the user's running AniCraft desktop session.

## Important host boundaries

The AniCraft command-plugin API supports on-demand bone and warp queries and undoable warp-pin edits. MCP adds external access to the host command registry, including `bone.info`, `bone.pose`, and `timeline.goto`. It does not provide an independent playhead per layer, durable arbitrary plugin metadata, or continuous Smart Controller evaluation during playback. A true nested-symbol Slider or frame-synchronized controller still needs a dedicated host command or native evaluation hook.

## Verification

Run the complete Rust suite and build all three WASM plugins:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The V4 Python checks are separate from the Rust workspace:

```sh
python -m unittest discover -s tests -p "test_v4_mcp_slider.py" -v
python -m py_compile tools/v4_mcp_slider.py
```

The TypeScript demo retains its Bun-based scripts. GitHub Actions checks both existing projects.
