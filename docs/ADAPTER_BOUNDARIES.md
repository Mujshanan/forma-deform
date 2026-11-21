# CCC-01.6 — Adapter Isolation

**Status:** Implemented on a review branch; pending review.

## Purpose

Keep AniCraft-specific host integration at the plugin boundary. Reusable geometry, controller, deformation, and orchestration crates must remain independent of AniCraft commands, host response formats, WASM ABI helpers, and document mutation APIs.

This milestone formalizes and verifies the existing separation. It does not redesign plugin behavior or introduce a new host abstraction.

## Dependency boundary

The intended dependency direction is:

```text
AniCraft host
    ↕ ABI v1
anicraft-plugin-sdk
    ↑
AniCraft plugin adapters
    ├── bone-query plugin ──────────────── (read-only host inspection)
    ├── form-deform plugin ─────────────── (host queries + optional undoable pin edit)
    └── smart-controller plugin ───────── (host query + preview logging only)
             │
             ├── form-deform-core
             └── smart-controller-core

forma-deformation-integration
    ├── form-deform-core
    └── smart-controller-core

Reusable crates:
    forma-geometry
    form-deform-core
    smart-controller-core
    forma-deformation-integration
```

The diagram is a dependency/ownership guide, not a requirement that every plugin depend on both cores. Each adapter depends only on the reusable functionality it actually uses.

## Responsibilities

### Reusable crates

- `forma-geometry`: host-neutral value primitives.
- `form-deform-core`: deterministic pin-deformation mathematics and validation.
- `smart-controller-core`: controller binding model, evaluation, mapping, and preview behavior.
- `forma-deformation-integration`: independent orchestration of controller and pin evaluators; it does not compose their outputs.

Reusable crates must not:
- import `anicraft_plugin_sdk` or call host commands;
- parse AniCraft-specific JSON response shapes;
- own WASM ABI entry points, memory transfer, or host capability negotiation;
- mutate an AniCraft document or issue undo/redo operations;
- depend on plugin crates.

### AniCraft plugin adapters

- Parse plugin parameters and validate host-facing inputs.
- Invoke `anicraft-plugin-sdk` commands.
- Translate host response payloads into explicit inputs for reusable APIs.
- Translate reusable results/errors into host-facing logs or command responses.
- Perform host document writes only where the plugin's existing contract explicitly permits them.

Existing behavior remains authoritative:
- Bone Query is read-only.
- Form Deform may apply its calculated pin position only when its explicit `apply` parameter is enabled; the host edit remains undoable.
- Smart Controller V1 remains preview-only and must not write timeline or document state.

## SDK boundary

`anicraft-plugin-sdk` is the guest-side ABI v1 adapter support crate. It may contain host-specific command transport, serialization, memory/ABI handling, and plugin entry-point support. It is not a reusable geometry or deformation dependency.

The SDK must not depend on the reusable engines. Reusable crates must not depend on the SDK. This keeps the engine APIs testable without an AniCraft runtime and prevents host ABI details from leaking into FDM v2.

## Automated enforcement

`scripts/check-adapter-boundaries.sh` checks the normal Cargo dependency graph:

1. Each reusable crate is free of a direct or transitive normal dependency on `anicraft-plugin-sdk`.
2. Each AniCraft plugin adapter depends on `anicraft-plugin-sdk`.

The CI workflow runs this check. It supplements, rather than replaces, code review of host-specific response parsing and mutation calls.

## Non-goals

- No plugin command names, parameter schemas, ABI v1 layouts, or response formats are changed.
- No host command wrappers are moved into reusable crates.
- No common plugin framework or additional adapter crate is introduced.
- No controller/deformation composition semantics are added.
- No changes to undo behavior, preview behavior, or the existing pin formula.
- No FDM v2 geometry ownership, coordinate-space, persistence, or animation decisions are made.

## Review checklist

- [ ] Reusable crates remain free of AniCraft SDK dependencies.
- [ ] Host command calls and host response parsing remain in plugin adapters or the SDK.
- [ ] Only the Form Deform adapter can issue the existing optional pin-move command.
- [ ] Smart Controller V1 remains preview-only.
- [ ] CI executes the dependency-boundary script.
- [ ] Workspace tests and all plugin WASM release builds pass.
