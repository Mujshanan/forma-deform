# Integration decision record

**Status:** Prototype / integration investigation  
**Decision:** Not accepted for native integration  
**Owner:** Maintainer  
**Last reviewed:** 2026-11-21

This document is the maintainer's review entry point. The prototype demonstrates isolated calculations and a command-plugin adapter; it is not evidence that AniCraft currently supports a complete animation-controller lifecycle.

## 1. Target fork and pinned revisions

| Item | Current reference | Meaning |
|---|---|---|
| Target host repository | [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft) | Intended host fork |
| Target host commit (main head rechecked 2026-11-21) | [`bcfea816478f2a00ab172cefe67f0bd5696b3886`](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886) | Pinned P3 candidate; source reviewed, in-app smoke still pending |
| Native-tool work branch | [`feat/smart-controllers-tool`](https://github.com/Mujshanan/anicraft/tree/feat/smart-controllers-tool) at `b001b70e1444c3b194122f859b22b95cb098a22b` | Branch based on the existing three-commit native controller prototype; no native-tool changes merged |
| Prototype repository | [Mujshanan/forma-deform](https://github.com/Mujshanan/forma-deform) | This prototype and reusable calculation cores |
| Current plugin-fix branch | `fix/smart-controller-manifest-ids` | Manifest schema and stable driver-bone identity corrections; validation pending |
| Previous verified prototype baseline | [`0ed13f243068392d4a8c649fff5f091f022fe1e3`](https://github.com/Mujshanan/forma-deform/commit/0ed13f243068392d4a8c649fff5f091f022fe1e3) | Host-response compatibility fix; P0/P2 CI verified before the current manifest/ID corrections |
| Vendored AniCraft guest SDK source | [`bcfea816478f2a00ab172cefe67f0bd5696b3886`](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886) | SDK source pin recorded in `crates/anicraft-plugin-sdk/UPSTREAM.md`; independently checked against host `main` head, which happens to be the same SHA |
| Host plugin ABI | v1 | Confirmed in `docs/plugins.md` at the pinned host commit |
| Host build baseline | Rust 1.95 minimum; Cargo edition 2024 | `cargo run` from the pinned AniCraft repository root |
| Plugin build configuration | Rust stable; `wasm32-unknown-unknown`; release | Build the Smart Controller and Bone Query plugin crates using the repository's CI commands |

The host revision, plugin ABI, and prototype baseline are now pinned for P3. The SDK source pin and host main-head revision are recorded as separate facts even though they currently resolve to the same SHA. Source inspection is not a substitute for running the pinned host application.

## 2. Smoke-test baseline

### 2.1 Deterministic calculation baseline (automated)

The current automated fixture is `plugins/smart-controller/tests/core_composition.rs`. It is a **core-to-core composition test only**: it does not invoke the Smart Controller adapter, dispatch either plugin mode, issue an AniCraft command, or verify a host response. The synthetic JSON resembles a small subset of `bone.info`; the test extracts its keyed angle for the experimental three-anchor `ControllerBinding` evaluator. A separate `DeformRequest` then uses explicitly supplied test values.

**Fixture inputs**

- Bone ID: `17`
- Evaluated keyed angle: `22.5°`
- Spring swing present in the synthetic fixture: `2.0°` (not used by the separate DeformRequest; spring contribution is disabled)
- Experimental `ControllerBinding` input range: `-45°` to `+45°`, neutral `0°`
- Linear relative-delta geometry target:
  - negative offset: `(-12, 4)`
  - neutral offset: `(0, 0)`
  - positive offset: `(12, -4)`
- Authored neutral pin position: `(120, 80)`
- Deform gain: `(0.5, -0.25)` stage pixels per degree
- Deform keyed angle: `10°`; spring contribution disabled

**Expected values**

| Check | Expected |
|---|---:|
| Experimental binding normalized input | `0.5` |
| Experimental binding geometry offset | `(6, -2)` |
| Deformation base position after offset | `(126, 78)` |
| Effective deformation angle | `10°` |
| Absolute target pin position | `(131, 75.5)` |
| Repeated evaluation | Exactly the same result for the same inputs |

The test asserts the experimental binding's geometry offset, the separate deformation core's absolute target, and repeatability. It does **not** test Slider frame preview, plugin mode dispatch, DTO translation, host commands, or adapter behavior. Slider mapping has separate pure-core tests. This fixture is **not** an adapter integration test, real AniCraft project, or proof of host behavior.

### 2.2 Required in-app host smoke project (base fixture identified; execution outstanding)

Use the pinned host's existing fixture as the starting point:

- Source: [`crates/engine/tests/fixtures/armature-v3-shape.anicraft`](https://github.com/Mujshanan/anicraft/blob/bcfea816478f2a00ab172cefe67f0bd5696b3886/crates/engine/tests/fixtures/armature-v3-shape.anicraft)
- Host revision: `bcfea816478f2a00ab172cefe67f0bd5696b3886`
- Stage: 400 × 300 px at 24 fps
- Armature layer: `102`; bones: `100` and `103`
- Normal layer: `1` (empty in the source fixture)
- Existing armature shape element: `10`

Prepare the actual smoke project in AniCraft:

1. Open the pinned fixture. Confirm bone ID `100`, its keyed angle, and its spring swing with `bone.info` using armature layer `102`. Slider driver selection uses the stable bone ID, not the editable bone name.
2. On normal layer `1`, draw a separate rectangle. Add an Asset Warp mesh and one pin to it. Record the actual element ID, pin ID, and the pin's current stage-pixel `at` position from `warp.info`. Do not reuse the synthetic IDs from the core test.
3. Build the Smart Controller module with `cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin`, then install that exact WASM module in the pinned host.
4. Slider check: use driver layer `102`, driver bone ID `100`, action layer `1`, rotation range `-45°…+45°`, and frame range `0…20`. Keep **Use manual driver angle** off. Set the bone to `22.5°`; expect normalized input `0.75` and Exclusive preview frame `15`. Confirm the plugin only logs a preview and does not change the timeline. The action-layer ID remains preview context only; this V1 plugin does not drive the timeline.
5. Deform check: use bone ID `100`, the recorded element/pin IDs, the actual pin `at` position as the neutral position, neutral angle `0°`, gains `(0.5, -0.25)` px/degree, and spring swing disabled. At a keyed bone angle of `10°`, expect the absolute pin target to be `(neutral_x + 5, neutral_y - 2.5)` px. Preview first, then Apply.
6. Confirm the pin moves to the reported target, one Undo restores its original position, and repeating evaluation from the same authored neutral values yields the same target without drift.
7. Save the modified project as `smart-controller-smoke.anicraft`; record the actual bone/element/pin IDs, host-returned values, plugin logs, and a screenshot.

This base fixture does not already contain an Asset Warp mesh. The added rectangle and pin, actual IDs, and result log must be produced by the in-app run. The source fixture and static host-source review do not count as a passed smoke test.

## 3. Acceptance criteria

### Must-have for an integration decision

- [x] The target AniCraft commit and plugin ABI are pinned and reproducible.
- [ ] Plugin loads and executes on the target host without crashing or corrupting the project.
- [ ] Bone and Asset Warp responses are decoded from the real host; missing, malformed, or stale references fail explicitly rather than silently defaulting.
- [ ] Slider produces the documented deterministic frame preview, including endpoint and reversed-range behavior, without claiming to drive the timeline.
- [ ] Deform computes an absolute target from authored neutral values; optional spring swing is used only when explicitly enabled and actually available.
- [ ] Explicit Apply changes only the selected existing Asset Warp pin, is undoable, and does not accumulate drift across repeated evaluations.
- [ ] Coordinate conventions, units, ID lifetime, error behavior, and ABI allocation/ownership rules are documented and verified.
- [x] Core tests, adapter tests, strict lint, WASM builds, and bounded fuzz smoke runs pass on the pinned prototype revision.
- [ ] The real-host smoke project passes on the pinned AniCraft revision.
- [ ] The integration has a clear maintenance owner and a bounded compatibility/update procedure.

### Native-tool wishlist (not a V1 blocker)

These are desirable host-native capabilities, not promises of the current plugin:

- Continuous evaluation during playback and scrubbing.
- Durable plugin-owned bindings/presets stored in the AniCraft document.
- Native timeline/action-frame driving and nested-symbol displayed-frame control.
- Complete bind-mesh, skin-weight, and evaluated world-transform reads for live mesh deformation.
- Native bone/property write APIs and richer transform constraints.
- A first-class lifecycle/event API instead of on-demand command execution.

Do not implement host-specific workarounds that pretend these capabilities already exist. Reassess the wishlist only against a verified host API contract.

## 4. Kill criteria

Stop the integration effort (while retaining reusable host-independent cores) if any of the following is true:

- The target host cannot provide stable, usable bone and Asset Warp pin references for the must-have workflow.
- The plugin ABI or host command surface cannot support safe, bounded payload exchange and explicit undoable pin edits.
- The real-host smoke project cannot pass the baseline after a bounded debugging effort, or results depend on undocumented host state.
- Correct behavior requires fabricating geometry, weights, transforms, IDs, or persistence semantics that the host does not expose.
- The only way to deliver the advertised core workflow is to claim continuous playback, durable bindings, timeline driving, or live mesh deformation that is not actually supported.
- Upstream changes make the adapter a permanently invasive fork with no credible maintenance path, or no maintainer can own compatibility updates.
- The must-have scope cannot be delivered without compromising project integrity, undo behavior, deterministic calculations, or the documented memory/ABI safety contract.

A wishlist item being unavailable is not, by itself, a kill condition. It becomes one only if the product's agreed must-have workflow depends on it.

## 5. Current evidence and next decision

- On 2026-11-21, branch `fix/smart-controller-manifest-ids` was created from `feat/smart-controller-v1`. It changes unsupported manifest string parameters to ABI v1 choices, supplies required numeric bounds/defaults, adds an explicit manual-input toggle, and resolves Slider drivers by stable bone ID. The current branch has not yet been validated by a completed CI run or in-app smoke test.
- Bone Query's current response-field mapping was compared with `bone.info` at the pinned host commit; its fields already match the host response, so no speculative adapter rewrite was made.

- Pure Rust cores and the actual Smart Controller `run_with_host` mode dispatcher have automated tests; a fake host captures the same command/log boundary used by production `run()`. Coverage includes Slider manual/bone input and endpoint behavior, Deform preview/Apply and exact `warp.pin_move` payload, optional spring swing, malformed/missing host data, command failures, and repeated absolute evaluation. SDK tests cover malformed JSON and oversized plugin parameters. See `cargo test --workspace`.
- Static source review at host commit `bcfea816478f2a00ab172cefe67f0bd5696b3886` confirms ABI v1; `plugin_may_call` permits the `bone.*` and `warp.*` command namespaces; `bone.info` accepts a layer filter and returns bone IDs, names, keyed angles, and spring swing; `warp.info` returns `warped:false` without a `pins` field or, when warped, a pin list containing IDs and stage-pixel positions; and `warp.pin_move` accepts element/pin IDs and stage-pixel X/Y coordinates.
- The host's command-plugin contract makes a successful `plugin.run` one undo step. This is confirmed from source/docs, not yet from the in-app smoke run.
- The adapter was corrected to match the actual `warp.info` response variants: absent pins are accepted for an unwarped element, but are an error when `warped:true`.
- The prototype baseline commit `0ed13f243068392d4a8c649fff5f091f022fe1e3` passed all four workflows: [Platform](https://github.com/Mujshanan/forma-deform/actions/runs/44700916871), [Lint](https://github.com/Mujshanan/forma-deform/actions/runs/44700916955), [WebAssembly](https://github.com/Mujshanan/forma-deform/actions/runs/44700916909), and [Fuzz Smoke](https://github.com/Mujshanan/forma-deform/actions/runs/44700916897).
- The current host contract does **not** establish continuous playback/scrubbing evaluation, durable arbitrary plugin metadata, or a verified command to set a nested symbol instance's displayed frame.
- Weighted mesh skinning exists as a host-independent core foundation. It is not wired to live AniCraft mesh geometry because the required complete geometry, skin weights, and evaluated bone world poses are not exposed by the inspected host responses.
- The exact target host commit and ABI are pinned; the in-app smoke project has a pinned base fixture and reproducible preparation steps but has not yet been executed.

**Next decision gate:** run the prepared smoke project in the pinned AniCraft build, record actual host IDs/results, and verify Apply/Undo and repeatability (P3). The V1 contract, adapter tests, host commit pin, and static API review are complete; do not request native integration review until the in-app evidence is recorded.
