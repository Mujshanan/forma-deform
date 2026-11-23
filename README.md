# Forma Deform

**Forma Deform** is a focused, standalone deformation authoring Studio backed by independent deformation engines. It provides authoring tools for pins and offset drivers, point arrays and binding, cages, Bézier splines, and vector tessellation. It is not a general animation editor.

Forma Deform Studio must launch and operate independently. AniCraft is a future optional integration target, not a runtime dependency. The Studio should share AniCraft's intended visual language and interaction conventions through a small, independently consumable UI design kit; it must not depend on the AniCraft application or unverified host APIs.

The default development entrypoint now opens an initial browser-hosted Forma Deform Studio shell. It includes the focused deformation tool rail, viewport navigation, contextual inspector, and status area. Feature editors, geometry loading, engine adapters, and project persistence are not connected yet. The former rectangle/timeline prototype remains as legacy source and tests; it is no longer the default UI and is not the Studio document model.

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

## Studio workspace

Forma Deform Studio uses an AniCraft-compatible visual language without copying AniCraft's full feature set.

- **Top bar:** project actions, undo/redo, commands, and viewport/workspace options.
- **Left tool rail:** Select, Pin, Point Array, Bind, Cage, Bézier, and Tessellate.
- **Center viewport:** selected geometry, pins, offsets, arrays, cages, curve handles, and non-destructive overlays.
- **Right inspector:** contextual target/tool properties, driver and binding state, validation, and diagnostics.
- **Bottom status area:** selection summary, coordinates, operation status, and diagnostics.

There is no general animation timeline in the standalone Studio. Preview controls are focused on deformation inputs and must not become a second animation editor.

See [UI-01 workspace contract](docs/UI_01_DEFORMATION_AUTHORING_WORKSPACE.md), [UI-02 source/UI architecture audit](docs/UI_02_SOURCE_AND_UI_ARCHITECTURE_AUDIT.md), and [UI-03.1 shell implementation](https://github.com/Mujshanan/forma-deform/commit/ec44889fdb5ced812099618f9b67663109a03257).

## Current prototype status and limitations

The current command-plugin suite contains:

- **Bone Query:** read-only inspection of armatures, bones, poses, spring state, joint translation, and skin weights.
- **Form Deform:** deterministic bone-angle-driven Asset Warp pin positioning from an authored neutral position and per-degree gains.
- **Smart Controller:** neutral-centered input mapping, easing, and preview output.

The current command-plugin API supports on-demand queries and explicit undoable pin edits. It does not currently establish arbitrary durable plugin metadata or a continuous custom playback evaluator. Do not describe the prototype as providing live Smart Controller playback. Persistent bindings and native evaluation require explicit AniCraft host support.

Smart Controller evaluation and Form Deform pin evaluation remain independent. Do not implicitly combine their outputs. Multi-source composition requires a separately approved semantic design.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for the feature allocation matrix, external code-reuse audit, UI behavior, integration boundaries, milestones, and acceptance criteria.

Additional technical references:

- [Plugin overview](plugins/README.md)
- [Build and smoke-test guide](plugins/INSTALL.md)
- [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md)
- [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md)
- [Capability limits](docs/CAPABILITIES.md)

## Verification

For the Rust workspace and WASM plugins:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The TypeScript Studio shell uses the existing Bun development server and build scripts. The legacy rectangle-demo modules and tests remain in the repository but are no longer the default build entrypoint.
