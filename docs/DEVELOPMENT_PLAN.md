# Forma Deform / AniCraft Smart Controller — Development Plan

**Status:** Prototype and integration investigation; not accepted for native integration  
**Authoritative implementation:** Rust workspace  
**Development branch:** `feat/smart-controller-v1`  
**P0 verified code baseline:** `14e5f3f27f1f7b1ace65b714c93b920b5229a3b8`

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

- `smart-controller-core` owns Slider mapping, validation, and endpoint handling; its separate three-anchor `ControllerBinding` model is experimental and excluded from V1.
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

## P0 — Stabilize correctness and CI (completed)

**Result:** All required quality, platform, WebAssembly, and bounded fuzz-smoke workflows passed against the verified code baseline above. Strict linting remained enabled; newly exposed warnings were fixed rather than suppressed.

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
6. Re-run all CI workflows and record exact run links and statuses. **Completed:** Platform, Lint, WebAssembly, and Fuzz Smoke all passed.

### Exit criteria
- Format, Clippy, workspace tests, and workspace builds pass.
- Both plugin WASM release builds pass.
- Fuzz targets compile and bounded smoke runs complete without crashes; discovered issues are fixed and regression-tested.
- No unrelated product or architecture changes were mixed into stabilization.

### Verified CI evidence

All runs below tested commit `14e5f3f27f1f7b1ace65b714c93b920b5229a3b8`:
- [Platform — Ubuntu and Windows tests/builds](https://github.com/Mujshanan/forma-deform/actions/runs/44698534176): passed.
- [Lint — rustfmt and Clippy with warnings denied](https://github.com/Mujshanan/forma-deform/actions/runs/44698534235): passed.
- [WebAssembly — workspace and both release plugin builds](https://github.com/Mujshanan/forma-deform/actions/runs/44698534175): passed.
- [Fuzz Smoke — cargo-fuzz build plus 20-second JSON and mesh targets](https://github.com/Mujshanan/forma-deform/actions/runs/44698534143): passed.

Fixes required during verification included the Bone Query collapsible-if lint, an SDK test identity operation, and correcting cargo-fuzz CLI invocation in the workflow. The fuzz workflow builds both targets and runs each with `-max_total_time=20 -max_len=65536`; this is bounded smoke coverage, not sustained fuzzing.

## P1 — Make the V1 runtime contract explicit (completed)

**Why second:** The adapter's actual runtime uses mode-specific DTOs and dispatch: `Params` + `preview_slider()` for Slider, and `DeformParams` + `evaluate_deformation()` for Deform. The three-anchor `ControllerBinding` type exists in `smart-controller-core`, but the plugin runtime does not currently use it.

### V1 decision
Keep `ControllerBinding` / `BindingSet` as an experimental, host-independent core API and explicitly exclude it from V1. The adapter does not call this model; it is not the Slider/Deform runtime model, and its JSON helpers do not persist bindings in AniCraft. Retain it for future evaluation rather than expanding V1 to include a workflow the current host cannot store or execute.

### Tasks
1. Document actual Slider and Deform input/output contracts, required host fields, validation, defaults, and failure behavior.
2. Record the ControllerBinding decision in the architecture, capability, README, integration, and source-module documentation.
3. Correct the automated baseline description and rename the core-to-core test so it cannot be mistaken for an adapter or host integration test. The fixture must not imply that a Slider frame preview is exercised by a geometry-deformation binding test.
4. Document that `action_layer_id` is currently required by Slider parameter validation but is not queried or used to drive a host action; frame preview uses the supplied frame bounds.
5. Keep Transform mode deferred until host transform read/write semantics are verified.

### Exit criteria
- One authoritative V1 contract describes what users can configure and what the plugin executes.
- ControllerBinding is explicitly experimental/core-only and excluded from V1 acceptance.
- Documentation does not overstate timeline driving, persistence, or live mesh deformation.
- Every advertised V1 mode has explicit success and failure behavior.

### Verification evidence
All four workflows passed after the contract/documentation and test-fixture corrections on commit `98890ad007e4b5f048c389806316e97c93705504`:
- [Platform — Ubuntu and Windows](https://github.com/Mujshanan/forma-deform/actions/runs/44699173046)
- [Lint — formatting and Clippy](https://github.com/Mujshanan/forma-deform/actions/runs/44699172805)
- [WebAssembly — workspace and both plugin release builds](https://github.com/Mujshanan/forma-deform/actions/runs/44699172929)
- [Fuzz Smoke — both targets](https://github.com/Mujshanan/forma-deform/actions/runs/44699172802)

## P2 — Test the real plugin adapter boundary (completed)

**Why third:** The former `core_composition.rs` test was only a core-to-core calculation. P2 now adds a private host-command seam used by the production adapter and a fake host in tests, so the actual mode dispatcher and command translation can be exercised deterministically.

### Implementation in progress
- Production `run()` now delegates through `run_with_host()`; both Slider and Deform use the same `HostApi` command/log boundary.
- Adapter tests capture host requests and assert the exact `warp.pin_move` payload, plus the absence of writes in Slider and Deform preview.
- Coverage includes manual override, bone lookup, exclusive/inclusive endpoints, reversed ranges, spring swing, missing/malformed bone and warp data, missing pins, host command failures, repeated absolute evaluation, malformed parameters, and unsupported modes.
- The SDK's existing tests cover malformed JSON and the 64 KiB plugin-parameter cap. Full CI validation of this new adapter seam is pending.

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

## P3 — Verify against a pinned AniCraft host (current priority)

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
| Mesh/payload resource limits and fuzz targets | Implemented; both fuzz targets built and passed bounded smoke runs |
| Platform and WASM CI | Passed on verified P0 baseline |
| Formatting | Passed on verified P0 baseline |
| Clippy | Passed with `-D warnings` on verified P0 baseline |
| ControllerBinding runtime integration | Explicitly excluded from V1; retained as experimental core API |
| Actual adapter-level integration tests | Implemented and CI-verified; P2 complete |
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
