# Forma Deform

**Forma Deform** is AniCraft's deformation and rig-control tool. It focuses on how artwork responds to bones, pins, parameters, and authored controllers. It is not a standalone animation editor and does not replace AniCraft's document, timeline, rendering, undo, or plugin systems.

The product is a native AniCraft tool backed by reusable, host-independent Rust cores. The repository contains three historical WASM command adapters used as development, diagnostic, and integration probes. The former TypeScript rectangle/timeline demo has been retired; the Rust workspace is the sole implementation and test path.

## What Forma Deform owns

- Deformation specifications and pure deformation mathematics.
- Smart Controller definitions, input normalization, mapping, easing, validation, and preview.
- Deformation targets and explicit target bindings.
- Pin, vector, lattice, and mesh deformation algorithms where AniCraft's native capabilities are insufficient.
- Corrective poses, blend-shape contributions, and reusable deformation presets.
- Deterministic, testable evaluation of its own controller and deformation models.

## What AniCraft owns

- Canonical bone identity, bone transforms, coordinate spaces, and the authoritative rig model.
- Asset Warp mesh/pin storage and rendering.
- Timeline, playhead, keyframes, document persistence, undo/redo, command registration, plugin lifecycle, evaluation scheduling, cache invalidation, and export.
- Host-side document integration needed for durable plugin-owned bindings.

Forma Deform consumes host data through explicit contracts. It must not introduce a second document model, timeline, renderer, undo stack, or plugin framework.

## UI proposal

Forma Deform should appear as a native AniCraft tool/workspace, not as a separate application window.

1. **Stage (center):** the selected artwork with deformation mesh, pins, handles, neutral/rest positions, selected target outlines, and preview overlays.
2. **Tool strip (left):** Select Target, Pin, Handle, Soft Selection, Bind Controller, and Preview. Show only tools supported by the active target type.
3. **Properties panel (right):**
   - Target and binding inspector.
   - Neutral input, source bone/controller, range, normalization, easing, and mapping.
   - Per-target gains and axis constraints.
   - Validation messages and a read-only evaluated preview.
4. **Timeline (bottom):** use AniCraft's existing timeline and playhead. Do not create a second timeline. V1 preview is explicitly on demand; it does not claim continuous playback evaluation.
5. **Controller preview:** show input value, normalized value, selected mapping segment/keyform, output value, and target displacement. Preview must be deterministic and must not mutate authored geometry.
6. **Commands:** expose bind, unbind, capture neutral, preview, and apply/bake as explicit AniCraft commands. Destructive or persistent changes must participate in AniCraft undo.

### Conceptual layout

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ AniCraft menu / tool context / command palette                           │
├──────────────┬───────────────────────────────────────┬───────────────────┤
│ Tool strip   │ Stage                                 │ Properties        │
│              │                                       │                   │
│ Select       │       artwork + mesh                  │ Target            │
│ Pin          │       pins / handles                  │ Controller        │
│ Soft select  │       neutral + preview overlay       │ Mapping           │
│ Bind         │                                       │ Diagnostics       │
│ Preview      │                                       │ Preview values    │
├──────────────┴───────────────────────────────────────┴───────────────────┤
│ Existing AniCraft timeline / layers / playhead                           │
└──────────────────────────────────────────────────────────────────────────┘
```

This is a conceptual arrangement using AniCraft's existing dockable panels and stage. It is not a commitment to new host UI infrastructure.

## Current implementation status

The Rust cores and the historical WASM adapters have distinct responsibilities:

- **Form Deform core:** reusable host-independent deformation logic and contracts. The legacy Form Deform adapter is a narrow on-demand one-bone/one-Asset-Warp-pin command; it is not the FDM topology/binding/evaluation engine.
- **Smart Controller core:** host-independent controller models, validation, mapping, migration, and deterministic evaluation. The Smart Controller adapter's active command entry point remains the V1 Slider preview. Additional canonical preview modules are read-only foundations, not a complete native authoring or playback workflow.
- **Bone Query adapter:** read-only development diagnostics for AniCraft armature, bone, pose, spring, and optional skin-weight data.

The adapters are not a substitute for native AniCraft document persistence, undoable authoring workflows, or a continuous custom playback evaluator. Durable bindings and native evaluation require verified AniCraft host support.

Smart Controller evaluation and Form Deform pin evaluation remain independent. Do not implicitly combine their outputs. Multi-source composition requires a separately approved semantic design.

See [Legacy Adapter Status and Retirement Plan](docs/LEGACY_ADAPTER_RETIREMENT.md) for the current disposition and retirement gates.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for the feature allocation matrix, external code-reuse audit, UI behavior, integration boundaries, milestones, and acceptance criteria.

Additional technical references:

- [Plugin overview](plugins/README.md)
- [Build and smoke-test guide](plugins/INSTALL.md)
- [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md)
- [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md)
- [Capability limits](docs/CAPABILITIES.md)
- [Legacy adapter status and retirement plan](docs/LEGACY_ADAPTER_RETIREMENT.md)

## Verification

The current CI workflow checks Rust formatting, runs the Rust workspace tests, and builds the three historical WASM adapter crates. These checks establish build/test coverage only; they do not establish successful in-app integration or native playback evaluation.

For the Rust workspace and WASM adapters:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```


