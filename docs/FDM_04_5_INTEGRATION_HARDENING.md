# FDM-04.5 — Integration Hardening

## Scope

Close the rest-cage identity gap in adaptive curve evaluation, make the Rust verification contract explicit on active development branches, and reconcile the FDM-04 branch lineage without rewriting shared history.

## Rest-cage identity

A `ValidatedBindingRecord` now retains the geometry fingerprint of the certified rest cage used during validation. This fingerprint is runtime capability state, not persisted binding data, so the persisted binding schema remains unchanged.

Before adaptive curve samples are regenerated or mapped, evaluation requires the supplied rest cage fingerprint to match the validated binding's captured fingerprint. A different current-pose cage remains valid when it retains the same topology identity and revision; only the rest-cage argument is pinned to the original validated geometry.

The regression suite verifies that a certified cage with the same topology ID and revision but different geometry is rejected, while the existing translated current-pose evaluation remains accepted.

## CI contract

Push CI now covers `main`, `feat/**`, `test/**`, `perf/**`, and `fix/**`. The workflow runs workspace tests, the adaptive curve-sampling benchmark smoke run, and release builds for all three WASM plugin crates.

A trial `cargo fmt --all -- --check` exposed existing formatting drift across numerous untouched core and plugin files. The blocking format gate is deferred to a dedicated formatting-baseline change rather than introducing unrelated repository-wide churn in this correctness milestone.

The retired standalone Bun/TypeScript rectangle demo is not part of the current verification contract.

## Branch lineage

This PR targets `test/fdm-03-v1-integration-regressions`, the branch containing the FDM-04.2 integration merge commit. The FDM-04.4 branch carried the implementation commit history but did not contain that merge commit as an ancestor. The clean hardening branch was created directly from this integration branch (22 commits ahead, 0 behind at creation). The upstream branch advanced by three commits during review; its relevant freshness-validation and documentation changes were incorporated into this branch.

Targeting the upstream integration branch lets GitHub create the normal merge ancestry when this PR is eventually merged. No shared branch is reset, force-pushed, or rewritten. This PR is not authorized for automatic merge.

## Boundaries preserved

- No persisted binding schema migration.
- No change to current-pose cage semantics.
- No Path2D mutation or flattening.
- No Smart Controller/FDM composition.
- No production curve-sampling algorithm change.
- No host UI, cache, scheduler, apply, or bake work.
