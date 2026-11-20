# Integration decision record

**Status:** Prototype / integration investigation  
**Decision:** Not accepted for native integration  
**Owner:** Maintainer  
**Last reviewed:** 2026-11-20

This document is the maintainer's review entry point. The prototype demonstrates isolated calculations and a command-plugin adapter; it is not evidence that AniCraft currently supports a complete animation-controller lifecycle.

## 1. Target fork and pinned revisions

| Item | Current reference | Meaning |
|---|---|---|
| Target host repository | [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft) | Intended host fork |
| Target host commit for an integration run | **TBD — pin before host smoke testing or proposing a merge** | Do not substitute the moving default branch |
| Prototype repository | [Mujshanan/forma-deform](https://github.com/Mujshanan/forma-deform) | This prototype and reusable calculation cores |
| Prototype branch | `feat/smart-controller-v1` | Current development branch |
| Prototype baseline commit | [`ed7e8cd84e4b365bfd912edeac5f1682842116fd`](https://github.com/Mujshanan/forma-deform/commit/ed7e8cd84e4b365bfd912edeac5f1682842116fd) | Exact source revision reviewed for this record |
| Vendored AniCraft guest SDK source | [`bcfea816478f2a00ab172cefe67f0bd5696b3886`](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886) | SDK source pin recorded in `crates/anicraft-plugin-sdk/UPSTREAM.md`; **not** the target host commit |

Before any host integration test, record the exact AniCraft commit SHA, build configuration, plugin ABI version, and prototype commit SHA used together. The SDK source pin must not be misrepresented as the host fork revision.

## 2. Smoke-test baseline

### 2.1 Deterministic calculation baseline (automated)

The current reproducible baseline is the integration test in `plugins/smart-controller/tests/plugin_suite.rs`. It uses a small synthetic host response with the same stable-ID, keyed-angle, and spring-swing fields expected from `bone.info`.

**Fixture inputs**

- Bone ID: `17`
- Evaluated keyed angle: `22.5°`
- Spring swing reported: `2.0°`
- Slider range: `-45°` to `+45°`, neutral `0°`
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
| Slider/controller normalized input | `0.75` |
| Controller geometry offset | `(6, -2)` |
| Deformation base position after offset | `(126, 78)` |
| Effective deformation angle | `10°` |
| Absolute target pin position | `(131, 75.5)` |
| Repeated evaluation | Exactly the same result for the same inputs |

The test asserts the geometry offset, absolute target, and repeatability. This is a calculation/adapter-composition baseline, **not** a real AniCraft project or proof of host behavior.

### 2.2 Required in-app host smoke project (not yet checked in)

Before accepting native integration, create and retain a minimal AniCraft project fixture containing:

1. One armature with a single named driver bone, stable bone ID `17`, and a controllable keyed rotation.
2. One action/timeline with a known start/end frame range for Slider mode.
3. One Asset Warp element with a single existing pin, stable element/pin IDs recorded in the fixture notes.
4. A neutral pose and authored pin position of `(120, 80)` stage pixels.
5. A documented host build commit and exact steps to load/run the plugin.

Record the actual host-returned values and compare them with the deterministic baseline above. For the Deform Apply check, verify that the target is absolute (not accumulated), that the pin reaches the expected target within the host's documented coordinate precision, and that one undo restores the original pin position. Save the fixture project and a short result log/screenshot alongside the integration evidence; do not invent host IDs or claim this test has passed before running it.

## 3. Acceptance criteria

### Must-have for an integration decision

- [ ] The target AniCraft commit and plugin ABI are pinned and reproducible.
- [ ] Plugin loads and executes on the target host without crashing or corrupting the project.
- [ ] Bone and Asset Warp responses are decoded from the real host; missing, malformed, or stale references fail explicitly rather than silently defaulting.
- [ ] Slider produces the documented deterministic frame preview, including endpoint and reversed-range behavior, without claiming to drive the timeline.
- [ ] Deform computes an absolute target from authored neutral values; optional spring swing is used only when explicitly enabled and actually available.
- [ ] Explicit Apply changes only the selected existing Asset Warp pin, is undoable, and does not accumulate drift across repeated evaluations.
- [ ] Coordinate conventions, units, ID lifetime, error behavior, and ABI allocation/ownership rules are documented and verified.
- [ ] Core tests, plugin tests, WASM build, and the real-host smoke project pass on the pinned revisions.
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

- The pure Rust cores and command adapter have automated tests; see `cargo test --workspace`.
- The adapter currently performs on-demand bone and warp queries. Deform Apply uses the undoable `warp.pin_move` command.
- The current host contract does **not** establish continuous playback/scrubbing evaluation, durable arbitrary plugin metadata, or a verified command to set a nested symbol instance's displayed frame.
- Weighted mesh skinning exists as a host-independent core foundation. It is not wired to live AniCraft mesh geometry because the required complete geometry, skin weights, and evaluated bone world poses are not exposed by the inspected host responses.
- The in-app smoke project and exact target host commit are still outstanding.

**Next decision gate:** pin the AniCraft host commit, build the minimal smoke project, run the real-host checks, and update this record with evidence before requesting native integration review.
