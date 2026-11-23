# Forma Deform

**Forma Deform** is AniCraft's proposed deformation and rig-control tool. It focuses on how artwork responds to bones, pins, parameters, and authored controllers. It is not a standalone animation editor and does not replace AniCraft's document, timeline, rendering, undo, or plugin systems.

The repository currently contains an experimental TypeScript rectangle/timeline demo and a Rust/WASM AniCraft command-plugin suite. Treat those as separate development tracks: the long-term product is the native AniCraft tool, while the current command plugins are prototypes and integration probes.

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


## Native UI implementation status (2026-11-23)

The first M1 Smart Controller UI foundation is merged in [AniCraft PR #2](https://github.com/Mujshanan/anicraft/pull/2). It adds a native tool-strip entry, transient Stage handles, a foreground overlay, and a draft-only Properties inspector. It does not yet implement source/target binding, mapping, evaluation, preview, persistence, target application, or live playback. See [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md) for the detailed specification and the implementation-status addendum.


## Native UI progress (2026-11-23)

The M1 Smart Controller UI now supports click-to-create/select and drag-to-move transient Stage handles, with live coordinates in Properties. This is merged into AniCraft in [PR #3](https://github.com/Mujshanan/anicraft/pull/3). It remains draft-only editor state; persistent bindings, mapping/evaluation, target application, undo integration and live playback are not implemented. Native CI and manual Windows smoke-test evidence remain outstanding. See [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md).


## Native UI progress — M2 (2026-11-23)

AniCraft [PR #4](https://github.com/Mujshanan/anicraft/pull/4) is merged. The Smart Controller Properties inspector now displays the currently selected bone identity and selected artwork identity using AniCraft's existing selections. This is read-only inspection only: target compatibility, controller-input snapshots, neutral capture, bindings, persistence, evaluation, and playback remain unavailable. No automated test or native Windows CI evidence was reported for the merge. See [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md).
