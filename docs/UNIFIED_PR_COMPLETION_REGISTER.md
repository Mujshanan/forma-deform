# Unified PR Completion Register

**Repository:** `Mujshanan/forma-deform`  
**Audit date:** 2026-11-23  
**Canonical integration target:** `main`  
**New consolidation branch:** `consolidation/final-pr-register-2026-11-23`  
**Purpose:** Replace fragmented PR review with one auditable register and one canonical integration review.

> **Important:** This file is an audit and work register, not proof that every feature is complete. PR descriptions are author-reported scope and CI claims; they are not independently re-run by this register. A PR being merged into a feature branch does not mean it is in `main`. All previously open PRs have now been administratively closed without merging after this replacement PR was created. Their head branches remain preserved; see the verified branch inventory and recovery matrix below.

## 1. Non-negotiable project gates

1. **S0 remains OPEN.** No S0 closure is inferred from documentation, source inspection, or this consolidation.
2. Native AniCraft mutation-boundary/runtime implementation remains blocked until formal S0 closure and separate explicit implementation authorization.
3. CCC-01.5 remains in force: Smart Controller binding evaluation and Form Deform pin evaluation are independent. No implicit cross-engine composition.
4. Native Asset Warp remains AniCraft-owned; FDM must not silently replace it.
5. Preserve Windows as a required release target.
6. Keep authored state, evaluation snapshots, freshness/identity, and host mutation/undo ownership explicit.
7. No tests are described as passing unless a matching exact-head CI result or reproducible local run exists.
8. Closing an old PR is administrative only. It does not merge its code, transfer its commits, or prove its requirements complete.

## 2. PR-by-PR register

The register now covers PRs #1–#85. PRs #84 and #85 are recorded in the verified post-register update section below.

Status is the GitHub PR state at audit time. “Merged” means merged to that PR's stated base, not necessarily to `main`. Remaining work below is the acceptance work still required before the contribution can be considered integrated and complete.

### Foundation, Smart Controller and CCC track

| PR | State / contribution | Missing work, tests, or disposition |
|---:|---|---|
| #1 | Merged to main — rectangle deformation MVP | Historical prototype, not the current native product. Keep only as provenance; do not treat the Canvas demo as production FDM. |
| #2 | Merged to main — Smart Controller workspace/plugin foundation | V1 remains preview-only. Reconcile legacy adapter status and confirm current workspace/WASM verification on the final integration head. |
| #3 | Closed, unmerged — MCP global-playhead Slider prototype | Experimental global playhead, not nested-symbol/local-frame Slider. Live AniCraft desktop bridge, repeated live updates, latency evidence and UI recording remain outstanding; retain as experimental or park. |
| #4 | Closed, unmerged — shared geometry primitives | Verify current-head workspace tests and all three release WASM builds; confirm no public-path/serde compatibility regressions. |
| #5 | Closed, unmerged — canonical controller binding consolidation | Verify schema-v1 round trips, type-path identity, canonical evaluation, workspace tests and WASM builds on the integrated head. |
| #6 | Closed, unmerged — independent Smart Controller/FDM orchestration | Preserve separate outputs/errors. Verify omitted/disabled requests and failure isolation; no implicit composition. |
| #7 | Closed, unmerged — adapter isolation | Verify dependency-boundary script and CI on final tree; inspect all reusable crates and adapters for forbidden dependency direction. |
| #8 | Closed, unmerged — compatibility verification | Re-run edge-case tests and document remaining host runtime, undo, mutation and ABI/security limitations. Formatting is a separate gate. |
| #9 | Merged into CCC feature branch (not main) — FDM v2 readiness assessment | Historical assessment; ADM-001–007 were unresolved in that assessment. Reconcile against current decisions before using as authority. |
| #10 | Merged to main — Smart Controller Phase 1 foundation | Historical CI reported Rust/Bun/WASM passes; rustfmt and Clippy were not covered. Re-run on final head. |
| #11 | Merged to main — unified evaluator and read-only host preview | Mapper was an injected prerequisite at the time; later work supplies parts of it. Confirm current integration and host smoke test; no playback/persistence/apply is implied. |
| #12 | Closed, unmerged — canonical development guide/proposal | Reconcile with current project scope and newer S0/FDM-06 evidence; remove superseded roadmap statements only after preserving decisions. |
| #13 | Closed, unmerged — AniCraft native-tool feasibility audit | Source baseline is historical. Refresh against pinned/current host SHA and reconcile native UI, persistence, evaluator and command capabilities. |
| #14 | Closed, unmerged — Smart Controller core characterization/consolidation | Verify parity/error-contract tests and determine whether its changes are already superseded by merged #18 or other main commits. Preserve unique findings only. |
| #15 | Closed, unmerged — native authoring/symbol workflow specification | Reconcile five controller families and incomplete PoseSet1D/JoystickFivePose/Discrete semantics with current canonical schema and host APIs; tests are specified, not implementation. |
| #16 | Closed, unmerged — native Smart Controller/Graphic symbol authoring design | Reconcile with #15 and host branch state; verify symbol identity/instance semantics, commands, undo and acceptance tests before retaining as normative. |
| #17 | Closed, unmerged — earlier canonical mapper | Superseded by #18. No action unless unique commits/content are found. |
| #18 | Merged to main — canonical Smart Controller mapper | Confirm exact current-main presence and tests; PoseLibrary persistence and JoystickFivePose limitations were explicitly outside scope at the time. |
| #19 | Closed, unmerged — CCC-01.5 separation correction | Verify the independent preview regression test and removal of the unused cross-crate dependency on current main; CI required. Preserve separation. |
| #20 | Closed, unmerged — stable target identity/geometry snapshot design | Reconcile with later FDM M1 contracts (#30–37) and FDM-06 identity/freshness findings; avoid duplicate or contradictory identity models. |

### Smart Controller host authoring and M-series work

| PR | State / contribution | Missing work, tests, or disposition |
|---:|---|---|
| #21 | Closed, unmerged — M2 canonical AniCraft preview | PR description reports Rust/Bun/WASM CI pass, but host manifest compatibility failed: string manifest fields unsupported. Confirm correction/replacement and host manifest compatibility before acceptance. Preview remains read-only; no persistence, setter, playback or keyframe write. |
| #22 | Closed, unmerged — M3 native settings-dialog authoring | Verify host-supported manifest types, dialog constraints, canonical document construction and preview reuse. Confirm exact-head CI and real host smoke test. |
| #23 | Closed, unmerged — M4 versioned persistent payload | Core envelope tests exist by description, but durable save/load, AniCraft-owned extension, commands and undo/redo are explicitly not implemented. Host-side persistence is a blocker. |
| #24 | Closed, unmerged — M5 current-frame apply/range bake | PR reports CI run #632 passed and bounded preflight/transactional writes. Independently verify exact head, host command compatibility, failure rollback, stale recheck, undo behavior and documented limits. Do not treat range bake as playback evaluation. |
| #25 | Closed, unmerged — M3A command authoring | PR reports CI passed at a specific head. Re-run exact-head checks; host smoke test for frame selection, read-only preview, Form Deform apply opt-in and consistent evaluation frame. |
| #26 | Closed, unmerged — M3B host extension requirements | Requirements only. Reconcile against actual AniCraft API/host changes; dockable panels, overlays, persistence and transactions remain host-owned implementation work. |
| #27 | Closed, unmerged — M7 five-pose joystick mapping | Verify deterministic anchor preservation, clamping, angle/scalar/vector blending and invalid/ambiguous cases. Pose persistence, live playback and multi-controller composition remain out of scope. |
| #28 | Closed, unmerged — M6 native host evaluation contract | Design only. Host per-frame evaluator hook was not exposed in the audited SDK. Confirm current host API; implementation and in-app playback proof remain absent. |
| #29 | Closed, unmerged — schema v2/PoseGrid | PR reports CI #673 passed; representative performance benchmarks remain pending. Verify migration compatibility, bounds/cap, non-uniform axes, wrapped angles and current API integration. |
| #30 | Closed, unmerged — M1 evaluation lifecycle contracts | Verify exact-head core tests and reconcile stamps/fingerprints/coordinate rules with later FDM-01 and FDM-06 contracts. No host evaluator or apply path is included. |
| #31 | Closed, unmerged — persistence schema/composition graph validation | Verify JSON bounds, unknown-field/version behavior, graph signatures/cardinality/cycle checks and compatibility. No runtime host persistence is implied. |
| #32 | Closed, unmerged — composition cardinality/planning | Verify deterministic topological ordering, all invalid cardinality/edge/cycle cases and integration with later evaluator. Planning is not operator execution. |
| #33 | Closed, unmerged — canonical geometry contract integration | Verify borrowed snapshot identity, coordinate-space exactness and Path2D preservation. No evaluator or host mutation is included. |
| #34 | Closed, unmerged — operator invocation contracts | CI was pending in PR description. Require exact-head tests for invalid definitions, identity/version mismatch, coordinate mismatch and error classification. |
| #35 | Closed, unmerged — single-handle displacement operator | CI was pending in PR description. Verify deterministic falloff, Bézier control-point policy, immutability, finite inputs and Path2D topology preservation. Reference operator only; not native Asset Warp. |
| #36 | Closed, unmerged — stateless single-operator evaluation engine | Verify freshness checks, deterministic diagnostics, fingerprint validation and request/result binding. Scheduler, graph evaluation, cache, apply, undo and bake remain out of scope. |
| #37 | Closed, unmerged — global cage embedding validation | Verify all manifold, intersection, winding, degeneracy and vertex-link adversarial cases. Revalidate after topology changes; host binding/evaluation integration is separate. |

### Quality, FDM-03/04/05 and formatting work

| PR | State / contribution | Missing work, tests, or disposition |
|---:|---|---|
| #38 | Merged into quality branch — controller validation refactor | Not necessarily in main. Verify ancestry and behavior-preservation regression tests before cherry-pick/integration. |
| #39 | Closed, unmerged — weighted composition refactor | PR reports exact-head fmt/tests/WASM checks; independently verify checks and error/order/float accumulation parity. |
| #40 | Closed, unmerged — cage binding and stale protection | PR reports successful CI/Clippy/tests. Verify current-head persistence/migration and adversarial stale target/topology/fingerprint tests; reconcile with later FDM-03/04 work. |
| #41 | Closed, unmerged — canonical DAG evaluator decomposition | Verify behavior-preserving tests for graph order, diagnostics, unavailable inputs, mapper failures, contributions and stable sort; exact-head CI required. |
| #42 | Closed, unmerged — native AniCraft integration traceability docs | Historical host branch/UI evidence only; native UI scaffolding was not merged to AniCraft main. Refresh host SHA and distinguish implemented from transient/unmerged host work. |
| #43 | Closed, unmerged — Rust formatting CI | PR reports CI pass. Verify the workflow is present and required on canonical main, and that it runs `cargo fmt --all -- --check`. |
| #44 | Closed, unmerged — non-blocking Clippy report | Verify artifact retention/report behavior and avoid confusing report-only status with a blocking clean lint gate. |
| #45 | Closed, unmerged — stable rustfmt sweep | PR reports 47-file formatting-only CI pass. Verify exact diff remains formatting-only and reconcile overlap with the separate 69-file FDM-FMT baseline. |
| #46 | Closed, unmerged — FDM-01.5 regression/quality gates | Verify full Path2D topology, immutable input, operation distinction and stale-source tests. No scheduler/cache/host apply/undo/bake is included. |
| #47 | Closed, unmerged — FDM-03 integration regressions | Verify persistent binding round-trip/revalidation, stale source/cage rejection, topology identity, adapter persistence and adaptive samples. Reconcile its two-parent lineage and current base. |
| #48 | Merged into FDM-03 feature branch (not main) — FDM-04.1 evaluator | Not necessarily in main. Verify current branch ancestry and tests for mapped/clamped/extrapolated/unbound features, deterministic output and stale dependencies. |
| #49 | Closed, unmerged — legacy adapter retirement status | Documentation only. Verify native parity before retiring Form Deform adapter; retain Bone Query diagnostic tooling and retire vendored SDK last as documented. |
| #50 | Merged into FDM-03 feature branch (not main) — FDM-04.2 adaptive curve sampling | Not necessarily in main. Verify deterministic sample regeneration, stamped policy, stale/tamper/topology rejection and Path2D non-mutation. |
| #51 | Merged into FDM-04.2 feature branch (not main) — FDM-04.3 numerical regressions | Not necessarily in main. Verify exact tests and numerical boundary coverage on canonical integration head. |
| #52 | Merged into legacy-retirement feature branch (not main) — Bun demo retirement/Rust verification repair | Not necessarily in main. Verify the removed demo is truly obsolete, no active tooling/docs depend on it, and Rust verification is preserved before integration. |
| #53 | Closed, unmerged draft — earlier FDM-04.4 benchmark/gates | Superseded by #54; compare for unique content, then close as superseded after preservation. |
| #54 | Merged into FDM-04.3 feature branch (not main) — FDM-04.4 benchmark/gates | Not necessarily in main. Verify benchmark stability, thresholds, CI integration and exact-head evidence. |
| #55 | Closed unmerged — earlier FDM-04.5 hardening | Superseded by #56. Check unique content before final archival; do not revive wholesale. |
| #56 | Merged into FDM-03 feature branch (not main) — FDM-04.5 clean lineage | Not necessarily in main. Verify it incorporates required #54/#51/#50/#48 content and passes integration tests. |
| #57 | Closed, unmerged — FDM-05.1/05.2 paired output/output-space refinement | Verify output pairing, coordinate-space compatibility, replacement vs independent contribution semantics and tests. Stacked on #47. |
| #58 | Closed, unmerged — FDM-05.3 structured/derived output assembly | Verify constructors, derived outputs, cardinality and deterministic assembly. Stacked on #57; exact-head CI required. |
| #59 | Closed, unmerged — FDM-05.3 constructor regression tests | Verify the tests exercise public constructors and malformed/edge inputs; retain with #58 only if non-duplicate and green. |
| #60 | Closed, unmerged — FDM-05.4 integration readiness/output contract | Verify end-to-end output contract, stale/freshness and host boundary. This is readiness hardening, not native host implementation. |

### Maintenance, FDM-06 and S0 evidence chain

| PR | State / contribution | Missing work, tests, or disposition |
|---:|---|---|
| #61 | Closed, unmerged — Clippy argument-count cleanup for #47 | PR reports Clippy clean at stated head. Verify no crate-wide suppression remains, exact head, and no API/behavior regression. |
| #62 | Closed, unmerged — workspace formatting/integration ownership audit | PR description reports 71/74 files fail formatting and stops before Clippy/tests/bench/WASM. This is an audit, not a passing maintenance change. |
| #63 | Closed, unmerged — Path2D persistence CAS | Verify atomic slot revision + geometry stamp compare-and-swap, tombstone monotonicity, conflict propagation and concurrency/stale-write tests. |
| #64 | Closed, unmerged — FDM-06.2 native integration contract | Design only. Reconcile with later FDM-06.6–06.13 evidence and keep host responsibilities explicit. |
| #65 | Closed, unmerged — FDM-06.3 host conformance plan | Competes with #71. Compare source evidence/test matrix and preserve unique material in one canonical record. |
| #66 | Closed, unmerged — FDM-06.4 host extension/target identity | Reconcile proposed DocumentId/TargetRef lifecycle with later host source audit and FDM-06.8 state ownership. Design is not implementation. |
| #67 | Closed, unmerged — FDM-06.5 target identity/revision audit | Verify all source references against pinned AniCraft SHA and reconcile frame/edit context, incarnation and revision ownership with #74/#75/#80. |
| #68 | Closed, unmerged — FDM-FMT-01 baseline | Baseline only: 69 formatting-diff files, six overlap #61, #63 CAS files excluded. No formatting sweep completed by this PR. |
| #69 | Closed, unmerged — FDM-06.6 mutation invalidation contract | Competes with #74. Compare contracts, tests and later source evidence; preserve one authoritative version. Runtime implementation is gated. |
| #70 | Closed, unmerged — FDM-06.7 implementation-readiness review | Decision: not ready for runtime implementation. Public mutable Session/Document APIs, settlement side effects, identity ledger and atomic publication remain blockers. Proposed tests are not executed tests. |
| #71 | Closed, unmerged — alternate FDM-06.3 host conformance plan | Competes with #65. Reconcile against the pinned AniCraft baseline; preserve only unique evidence and test cases. |
| #72 | Closed, unmerged — FDM-06.8 mutation boundary/state ownership spec | Design-only. Verify internal consistency with #73–80; no host implementation or conformance pass is claimed. |
| #73 | Closed, unmerged — FDM-06.9 host transaction API/persistent-state plan | Planning only. File-by-file migration, transaction, identity, persistence and conformance plan still needs implementation after S0/authorization gates. |
| #74 | Closed, unmerged — alternate FDM-06.6 host mutation invalidation contract | Competes with #69. Compare and reconcile; do not retain parallel normative contracts. |
| #75 | Closed, unmerged — FDM-06.10 host baseline/mutation inventory | Read-only audit. Verify pinned source SHA/tree and source citations remain current; does not close callback/consumer/CI evidence gaps. |
| #76 | Closed, unmerged — FDM-06.11 S0 registry/ownership assessment | Explicitly not S0 closure. Command registry seed does not replace callback-level transitive effects, compile-backed consumer coverage or host CI. Compare with #77. |
| #77 | Closed, unmerged — alternate FDM-06.11 registry/ownership contract | Competes with #76. Preserve unique inventory/effect mapping only; no S0 closure or implementation authorization. |
| #78 | Closed, unmerged — FDM-06.12 gated mutation-boundary implementation plan | Planning only. Reconcile with #79/#80 and preserve gate order; no runtime code or host tests. |
| #79 | Closed, unmerged — FDM-06.12 workspace consumer/host baseline map | Reports 395 command IDs and consumer/test ownership map; callback transitive effects and compiler-backed consumer verification remain incomplete. Host PR/CI baseline unavailable at report time. |
| #80 | Closed, unmerged — FDM-06.13 S0 closure-review evidence addendum | Latest evidence submission, **NOT CLOSED**. First-pass effect graph is incomplete; 36 callback manifest reconciliation and transitive effects, compiler-backed consumer verification, and successful pinned-host CI remain outstanding. Retain as current S0 evidence base, then complete gaps and submit for formal review. |
| #81 | Closed, unmerged — README/DEVELOPMENT status refresh | Docs only. Reconcile after the final PR plan is accepted; do not merge stale status before canonical integration is decided. |
| #82 | Closed, unmerged — earlier repository consolidation plan | Superseded by this unified register. Preserve any unique branch inventory details; close after this replacement PR is established. |

## 3. Canonical workstreams and acceptance gates

### A. S0 — audit evidence only (BLOCKING)

Required before any native mutation-boundary implementation:
- Reconcile the original 36-row inline-callback manifest against source locations.
- Complete every callback's transitive call chain and effect sinks; unresolved edges must be explicitly recorded.
- Complete the effect graph for all 95 named callbacks and reconcile the 131 `mutates=false` command set, including nested execution/settlement paths.
- Compiler-backed verification of all relevant public API consumers and non-command mutation paths.
- Successful host CI on the exact pinned AniCraft baseline, with exact commit SHA and check/run evidence.
- Formal evidence review and explicit S0 closure decision.

**Current disposition: OPEN.** No runtime implementation may be included in this PR as if authorized.

### B. FDM-01 through FDM-05 — host-independent core

Candidate dependency order:
1. M1 identity/snapshot/evaluation contract (#20, #30).
2. Persistence and graph validation (#31–32).
3. Canonical geometry/operator contracts and reference operator (#33–37).
4. Regression/quality gates (#46).
5. FDM-03 binding integration and FDM-04 evaluation chain (#47–56), preserving branch-local merges.
6. FDM-05 output assembly/refinement (#57–60).
7. Clippy/formatting/CAS maintenance (#61–63).

Acceptance: exact-head workspace tests, crate-specific tests, WASM builds where relevant, Clippy policy, rustfmt, migration and adversarial regression coverage. Confirm all candidate commits are actually included in the new PR; old PR titles are not proof of inclusion.

### C. AniCraft host integration — design now, implementation later

Use #64–80 as evidence/design history, not permission to implement. Once S0 closes and separate authorization is granted, create a separately gated host implementation series for transaction ownership, identity ledger, persistent extension, freshness, snapshots, evaluation publication, apply/bake and undo/redo. The final product review must include actual AniCraft host CI and GUI/undo smoke evidence.

### D. Smart Controller legacy/M-series

Reconcile #12–29 with current main and current AniCraft capabilities. Keep Smart Controller V1 preview-only unless separately authorized; preserve CCC-01.5. Resolve duplicate specifications and obsolete host assumptions before integrating any implementation.

### E. Quality and legacy retirement

- Reconcile #43–45 with #61–68. Do not mix formatting-only changes with behavior changes.
- #62's reported 71/74 failure must be resolved, not hidden by a weaker check.
- Verify #52 retirement changes against the current product and active scripts before integration.
- Make report-only Clippy clearly distinct from a blocking Clippy gate.

## 4. New unified PR policy

This PR is the single canonical review entry point. It must contain:
- This register, updated as each work item is actually integrated and tested.
- A commit manifest mapping each included commit to source PR, source SHA, and workstream.
- A dependency/overlap report showing superseded duplicate work and preserved unique changes.
- Exact CI evidence for the unified head (not merely source PR checks).
- A list of excluded/gated work, especially S0-dependent AniCraft runtime changes.

**Do not mass-merge old PRs.** Close old open PRs only after their commits are preserved by reachable branches and this replacement PR exists. Closing does not delete branches. Do not force-push or delete source branches during consolidation.

## 5. Final acceptance checklist

- [ ] All open PRs reviewed against their exact head/base and file-level diff.
- [ ] All unique commits/content accounted for in the unified branch or explicitly excluded with rationale.
- [ ] Duplicate/superseded PR content preserved before administrative closure.
- [ ] S0 evidence remains accurately OPEN until all evidence gates and formal review pass.
- [ ] CCC-01.5 and Native Asset Warp ownership invariants preserved.
- [ ] Formatting sweep completed separately and verified.
- [ ] Rust workspace tests and relevant crate tests pass on the unified exact head.
- [ ] Clippy policy and all three release WASM builds pass on the unified exact head.
- [ ] Windows release/build verification passes.
- [ ] AniCraft host checks and GUI/undo smoke evidence are included only when actual host implementation is authorized and completed.
- [ ] README and DEVELOPMENT match the resulting canonical state.
- [ ] Old open PRs closed only after the new review entry point and preservation checks are complete.

## 6. Audit limitations

This register was assembled from GitHub PR metadata and PR descriptions available on 2026-11-23, plus the previously recorded repository/branch audit. It is not a full source-diff review of all 85 PRs, does not establish that every listed commit is present in the default branch, and does not independently rerun historical CI. The “missing work” entries are acceptance gaps inferred from each PR's declared scope and limitations. Before any code cherry-pick or merge, inspect exact file diffs, ancestry, changed-file overlap, CI status and commit inclusion.

**Administrative consolidation update (2026-11-23):** replacement PR #83 is open as the sole open PR. All previously open PRs (#3–#8, #12–#16, #19–#47, #49, #53, #57–#80, #81 and #82) have been closed without merging. Their head branches were retained; no branch deletion or force-push was performed. Previously merged PRs remain merged and previously closed PRs remain closed. This administrative closure does not integrate their commits into `main` or mark their work complete.

**Current result:** PR #83 remains the sole open PR. PRs #84 and #85 were subsequently merged directly to `main` as documentation-only updates. No feature/runtime code was integrated by this consolidation. S0 remains OPEN; no runtime implementation is authorized. PR #83's documentation-only CI run succeeded, but this is not evidence of FDM or AniCraft host conformance.


## 7. Verified ancestry and recovery audit — 2026-11-23

See [PR_COMMIT_RECOVERY_MATRIX.md](PR_COMMIT_RECOVERY_MATRIX.md) for the per-PR base/head/merge SHAs, changed-file counts, current branch refs and recovery dispositions.

### 7.1 Corrected PR disposition

The full PR history now extends through #85:

| Disposition | Count | PRs |
|---|---:|---|
| Merged directly into `main` | 7 | #1, #2, #10, #11, #18, #84, #85 |
| Merged into non-main feature branches | 8 | #9, #38, #48, #50, #51, #52, #54, #56 |
| Closed without merging | 69 | All other closed PRs from #1–#85 |
| Open | 1 | #83 |

The 8 non-main merges are not in `main` merely because GitHub labels their PRs “merged.” They remain branch-local until a verified integration reaches `main`.

### 7.2 PRs #84 and #85 — later main documentation updates

- **#84** merged to `main` at `d936b3b3b41e788161fe84acd23421ad51c2c73a`. It records the partial native M1 Smart Controller UI foundation and explicitly retains the gaps in source/target binding, mapping, evaluation, preview, persistence, target application, live playback, and Windows host verification.
- **#85** merged to `main` at `7ae59f4f51da19b0e2160deed966ff5da8870903`. It records transient Stage handle creation/selection, drag-to-move, live Stage coordinates in Properties, and unit coverage. It does not establish persistent bindings, mapping/evaluation, target application, undo integration, live playback, or native host CI/manual Windows smoke evidence.
- Both PRs changed only `DEVELOPMENT.md`, `README.md`, and `docs/UI_UX_DESIGN.md`. They are documentation updates, not native runtime integration.

### 7.3 Current branch recovery map

- `test/fdm-03-v1-integration-regressions` is 313 commits ahead and 2 behind current `main`. It is the FDM-01–04.5 candidate branch and contains branch-local merges #48/#50/#51/#54/#56. Its Forma Deform CI run succeeded. It has not been merged to `main`.
- `fix/fdm-fmt-01-2-ci-runner` is 327 commits ahead and 2 behind `main), and 14 commits ahead of the FDM-03 branch. Its current CI fails at `cargo fmt --all -- --check`; Clippy, workspace tests, benchmark and the three WASM builds were skipped. Do not describe this maintenance branch as formatting-clean.
- `audit/fdm-06-13-s0-closure-review-evidence` is 330 commits ahead and 2 behind `main`. It carries FDM-05 output code and the FDM-06.13–06.17 evidence bundle. Its Forma Deform CI run succeeded, but this is not AniCraft host CI. S0 remains OPEN.
- `feat/fd-m1-main-integration` is 10 commits ahead and 7 behind `main`, with a nine-file delta covering canonical mapper/composition/mapping, mapper tests and Smart Controller adapter/preview. Its Forma Deform CI run succeeded. It is not integrated into current `main).
- The FDM-03 and FDM-FMT branches are not a single linear history. The FDM-FMT branch is 14 commits ahead of FDM-03. The S0 evidence branch and FDM-FMT branch diverge from merge base `484fc2127340568b555db2ff7364da1a7aa7bb21`: 76 commits on one side and 73 on the other. Their shared core-file overlap requires semantic reconciliation, not a blind merge.
- All PR head branch refs #1–#85 were present in the current branch inventory. The original base branch `feat/fd-m02-1-identity-topology-contracts` for PR #47 is absent; PR #47's head branch remains available.

### 7.4 Current main and canonical PR state

- Current `main` HEAD at this audit snapshot: `7ae59f4f51da19b0e2160deed966ff5da8870903`.
- PR #83 remains OPEN and DRAFT, with head `9e5e355676bc4546f651090cce3f2e0bce8f1214). Compared with current `main`, it is 2 commits ahead and 2 behind, with a one-file documentation-only diff. It has not been merged or synchronized to the latest base.
- PR #83 CI run #45208172135 completed successfully. This is a documentation PR run; it does not validate the FDM recovery branches or AniCraft host.
- No old PR commits were cherry-picked, rebased, merged into `main`, or moved during this audit.

### 7.5 S0 remains a hard gate

The latest PR #80 evidence and FDM-06.17 addendum still mark S0 **PARTIAL / IN PROGRESS / OPEN**. The 36-row manifest reconciliation, complete transitive callback effects, compiler-backed consumer verification, and successful pinned AniCraft host CI remain outstanding. The successful Forma Deform branch CI runs do not satisfy these host gates.

**Disposition:** the commit history is recoverable, but it is split across divergent workstreams. The next safe step is source-level review of candidate commits and overlap, beginning with FDM-03 versus FDM-FMT and the FDM-05/S0 branch. No mass merge or implementation is authorized.
