# Repository Consolidation Plan

**Snapshot:** 23 November 2026  
**Canonical integration target:** `main`  
**Status:** consolidation inventory and controlled integration plan; no feature PRs merged or closed by this document.

## 1. Objective

Restore one understandable development path without losing useful work, bypassing required review, or treating branch-local CI as evidence for another commit.

Rules:

1. `main` is the only canonical baseline and eventual release integration branch.
2. Feature branches are evidence/work containers, not release state.
3. Preserve source branches and PR history until their contents are classified and either integrated or explicitly superseded.
4. Never merge a stacked PR before its parent dependency is integrated or deliberately retargeted.
5. A green check is valid only for the exact head SHA and workflow run.
6. S0 remains OPEN. No native runtime implementation may begin until formal S0 closure and separate explicit implementation authorization.
7. CCC-01.5 remains binding: Smart Controller binding evaluation and Form Deform pin evaluation stay independent. Native Asset Warp remains AniCraft-owned.
8. Documentation, formatting, correctness changes and feature implementation must be separate reviewable changes.

## 2. Consolidation findings

The repository currently has several distinct development histories:

- Legacy Smart Controller authoring/persistence chain: PRs #22–#29.
- FDM v2 contract and operator/cage chain: PRs #30–#40 and #46.
- Rust quality/formatting chain: PRs #38–#45 and #61–#63, with PR #68 recording a separate formatting baseline.
- FDM-03/04/05 implementation chain: PRs #47–#60.
- FDM-06 host-boundary and S0 evidence chain: PRs #64–#80.
- README/DEVELOPMENT status refresh: PR #81.

These chains are not all based on `main`. Some PRs are stacked on other open PRs; some closed/merged PRs were merged into feature branches rather than `main`. Their state must not be confused with default-branch integration.

## 3. Canonical tracks and disposition

### Track A — S0 evidence (highest priority; audit only)

Canonical current evidence chain:

`#73 → #75 → #79 → #80`

Related or competing work requiring reconciliation:

- #64 → #65 and #71: competing FDM-06.3 host-conformance plans.
- #66 → #67 → #69 → #70 → #72 → #73: earlier host identity/mutation design lineage.
- #74: alternate FDM-06.6 invalidation-contract branch.
- #76 and #77: competing FDM-06.11 S0 registry/ownership closure assessments.
- #78: gated mutation-boundary implementation plan, based on #76 lineage.
- #79 → #80: newer workspace-consumer baseline and S0 evidence addendum.

Disposition:
- Treat #80 as the latest S0 evidence submission, not closure.
- Compare #71 against #65 and #74 against #69; retain unique evidence, then nominate one canonical document for each requirement.
- Reconcile #76/#77 and #78 against the #79/#80 lineage.
- Do not close competing PRs until unique content is accounted for.
- Outstanding closure evidence remains: complete callback transitive-effect graph, reconciled original 36-row callback manifest, compiler-backed consumer verification, and successful host CI on the pinned AniCraft baseline.
- S0 stays OPEN until formal review accepts the complete evidence.

### Track B — FDM-04/05 implementation evidence

Relevant lineage:

- #47 (FDM-03 integration regression baseline).
- #48, #50, #51 (closed/merged within the #47 feature lineage).
- #52 (legacy Bun demo retirement, merged into the LEG-01.4 branch).
- #54 (sampling benchmark/quality gate; preferred over superseded #53).
- #56 (FDM-04.5 clean-lineage integration; supersedes closed unmerged #55).
- #57 → #58 → #59 → #60 (FDM-05 output integration and hardening).
- #61 → #62 → #63 (PR #47 maintenance/ownership and Path2D persistence CAS chain).

Disposition:
- Treat #53 as superseded by #54; preserve its history, but do not integrate both.
- Treat #55 as superseded by #56.
- Audit #52 and #54/#56 ancestry to determine the smallest verified source commits needed for a single integration branch. Do not assume their feature-branch merges put them on `main`.
- Keep #57–#60 stacked until their output contract, tests and exact-head CI are reviewed as a unit.
- Resolve #61–#63 separately from feature behavior; do not combine formatting or lint changes with functional changes.

### Track C — Rust quality and formatting

Relevant lineage:

- #38 merged into `quality/cq2-rustfmt-clean`, not necessarily `main`.
- #39 and #41 are decomposition/refactor candidates on that quality branch.
- #42 records native-host traceability documentation.
- #43 adds Rust formatting verification.
- #44 adds a non-blocking Clippy report.
- #45 applies stable rustfmt formatting on a predecessor branch.
- #68 records FDM-FMT-01 baseline only; it is not the formatting sweep.

Disposition:
- Establish one quality base from current `main`.
- Apply formatting-only changes as a dedicated PR with an exact file manifest and zero semantic diff.
- Add `cargo fmt --all -- --check` to CI only after the formatting baseline is clean.
- Keep Clippy report-only until its findings are triaged; do not turn it into a blocking gate without a separate decision.
- Review #39/#41 refactors independently and retain only changes with demonstrated behavioral equivalence and tests.

### Track D — Legacy product-plan branches

PRs #22–#29 represent earlier milestone/authoring/persistence/joystick planning and implementation attempts. Their bases are not a reliable continuation of the current S0-gated FDM-06 sequence.

Disposition:
- Inventory unique useful commits and specifications.
- Reconcile the five-pose joystick and PoseGrid work with current Smart Controller core APIs and CCC-01.5.
- Do not revive old native settings-dialog or persistence assumptions until the host contracts are verified.
- Retarget or close these PRs only after their unique content is preserved or superseded.

### Track E — Documentation

- #81 updates README.md and DEVELOPMENT.md from `main`.
- Keep the documentation PR separate from code integration.
- Before merging, correct any status statements invalidated by intervening PR changes and verify every branch/PR link and SHA.
- Documentation must continue to state that S0 is OPEN and that branch-local merges are not `main` integration.

## 4. Required integration order

1. Freeze the source baseline and capture current open PR heads, parent/base relationships and exact CI evidence.
2. Complete S0 evidence collection and formal closure review. No runtime implementation during this step.
3. Reconcile duplicate/competing audit documents (#65/#71, #69/#74, #76/#77, #78/#79/#80).
4. Establish the canonical FDM-04/05 branch from the latest approved parent; select #54 over #53 and #56 over #55; review #57–#60 and #61–#63 for compatibility.
5. Establish one formatting/quality branch from the chosen integration baseline; verify formatting-only diff and exact tests.
6. Integrate legacy demo retirement from #52 only after verifying its dependency and ensuring no still-needed demo files are removed.
7. Integrate reviewed feature work in dependency order, with exact-head CI at each merge boundary.
8. Merge #81 after reconciling documentation with the resulting `main`.
9. Only after S0 closure and separate authorization, begin native runtime implementation.

## 5. Branch and PR handling policy

For each PR, assign exactly one disposition:

- **KEEP / canonical:** unique work remains required and is on the selected dependency path.
- **CHERRY-PICK / transplant:** useful commit(s) need to be moved to a clean canonical branch.
- **RETARGET:** PR base is wrong but the complete diff remains valid against a new parent.
- **SUPERSEDED:** all useful content is represented by a newer PR/commit; close only after verifying equivalence.
- **PARK:** potentially useful but blocked by a contract or milestone.
- **REJECT:** incorrect or unsafe; requires explicit documented rationale.

Do not mass-retarget stacked PRs. Retargeting changes merge bases and may silently change the effective diff. Recompute and review each diff after every base change.

## 6. Immediate next actions

1. Generate a commit-level ancestry and changed-file overlap report for every open PR.
2. Verify exact-head CI/check status for candidate canonical PRs.
3. Inspect duplicate audit PR diffs and record unique evidence to preserve.
4. Build one integration branch only after the above classification; keep it non-default and non-release.
5. Request review at each gate. Do not merge automatically.

## 7. Non-goals of this consolidation pass

- No source-code edits.
- No PR merges or closures.
- No branch deletion or force-push.
- No weakening or bypassing of CI.
- No S0 closure declaration.
- No native runtime implementation authorization.

This document is an auditable consolidation map, not a declaration that integration work is complete.
