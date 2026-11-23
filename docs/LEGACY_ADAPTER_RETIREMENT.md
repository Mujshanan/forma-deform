# Legacy Adapter Status and Retirement Plan

**Assessment date:** 2026-11-23  
**Audited source branch:** `feat/fdm-02-1-identity-topology-contracts`  
**Audited source commit:** `70ec1b3a55d602de93a5b9424fcd8dad99b35807`  
**Scope:** Documentation and retirement planning only. This document does not authorize adapter deletion, host-repository changes, or PR merges.

## 1. Executive status

Forma Deform is one AniCraft deformation and rig-control product with reusable Rust domain cores. The three WASM command crates are historical development, diagnostic, and integration adapters. They are not three independent production products, and their presence does not prove that the native AniCraft workflow is complete.

The current workspace contains:

- `form-deform-core` — host-independent deformation mathematics and contracts.
- `smart-controller-core` — host-independent controller model, validation, mapping, evaluation, compatibility, and migration.
- `anicraft-plugin-sdk` — vendored AniCraft guest-side ABI v1 helpers.
- `anicraft-bone-query-plugin` — read-only diagnostic adapter.
- `anicraft-form-deform-plugin` — legacy one-bone/one-pin command adapter.
- `anicraft-smart-controller-plugin` — transitional command adapter with an active V1 Slider preview path and additional read-only canonical preview foundations.

The current CI workflow is configured to run workspace tests and build all three WASM adapters. This is build/test automation, not evidence of successful in-app smoke testing or native playback integration.

## 2. Adapter disposition matrix

| Component | What exists now | Decision | Retirement condition |
|---|---|---|---|
| `plugins/form-deform` | On-demand command that reads a bone angle and calculates/applies one Asset Warp pin position | **Retirement candidate** | Native FDM host workflow reaches the parity gate in section 4; useful fixtures and core tests are retained |
| `plugins/smart-controller` | Active V1 Slider preview command; additional input, target-inspection, migration, and canonical read-only preview helpers | **Transitional; retain** | Native Smart Controller authoring, persistence, preview, and target workflow are implemented and verified; adapter-only tests/fixtures are preserved or migrated |
| `plugins/bone-query` | Read-only inspection of bone, pose, spring, joint translation, and optional skin-weight data | **Retain as diagnostics for now** | Equivalent native diagnostics exist, or the team explicitly elects to keep a separate development-only tool |
| `crates/anicraft-plugin-sdk` | Vendored guest-side ABI v1 helper crate consumed by all three adapters | **Retain; retire last** | No remaining workspace member, test, script, or supported workflow depends on it |

### 2.1 Form Deform adapter

The old adapter is intentionally narrow:

- Reads an on-demand bone query through `bone.info`.
- Uses an authored neutral angle, neutral pin position, and per-degree gains.
- Calculates an absolute target position for one Asset Warp pin.
- Can issue `warp.pin_move` through an explicit Apply option.
- Does not provide the modern FDM topology, cage, binding, persistence, or evaluation architecture.

Its mathematical and geometry contracts belong in `form-deform-core`, not in the legacy host command wrapper. Retiring the wrapper must not remove reusable core behavior or regression fixtures.

### 2.2 Smart Controller adapter

The active plugin entry point remains the V1 Slider preview path:

- It can use an on-demand `bone.info` input or explicit `manual_value`.
- It normalizes and maps the input to an action frame.
- It reports deterministic preview output through the plugin log.
- It does not set the timeline, apply geometry targets, write bone properties, or run continuously during playback/scrubbing.

The adapter also contains useful foundations: numeric bone-ID parsing, invocation-scoped input snapshots, read-only `warp.info` target inspection, canonical document preview evaluation through an injected mapper, target capability planning, and explicit migration/reverse-export helpers.

Those foundations are not a complete native workflow. The canonical preview plan is descriptive/read-only; it does not call target setters or mutate the document. The current active command entry point must not be described as exposing all of these capabilities as a user-facing integrated feature.

### 2.3 Bone Query adapter

Bone Query is read-only and has value as a development diagnostic. It reports host data that is useful when checking identity, keyed pose, spring swing, joint translation, and shape skin influences.

It is not a production dependency of either deformation core. Its useful fixtures and diagnostic behavior should be preserved if the WASM adapter is later removed.

### 2.4 SDK

The vendored SDK is a packaging boundary for the host ABI, not domain logic. Its provenance is pinned in `crates/anicraft-plugin-sdk/UPSTREAM.md`.

Do not remove the SDK while any adapter, test, script, or documented workflow still consumes it. Its retirement is the final cleanup after all adapter consumers have been removed or migrated.

## 3. Core behavior that must remain

Adapter retirement is not permission to delete core compatibility or evaluation behavior.

- Keep `form-deform-core` as the host-independent deformation domain.
- Keep `smart-controller-core` as the host-independent controller domain.
- Keep `smart-controller-core/src/legacy.rs` and `migration.rs` until a separate compatibility-retirement decision defines supported source formats, migration guarantees, and replacement tests.
- Preserve deterministic evaluator, validation, composition, and migration tests.
- Keep Smart Controller evaluation independent from Form Deform pin-deformation evaluation. No cross-engine output composition is implied by removing adapters.
- Keep AniCraft responsible for authoritative document data, timeline, rendering, undo/redo, host target identity, and evaluation scheduling.

## 4. Native replacement parity gates

An adapter is not replaceable merely because a native UI control or a core API exists. The replacement must demonstrate the corresponding end-to-end behavior.

### 4.1 Form Deform replacement gate

Before retiring `plugins/form-deform`, verify all of the following in the native AniCraft workflow:

1. The native tool consumes stable host source and target identities rather than transient selection indices.
2. FDM topology and binding data are validated against current geometry/topology revisions.
3. Authored binding data has a versioned, host-owned persistence path with save/reopen and migration tests.
4. Preview is non-mutating and produces deterministic results from explicit inputs.
5. Apply/bake uses a host command with coherent undo/redo behavior.
6. Stale bindings, missing targets, unsupported target types, and invalid geometry produce actionable diagnostics.
7. Native Windows build and manual smoke tests pass on a recorded AniCraft commit and fixture.
8. The old adapter's unique tests and fixtures are either retained at core/host-contract level or intentionally documented as obsolete.

### 4.2 Smart Controller replacement gate

Before retiring `plugins/smart-controller`, verify:

1. Native authoring and inspection UI is connected to a versioned document-owned controller model.
2. Stable controller, source, and target identities survive save/close/reopen.
3. The host resolves evaluated source inputs and passes explicit snapshots to the correct core API.
4. Preview is read-only and deterministic; target application uses an explicit, undoable host transaction.
5. Validation, stale references, missing inputs, and migration failures are surfaced as diagnostics.
6. Native Windows build, automated tests, save/reopen tests, undo/redo tests, and manual smoke tests pass.
7. Legacy format migration coverage remains in `smart-controller-core`; it is not discarded with the adapter.

### 4.3 Bone Query disposition gate

Before deleting `plugins/bone-query`, choose and record one of these outcomes:

- Native AniCraft diagnostics provide equivalent inspection and useful fixture coverage is retained; or
- The WASM adapter remains as a separate development-only diagnostic tool with a documented maintenance owner.

It does not need to become part of the production deformation runtime.

## 5. Safe retirement order

1. **Documentation alignment (LEG-01.4):** classify the adapters accurately, correct outdated installation guidance, and publish the parity gates.
2. **Native replacement verification (LEG-01.5):** assess the actual native AniCraft implementation against sections 4.1 and 4.2. Do not infer completion from a UI scaffold or core-only test.
3. **Form Deform adapter retirement:** remove the legacy runtime only after its native parity gate passes; preserve core contracts and relevant regression coverage.
4. **Smart Controller adapter retirement:** remove the transitional command wrapper only after its native parity gate passes; preserve core migration and compatibility behavior.
5. **Bone Query disposition:** migrate its diagnostic value to native tooling or explicitly retain it as a development-only tool.
6. **SDK retirement:** only after all remaining ABI v1 consumers, tests, scripts, and workflows are gone.
7. **Workspace and CI cleanup:** remove retired workspace members, obsolete WASM build jobs, and SDK dependency checks only in the same reviewed change that proves no required consumers remain. Keep native core and host-integration verification.

The order of the Smart Controller and Form Deform adapter removals may change if their native replacement gates are completed at different times. The SDK remains last because all three adapters currently depend on it.

## 6. CI and verification boundary

The current repository workflow contains:

- `cargo test --workspace`
- WASM release builds for Bone Query, Form Deform, and Smart Controller.

These checks are useful for maintaining the retained adapters, but they do not verify:

- AniCraft in-app installation or host response compatibility.
- Native tool authoring and document persistence.
- Undo/redo behavior of native edits.
- Continuous playback or scrubbing evaluation.
- Native Windows GUI smoke tests.

A successful CI run must be cited by its exact commit and run URL. Manual host verification must be recorded separately with the AniCraft commit, test fixture, and observed workflow.

## 7. Documentation ownership and change control

- This document is the canonical disposition and retirement-gate record.
- `plugins/README.md` is the concise adapter directory/status index.
- `plugins/INSTALL.md` describes development-only build and smoke-test procedures.
- `plugins/smart-controller/README.md` describes the active V1 command separately from canonical preview foundations.
- `docs/CAPABILITIES.md` describes Smart Controller capability boundaries.
- `docs/ANICRAFT_SDK_AUDIT.md` remains the source-level audit of the ABI v1 host contract.

No adapter, SDK, workspace member, or runtime behavior is removed by LEG-01.4. Any deletion requires a separate implementation change, explicit review of the affected tests and CI, and the relevant native replacement gate.
