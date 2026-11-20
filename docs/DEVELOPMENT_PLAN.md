# Forma Deform / AniCraft Smart Controller — Development Plan

**Status:** Prototype and integration investigation; not accepted for native integration  
**Authoritative implementation:** Rust workspace  
**Development branch:** `feat/smart-controller-v1`  
**Reviewed baseline:** `72098241fac93c5b4eff86384f9c12fa3a0906a3`

This plan supersedes the original TypeScript/browser-canvas rectangle-editor roadmap previously stored in this file. The former demo has been retired; document geometry and timeline primitives now live in `forma-document-core`. Do not restart the TypeScript prototype unless the architecture is explicitly reconsidered.

## Product and repository boundaries

There are currently two AniCraft plugins:

1. **AniCraft Smart Controller** — one installed plugin with two modes:
   - **Slider:** maps a driver bone's evaluated angle to an action-frame preview.
   - **Deform:** calculates an absolute Asset Warp pin target from authored neutral values; Apply is explicit and undoable.
2. **Bone Query** — separate read-only inspection utility.

The Smart Controller adapter composes independent host-free calculation crates and the generic plugin SDK:

```text
anicraft-smart-controller-plugin
  ├── smart-controller-core
  ├── form-deform-core
  └── anicraft-plugin-sdk
```

- `smart-controller-core` owns Slider mapping, validation, endpoint handling, and the separate three-anchor `ControllerBinding` model.
- `form-deform-core` owns deformation types, bone transforms, weighted mesh deformation, pin references, validation, and evaluation.
- `anicraft-plugin-sdk` owns generic ABI, serialization, allocation, and host communication only.
- `forma-document-core` owns project document geometry and timeline data; it is not Smart Controller binding persistence.
- The plugin adapter owns the manifest, mode dispatch, host DTOs, command calls, and conversion between host data and pure core inputs.

Neither pure core depends on the SDK, AniCraft, or the other core. The former standalone Form Deform plugin is retired; its pure core remains.

## Verified host boundary

The current command-plugin surface supports on-demand bone and warp queries and an undoable `warp.pin_move` edit. It does **not** establish durable arbitrary plugin metadata or document-owned binding persistence, continuous playback/scrubbing evaluation, a verified command for setting a nested symbol instance's displayed frame, or complete live bind-mesh/skin-weight/evaluated bone world-transform data.

Do not invent workarounds or claim these capabilities. Slider is preview-only. Weighted mesh skinning remains a host-independent core foundation until the host supplies complete verified inputs.

---

# Prioritized execution plan

Work strictly in this order. A later milestone must not conceal or bypass a failed earlier gate.

## P0 — Stabilize correctness and CI (current priority)

**Why first:** The latest reviewed Platform and WebAssembly workflows passed, but the Lint workflow failed. A non-green quality gate makes subsequent adapter and host evidence less trustworthy.

### Tasks
1. Fix all Clippy warnings under `cargo clippy --workspace --all-targets -- -D warnings`. Known failures from the last reviewed run:
   - `Vec2::add` and `Vec2::sub`: `should_implement_trait`;
   - nested `format!` inside `write!`: `format_in_format_args`;
   - needless explicit lifetime in `find_bone`.
   Re-check current CI output before assuming this list is exhaustive.
2. Run `cargo fmt --all`, commit intended formatting changes, then require `cargo fmt --all -- --check` to pass. Do not dismiss repository-wide formatting failures as pre-existing.
3. Require native tests and workspace builds on Ubuntu and Windows.
4. Require the WebAssembly workspace build and release builds for both plugin crates.
5. Verify the separate cargo-fuzz package builds. Run bounded smoke sessions for both JSON-payload and mesh-geometry fuzz targets; record commands, duration, and findings. A successful build is not evidence of sustained fuzzing.
6. Re-run all CI workflows and record exact run links and statuses.

### Exit criteria
- Format, Clippy, workspace tests, and workspace builds pass.
- Both plugin WASM release builds pass.
- Fuzz targets compile and bounded smoke runs complete without crashes; discovered issues are fixed and regression-tested.
- No unrelated product or architecture changes are mixed into stabilization.

## P1 — Make the V1 runtime contract explicit

**Why second:** The adapter's actual runtime uses mode-specific DTOs and dispatch: `Params` + `preview_slider()` for Slider, and `DeformParams` + `evaluate_deformation()` for Deform. The three-anchor `ControllerBinding` type exists in `smart-controller-core`, but the plugin runtime does not currently use it.

### Tasks
1. Document actual Slider and Deform input/output contracts, required host fields, validation, defaults, and failure behavior.
2. Resolve the V1 status of `ControllerBinding`: either integrate it into a defined runtime workflow, or mark it as a core-level experimental/future API and exclude it from V1 acceptance. Do not describe it as runtime functionality until the adapter uses it.
3. Align `INTEGRATION.md`, `CAPABILITIES.md`, `SMART_CONTROLLER_PLAN.md`, README, and tests so no fixture implies that a Slider frame preview is exercised by a geometry-deformation binding test.
4. Keep Transform mode deferred until host transform read/write semantics are verified.

### Exit criteria
- One authoritative V1 contract describes what users can configure and what the plugin executes.
- Documentation does not overstate ControllerBinding, timeline driving, persistence, or live mesh deformation.
- Every advertised V1 mode has explicit success and failure behavior.

## P2 — Test the real plugin adapter boundary

**Why third:** Existing `plugin_suite.rs` is a core-composition test. It verifies a synthetic ControllerBinding-to-deformation calculation, not the actual plugin adapter, command dispatch, or Slider frame preview.

### Tasks
1. Add a testable host-command boundary (or equivalent deterministic seam) so tests can provide host responses and capture commands without a running AniCraft process.
2. Exercise actual mode dispatch for Slider and Deform.
3. Test valid, missing, malformed, stale, and wrong-type host responses; missing bones and pins must fail explicitly.
4. Test Slider normalization, endpoint policies, reversed ranges, manual override, and proof that no timeline write is issued.
5. Test Deform calculation-only versus Apply, exact `warp.pin_move` payload, selected pin/element identity, optional spring swing, and command failure propagation.
6. Test repeated evaluation from unchanged authored neutral values to prove no accumulated drift.
7. Test malformed parameters, unknown modes, oversized/malformed ABI payloads, and error reporting.

### Exit criteria
- Tests execute the same adapter dispatch and translation code used by the plugin entry point.
- Tests assert outputs and host commands, including absence of forbidden commands.
- Pure-core tests remain independent and pass.

## P3 — Verify against a pinned AniCraft host

**Why fourth:** Unit and simulated-host tests cannot establish actual host compatibility.

### Tasks
1. Pin the exact AniCraft fork commit, plugin ABI version, build configuration, and matching prototype commit in `INTEGRATION.md`.
2. Create the minimal in-app smoke project described there: one driver bone, one action/timeline, one Asset Warp element, and one existing pin.
3. Verify plugin loading, real `bone.info` and `warp.info` decoding, Slider preview values, Deform absolute target, Apply, and one-step undo.
4. Repeat Deform evaluation and verify no accumulation or unintended edits.
5. Save the project fixture and result log/screenshots; record actual IDs and coordinate precision instead of assuming synthetic values.
6. Update the integration decision record with evidence and host-version limitations.

### Exit criteria
- A reproducible smoke test passes on pinned host and plugin revisions.
- Host behavior, coordinate conventions, ID lifetime, ABI ownership, and undo semantics are evidenced.

## P4 — Decide whether native integration is justified

**Why fifth:** Native integration should follow evidence, not precede it.

### Tasks
1. Evaluate the must-have acceptance criteria and kill criteria in `INTEGRATION.md`.
2. Estimate adapter maintenance cost and any required AniCraft host changes.
3. Confirm an owner for compatibility updates and a bounded upgrade procedure.
4. Decide to proceed, narrow scope, or stop while retaining reusable host-independent cores.

### Exit criteria
- A written decision records evidence, scope, owner, compatibility costs, and unresolved risks.
- No PR or merge is implied by passing prototype gates.

## P5 — Capability expansion (only after host API verification)

Potential future work, not V1 blockers:
- durable document-owned bindings/presets after a supported storage API exists;
- native playback/scrubbing evaluation after a verified lifecycle/event API exists;
- timeline/action-frame driving after a verified command exists;
- Transform mode after host transform reads/writes are documented and tested;
- live mesh deformation only when complete geometry, weights, and evaluated transforms are exposed.

Each capability requires its own host contract review, core/adapter boundary, tests, and updated acceptance criteria. Do not start speculatively.

---

## Current status snapshot

| Area | Status |
|---|---|
| Rust workspace and pure calculation cores | Implemented |
| Unified Smart Controller plugin (Slider + Deform) | Implemented; adapter tests need strengthening |
| Separate Bone Query plugin | Implemented |
| Generic SDK and WASM packaging | Implemented |
| Document geometry/timeline core | Implemented; not binding persistence |
| Mesh/payload resource limits and fuzz targets | Implemented; fuzz execution evidence outstanding |
| Platform and WASM CI | Passed in the previous reviewed run; post-change rerun not yet confirmed |
| Formatting | `cargo fmt --all` applied across the workspace; post-change CI check not yet confirmed |
| Clippy | Three previously reported core warnings corrected; post-change CI check not yet confirmed |
| ControllerBinding runtime integration | Not wired; P1 decision |
| Actual adapter-level integration tests | Incomplete; P2 |
| Pinned-host smoke project | Outstanding; P3 |
| Native integration decision | Not accepted; P4 |
| Durable bindings and continuous playback | Unsupported by currently verified host surface; P5 |

## Verification commands

```sh
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --workspace --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

The fuzz package is a separate cargo-fuzz workspace; use its own manifest and documented targets. Do not add it to the root workspace.
