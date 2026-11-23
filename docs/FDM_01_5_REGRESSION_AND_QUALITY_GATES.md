# FDM-01.5 — Regression and Quality Gates

**Status:** Regression coverage and CI gates added; final completion is conditional on CI passing on the final branch head.  
**Branch:** `feat/fdm-01-5-regression-quality-gates`  
**Base:** Verified FDM-01.4 head `0a1536b0a5396f112452d5cde0fd5bb858b47baa`  
**Scope:** Forma Deform core and repository verification only

## Purpose

Protect the combined FDM-01.1–01.4 behavior against regressions at the boundaries between geometry snapshots, Path2D, operator dispatch, single-operator evaluation, and immutable results.

The goal is not to duplicate every module's unit tests. Existing unit tests continue to own local behavior; the FDM-01.5 integration tests exercise the contracts together.

## Cross-module regression coverage

The integration suite in `crates/form-deform-core/tests/fdm_01_regression.rs` verifies:

- A captured source snapshot can pass through the single-handle operator and evaluation engine to produce a validated immutable result.
- The full Path2D command sequence is preserved, including line, quadratic, cubic, and close commands. The test compares command discriminants and counts rather than relying on a rendered approximation.
- Quadratic and cubic paths are evaluated without converting the payload to sampled polylines.
- The original input geometry and captured source snapshot remain unchanged after evaluation.
- Both `ReplaceSourceGeometry` and `IndependentDeformationContribution` remain explicit result operations. They carry the same independently evaluated payload in this single-operator test; the engine does not combine contributions.
- The result validates against its captured request and its source-freshness check reports the unchanged source as current.
- A source revision change between captured and current requests is rejected before a result is produced, with the stable `StaleSourceSnapshot` diagnostic.

## Quality gates

The CI workflow now names and separates the relevant gates:

1. TypeScript typecheck, tests, and build.
2. Rust formatting check for the FDM operator, evaluation engine, and new cross-module regression test.
3. FDM-core Clippy across all targets, with warnings denied. The pre-existing `clippy::too_many_arguments` allowance remains narrowly explicit.
4. An explicit `cargo test -p form-deform-core --all-targets` gate, including integration tests.
5. Full Rust workspace tests.
6. Release WASM builds for bone-query, Form Deform, and Smart Controller plugins.

The package-level test is intentionally retained alongside workspace tests: it makes the FDM-core quality gate visible and independently attributable while the workspace gate protects cross-crate compatibility.

## Contract boundaries preserved

- No AniCraft source or plugin host behavior is changed.
- No native Asset Warp replacement or duplicate solver is introduced.
- No source freshness is inferred from a mutable live host reference; the host still owns atomic final validation before applying a result.
- No graph scheduler, cache, parallel evaluation, result application, undo/redo, or baking is added.
- Independent deformation contributions are not combined.
- Smart Controller and Form Deform evaluation remain separate under CCC-01.5.

## Completion criteria

FDM-01.5 is complete only when the final branch-head commit has a successful GitHub Actions run covering every gate above, and a focused review confirms that the new regression tests exercise the intended contracts without weakening or changing FDM-01.1–01.4 semantics.

A successful run on an earlier commit does not satisfy this requirement. The branch and PR remain draft/unmerged until explicitly authorized.
