# Forma Deform

**Forma Deform** is AniCraft's deformation and rig-control subsystem. It focuses on how artwork responds to bones, pins, parameters, and authored controllers. It is not a standalone animation editor and does not replace AniCraft's document, timeline, rendering, undo, or plugin systems.

The intended product is a native AniCraft tool/workspace. The repository contains Rust deformation/controller cores and historical AniCraft command-plugin adapters. The default `main` branch is not yet the complete native integration: active implementation, audit, and retirement work is tracked in separate pull-request chains.

## Product boundary

### Forma Deform owns

- Pure deformation algorithms and deformation-specific data structures.
- Smart Controller definitions, input normalization, mapping, easing, validation, and on-demand preview.
- Deformation target abstractions and explicit target-binding specifications.
- Corrective poses, deformation contributions, and reusable deformation presets.
- Deterministic evaluation of its own models against explicit host inputs.

### AniCraft owns

- Canonical document, element and bone identity; rig transforms and coordinate spaces.
- Native Asset Warp storage, geometry editing and rendering.
- Timeline, playhead, keyframes, document persistence, undo/redo and export.
- Command registration, plugin lifecycle, evaluation scheduling, cache invalidation and host transactions.

Forma Deform must consume host data through explicit contracts. It must not introduce a second document model, timeline, renderer, undo stack, cache system, or plugin framework.

## Accepted integration constraints

- Smart Controller binding evaluation and Form Deform pin evaluation remain independent (CCC-01.5). Do not combine their outputs implicitly.
- Native Asset Warp remains AniCraft-owned; FDM must not replace it without a demonstrated capability gap and an explicit decision.
- Preview is read-only and on demand. It does not imply continuous playback evaluation.
- Apply/bake must be an explicit host command and participate in AniCraft undo.
- Durable plugin-owned bindings require a verified AniCraft document-extension/persistence contract.
- No runtime implementation, host change, PR merge, or release is authorized merely by this roadmap.

## Proposed native-tool workspace

1. **Stage:** selected artwork with supported deformation mesh, pins, handles, neutral/rest markers and preview overlays.
2. **Tool strip:** target selection, pin/handle tools, soft selection, binding and preview; expose only tools supported by the active target.
3. **Properties:** target/binding inspector, source input, neutral/range, normalization, easing, mapping, per-target gains, diagnostics and read-only preview.
4. **Timeline:** use AniCraft's existing timeline and playhead; never create a second timeline.
5. **Commands:** bind, unbind, capture neutral, preview and explicit apply/bake.

This is a workspace proposal using AniCraft's existing UI infrastructure, not a commitment to new host infrastructure. See [docs/UI_UX_DESIGN.md](docs/UI_UX_DESIGN.md).

## Current repository status — 23 November 2026

This status distinguishes the default branch from work on separate PR branches.

| Area | Current evidence | Status |
|---|---|---|
| Default-branch baseline | Latest recorded `main` commit: `e182e447` — Flare source-level reuse audit | Current source/document baseline |
| Legacy Bun/TypeScript demo retirement | PR [#52](https://github.com/Mujshanan/forma-deform/pull/52) merged into the LEG-01.4 branch; PR [#54](https://github.com/Mujshanan/forma-deform/pull/54) carries retirement into the FDM-04 branch chain | Not yet represented as a merge to default `main` |
| FDM-05.4 output-contract hardening | PR [#60](https://github.com/Mujshanan/forma-deform/pull/60) | Open; do not treat as merged |
| Workspace formatting remediation | PR [#68](https://github.com/Mujshanan/forma-deform/pull/68) records a 69-file baseline only | No formatting sweep performed by that PR |
| Native host boundary and S0 audit | PRs [#73–#80](https://github.com/Mujshanan/forma-deform/pulls) | Open, documentation/audit/planning work; S0 remains OPEN |

The latest S0 evidence addendum is PR #80. It adds source-located evidence for the 36 previously missed inline callback entries, first-order effect classification, a first-pass graph for the 131 `mutates=false` commands, and a verified AniCraft archive/source baseline. This is not formal S0 closure: callback transitive-effect coverage, compiler-backed consumer verification and successful host CI evidence remain outstanding. Do not mark S0 closed.

The AniCraft source baseline used by the current host audit is pinned to `2df7775907d1458ce653e7e6314ec8a5d7d565f5`. Reconfirm that baseline before relying on the audit for implementation.

## Prototype capabilities and limitations

The Rust workspace includes deformation and controller cores, together with historical command-plugin adapters for:

- **Bone Query:** diagnostic/read-only rig inspection.
- **Form Deform:** deterministic bone-angle-driven Asset Warp pin positioning from authored neutral positions and per-degree gains.
- **Smart Controller:** neutral-centered input mapping, easing and on-demand preview.

These adapters are integration probes, not a completed native authoring workflow. Do not claim live Smart Controller playback, durable plugin-owned binding persistence, or native FDM host integration until the corresponding host contracts and tests exist. Adapter retirement remains gated on verified native parity and consumer migration.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for the feature allocation matrix, code-reuse audit, current PR/evidence status, integration boundaries, milestones and acceptance criteria.

Additional references:

- [Native-tool UI/UX design](docs/UI_UX_DESIGN.md)
- [Plugin overview](plugins/README.md)
- [Build and smoke-test guide](plugins/INSTALL.md)
- [AniCraft SDK audit](docs/ANICRAFT_SDK_AUDIT.md)
- [Smart Controller plan](docs/SMART_CONTROLLER_PLAN.md)
- [Capability limits](docs/CAPABILITIES.md)

## Verification

Use the commands appropriate to the checked-out branch and its current workspace. For the Rust workspace and WASM adapters:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-form-deform-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

Do not infer that these commands passed on a branch from a result reported for a different commit or PR. Check the exact head SHA and workflow run. The legacy Bun demo/toolchain retirement is present on its dedicated PR branch chain; the default branch must not be described as retired until that work is integrated there.
