# Form  Deform

**Form  Deform** is AniCraft's deformation and rig-control product, developed as a native AniCraft tool. It focuses on how artwork responds to bones, pins, parameters, and authored controllers. It is not a standalone animation editor and does not replace AniCraft's document, timeline, rendering, undo, or plugin systems.

## Product evolution and current implementation status

Forma Deform began as three separate proof-of-concept plugins. Their capabilities were consolidated into one Forma Deform product and codebase, which then evolved toward a native AniCraft tool. The three plugin crates still present in this repository are historical adapters, independently buildable integration probes, and compatibility/reference tracks; they should not be interpreted as three separately shipped products.

The native Smart Controller work is tracked in the AniCraft host repository. At the time of this audit, branch [`feat/smart-controller-ui-01-03`](https://github.com/Mujshanan/anicraft/tree/feat/smart-controller-ui-01-03) contains a native toolbar tool, Stage handle overlay, and Properties inspector. That branch is not merged into AniCraft `main`. The current UI slice is transient authoring scaffolding: controller persistence, a host adapter to the controller core, evaluated source resolution, target application, and live preview are not yet connected. The branch's own tool hint says binding and preview are not connected.

This repository remains the home of reusable, host-independent deformation/controller logic and its tests. Native host UI and document/evaluation ownership remain in AniCraft. See [the native host integration traceability report](docs/quality/native-host-integration-traceability.md) for exact branch, source, and capability evidence.

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

## Current prototype status and limitations

The current command-plugin suite contains:

- **Bone Query:** read-only inspection of armatures, bones, poses, spring state, joint translation, and skin weights.
- **Form Deform:** deterministic bone-angle-driven Asset Warp pin positioning from an authored neutral position and per-degree gains.
- **Smart Controller:** neutral-centered input mapping, easing, and preview output.

The current command-plugin API supports on-demand queries and explicit undoable pin edits. It does not currently establish arbitrary durable plugin metadata or a continuous custom playback evaluator. Do not describe the prototype as providing live Smart Controller playback. Persistent bindings and native evaluation require explicit AniCraft host support.

Smart Controller evaluation and Form Deform pin evaluation remain independent. Do not implicitly combine their outputs. Multi-source composition requires a separately approved semantic design.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for the feature allocation matrix, external code-reuse audit, integration boundaries, milestones, and acceptance criteria. See [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md) for the detailed native-tool UI/UX specification, workspace layout, interaction flows, preview states, and acceptance criteria.

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

The separate TypeScript demo retains its Bun-based scripts. See the package scripts before running it.
