# Forma Deform — Smart Controller Development Plan

**Status:** Active development plan (2026-11-19)  
**Target host:** [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft)  
**Repository:** [Mujshanan/forma-deform](https://github.com/Mujshanan/forma-deform)  
**V1 objective:** Author, persist, and preview Smart Controller bindings on demand.  
**Runtime constraint:** V1 does not provide continuous playback/scrubbing evaluation.

## 1. Product direction

Forma Deform will provide a Smart Controller plugin for AniCraft, beginning with a small, independently testable controller-binding engine.

A binding connects a controller input (initially a bone angle) to one target property. V1 targets:

- **SymbolFrame:** map negative, neutral, and positive controller poses to frames of a nested symbol instance.
- **GeometryDeform:** map negative, neutral, and positive controller poses to a two-dimensional geometry offset.

V1 is an authoring and on-demand preview tool. It must not claim automatic evaluation during AniCraft playback or timeline scrubbing. That requires a host runtime integration not yet established by the current plugin SDK.

The existing TypeScript rectangle/timeline demo remains as a geometry experiment and visual reference. It is not the Smart Controller runtime and does not prove AniCraft plugin integration.

## 2. V1 scope

### Included

1. Define, validate, inspect, update, and remove controller bindings.
2. Identify controller bones and targets using stable host IDs.
3. Support finite, ordered input ranges with negative, neutral, and positive values.
4. Support Linear and Smoothstep interpolation.
5. Support AbsolutePose and RelativeDelta modes with explicit semantics.
6. Preview a binding for a caller-supplied controller angle.
7. Resolve target existence/type through the host adapter before host-side preview.
8. Persist bindings only through a verified AniCraft document metadata or engine-command API.
9. Keep the pure controller model, validation, normalization, easing, and mapping independent of WASM host calls.
10. Run core unit tests natively with `cargo test`; build the host adapter for `wasm32` only after its SDK contract is verified.

### Explicitly excluded

- Continuous evaluation during playback or timeline scrubbing.
- Runtime hooks or custom bone evaluator APIs not exposed by the verified SDK.
- Mutating authored keyframes during preview.
- Applying preview by repeatedly issuing relative transform commands.
- Arbitrary mesh topology changes, skinning, weight painting, or IK.
- Geometry targets requiring unsupported host mutation APIs.
- A custom animation editor replacing AniCraft.

## 3. Architecture

Intended Rust layout:

```text
crates/
  smart-controller-core/    # Native Rust library; no WASM or AniCraft dependencies
  anicraft-plugin/          # Thin WASM adapter; add only after SDK audit
src/                         # Existing TypeScript rectangle demonstration
docs/
  DEVELOPMENT_PLAN.md        # Existing rectangle-lab plan/history
  SMART_CONTROLLER_PLAN.md   # Authoritative Smart Controller plan
```

### Core responsibilities

- Binding data types and schema version.
- Input-range validation and signed neutral-centered normalization.
- Easing and piecewise mapping through negative → neutral → positive values.
- Pure target-value calculation.
- Binding-set validation, including duplicate binding IDs.
- Typed errors for invalid ranges, incompatible modes, non-finite arithmetic, and invalid targets.

The core must not import AniCraft SDK types, WASM host functions, browser APIs, or UI code.

### Adapter responsibilities

- Register plugin metadata and commands using the actual AniCraft SDK contract.
- Inspect the current document and resolve controller/target IDs.
- Load and save bindings through a verified durable storage mechanism.
- Convert host data into core inputs and results back into host operations.
- Keep preview read-only unless the SDK provides a documented temporary-preview mechanism.
- Report unsupported host capabilities rather than simulating them with unsafe mutations.

The core calculates values. The adapter resolves host identity, persistence, and display. The host remains authoritative for document validity and rendering.

## 4. Binding semantics

A binding has its own stable binding ID and a separate controller ID. One controller may drive multiple bindings.

Input ranges are ordered: `minimum < neutral < maximum`. Input is clamped and normalized piecewise around neutral:

- minimum maps to -1;
- neutral maps to 0;
- maximum maps to +1.

Easing is applied independently on each side so neutral remains fixed.

### Modes

- **AbsolutePose:** calculated target values are absolute values in the target's declared coordinate/property space.
- **RelativeDelta:** calculated geometry output is an offset relative to the neutral pose; at neutral it must be exactly zero. V1 does not allow RelativeDelta for SymbolFrame because integer frame selection is an absolute target, not a meaningful additive delta.

### Targets

- **SymbolFrame:** negative, neutral, and positive frame anchors. Interpolate piecewise and round to the nearest integer frame. RelativeDelta is invalid.
- **GeometryDeform:** negative, neutral, and positive 2D offsets. AbsolutePose returns the interpolated offset; RelativeDelta returns the interpolated offset minus the neutral offset.

All target identifiers are stable strings. Array indices and current selection order are not persistent identity.

## 5. Persistence and preview rules

Persistence is not assumed. Before implementing it, audit the current AniCraft plugin SDK and engine/document APIs for:

1. Reading the document and stable element/bone IDs.
2. Durable document metadata or supported plugin-owned storage.
3. Command registration and command argument/result schemas.
4. A non-destructive preview or temporary display-list override.
5. Whether command sequences can be atomic and undoable.

Choose the narrowest supported persistence mechanism and document its versioning behavior. Do not invent metadata fields or command schemas.

Preview flow:

1. Validate the binding.
2. Resolve the controller input supplied by the preview request.
3. Calculate the pure core result.
4. Ask the adapter to resolve and validate the target against the current document.
5. Return a preview result.
6. Display it only through a verified non-destructive host preview facility. If none exists, return the calculated result and report that visual host preview is unavailable.

Preview must never rewrite authored keyframes or accumulate transforms.

## 6. Development phases

### Phase 0 — Repository and SDK audit

- Preserve the existing TypeScript rectangle demo and its history.
- Record this Smart Controller scope and architecture.
- Inspect the exact AniCraft plugin SDK version and example plugins.
- Verify document inspection, stable IDs, persistence, command registration, and preview capabilities.
- Record capability findings with source paths and exact command schemas.

**Acceptance:** a capability matrix identifies what can be implemented entirely in a plugin and what requires host-side work. No guessed API is used.

### Phase 1 — Native Rust controller core

- Add a Cargo workspace without disrupting the Bun/TypeScript project.
- Implement versionable binding types, input ranges, easing, modes, and target variants.
- Implement validation, signed normalization, and pure target mapping.
- Add typed errors and native unit tests.
- Keep all host/WASM code out of this crate.

**Acceptance:** `cargo test -p smart-controller-core` passes natively; the crate has no AniCraft or WASM dependency; edge cases and overflow are tested.

### Phase 2 — Binding collections and serialization contract

- Define a versioned persisted binding-set representation.
- Validate duplicate binding IDs and incompatible target/mode combinations.
- Add serialization only after selecting a stable format and dependency policy.
- Add round-trip and unsupported-version tests.

**Acceptance:** valid binding sets round-trip; invalid/unknown versions fail explicitly; controller and target IDs are preserved exactly.

### Phase 3 — AniCraft SDK adapter

- Implement the plugin entry point and command registration against verified SDK APIs.
- Add bind, unbind, update, inspect, and preview actions where supported.
- Resolve host targets and validate their types before any write.
- Implement persistence using the verified mechanism.

**Acceptance:** plugin loads in AniCraft; authoring actions work; save/reopen preserves bindings; invalid IDs and unsupported capabilities produce actionable errors.

### Phase 4 — On-demand preview

- Accept an explicit angle or normalized input in the preview request.
- Calculate target output with the native core.
- Integrate only a documented non-destructive host preview facility.
- Ensure repeated preview requests are deterministic and do not mutate authored data.

**Acceptance:** preview at negative, neutral, and positive inputs returns expected values; repeated preview has no cumulative effects; no live playback claim is made.

### Phase 5 — Verification and v1 release

- Run native Rust tests and formatting/lint checks.
- Build the WASM adapter for the SDK's supported target.
- Test plugin loading, persistence, target validation, and preview in AniCraft.
- Document install, supported AniCraft version, limitations, and examples.

**Acceptance:** native tests and WASM build pass; a clean AniCraft project can author, save, reopen, inspect, and on-demand preview a binding; documented limitations match observed host behavior.

## 7. Testing strategy

### Native core tests

- Invalid, non-finite, reversed, and overflowing input ranges.
- Values below minimum and above maximum.
- Exact negative, neutral, and positive normalization.
- Linear and Smoothstep interpolation on both sides of neutral.
- Symbol-frame interpolation and rounding.
- Rejection of RelativeDelta for SymbolFrame.
- Geometry absolute offsets and neutral-relative deltas.
- Non-finite output detection after arithmetic.
- Duplicate binding IDs.
- Disabled bindings and deterministic repeated evaluation.
- No mutation of input bindings during evaluation.

### Adapter tests

- Unknown controller ID.
- Missing target ID.
- Target exists but has the wrong host type.
- Persistence round trip and schema migration behavior.
- Preview unsupported by host.
- Host operation failure without partial document mutation.

## 8. Risks and decision gates

| Risk | Required response |
|---|---|
| SDK cannot persist plugin metadata | Do not fake document persistence; evaluate a minimal host extension or defer persistence |
| SDK cannot read bone pose/input | Keep explicit-angle preview; request the smallest host API extension for bone-driven input |
| SDK cannot preview without mutation | Return computed preview data only; do not issue accumulating transform commands |
| Symbol frame cannot be temporarily overridden | Keep the core result and defer visual preview until host support exists |
| Geometry target cannot be identified stably | Do not use array positions; verify stable IDs first |
| Existing TypeScript demo diverges from plugin model | Keep it as a separate demo; do not couple it to plugin persistence or runtime |

## 9. Current progress

| Work item | Status |
|---|---|
| Existing TypeScript rectangle geometry/timeline demo | Implemented on `feat/rectangle-deformation-mvp` |
| Smart Controller v1 scope | Defined |
| Smart Controller development plan | Added in this branch |
| AniCraft SDK capability audit | Next |
| Native Rust core | In progress |
| Binding persistence | Blocked on SDK audit |
| WASM adapter | Blocked on SDK audit |
| Host visual preview | Blocked on SDK capability verification |
| Continuous playback integration | Out of v1 scope |

## 10. Definition of done

V1 is complete when a user can author and save a binding, reopen the project, inspect it, request a preview for a chosen controller angle, receive a validated target result, and—if supported by the verified host API—see that result without modifying authored animation data. Native Rust tests and the WASM plugin build must pass. Continuous playback/scrubbing evaluation remains a later host-integration milestone.
