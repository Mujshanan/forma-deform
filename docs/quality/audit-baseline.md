# CQ0 — Code Quality Audit Baseline

**Status:** Baseline established from repository metadata and GitHub Actions evidence; local execution was unavailable in the audit environment.  
**Reference branch:** `main`  
**Reference commit:** `e7c7fef4e82345af311370426450146a4795523d`  
**Baseline date:** 2026-11-23

## 1. Purpose and scope

This document records the reproducible starting point for the Forma Deform code-quality audit. CQ0 is diagnostic only: it does not change application code, public APIs, persisted formats, or runtime behavior.

The baseline covers the default branch, its workspace layout, documented project boundaries, current CI commands, and the most recent successful CI run on the reference commit. Feature branches and pull requests are recorded separately and are not treated as merged baseline behavior.

## 2. Repository structure observed

The repository contains two development tracks:

- A Rust workspace containing six members: `smart-controller-core`, `form-deform-core`, `anicraft-plugin-sdk`, and the Bone Query, Form Deform, and Smart Controller plugins.
- A separate TypeScript/Bun rectangle and timeline demo, with source under `src/` and tests under `tests/`.

The Smart Controller core has a meaningful module split across canonical models, composition, contribution, diagnostics, endpoints, errors, evaluation, graphs, IDs, inputs, legacy compatibility, mapping, migration, models, poses, previews, schemas, validation, and values. Its integration plugin has separate adapter, compatibility, input-provider, preview-plan, and target modules.

The repository already documents the product boundary: Forma Deform owns deformation/controller domain behavior, while AniCraft owns the authoritative document, timeline, renderer, undo/redo, plugin lifecycle, persistence infrastructure, and evaluation scheduling. These boundaries should be treated as architecture constraints during later audits and refactors.

## 3. CI baseline

**Latest successful main-branch run inspected:** [CI run 45013439505](https://github.com/Mujshanan/forma-deform/actions/runs/45013439505)  
**Commit tested:** `e7c7fef4e82345af311370426450146a4795523d`  
**Workflow job:** `verify` — success.

The workflow currently performs:

1. Bun dependency installation.
2. TypeScript type-check (`bun run typecheck`).
3. TypeScript tests (`bun test`).
4. TypeScript browser build (`bun run build`).
5. Rust workspace tests (`cargo test --workspace`).
6. Release WASM builds for all three AniCraft plugins.

Observed test results from the successful job log:

- **Rust:** 188 passed, 0 failed, 0 ignored, across 26 test binaries/suites.
- **TypeScript/Bun:** 9 passed.
- Type-check, demo build, Rust workspace tests, and all three WASM release builds completed successfully.

This is a CI-reported baseline, not a locally reproduced result in this audit environment.

## 4. Current verification gaps

The workflow file currently does not invoke the following checks:

- `cargo fmt --all -- --check` (the rustfmt component is installed, but no formatting check is run).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo doc --workspace --no-deps` or rustdoc warning enforcement.
- Dependency/license/security auditing.
- Coverage reporting or explicit minimum behavioral-coverage policy.
- Frozen Bun dependency installation.

The workflow uses the moving `stable` Rust toolchain and `bun-version: latest`. The repository tree inspected at this commit contains no `rust-toolchain.toml`, `Cargo.lock`, `bun.lock`, or `bun.lockb`. These observations should be rechecked before changing policy; in particular, lockfile strategy should be decided deliberately for a mixed library/plugin workspace.

## 5. Environment limitations

The audit runner used for this CQ0 session did not have `rustc`, `cargo`, `rustup`, or `bun` installed, and direct GitHub clone access was unavailable. The repository and CI evidence were inspected through GitHub's repository and workflow interfaces. Node.js was present, but it is not a substitute for the project's Bun scripts.

Therefore:
- No local test, format, Clippy, doc, or WASM command is claimed as executed.
- The successful GitHub Actions run is the source of truth for the baseline checks that it actually executes.
- The next audit step should arrange a reproducible Rust/Bun environment and run the expanded verification set.

## 6. Initial observations (not yet defect findings)

These are audit leads, not confirmed code defects:

| ID | Observation | Why it matters | Follow-up |
|---|---|---|---|
| CQ0-01 | CI does not run rustfmt or Clippy | Formatting drift and common Rust maintainability issues can pass CI | Evaluate adding both checks |
| CQ0-02 | Toolchains and Bun version float to latest/stable | A clean checkout may change behavior as toolchains evolve | Decide and document supported toolchain policy |
| CQ0-03 | No lockfiles were present in the inspected tree | Dependency resolution may not be repeatable across time/CI | Decide lockfile policy for workspace and demo |
| CQ0-04 | CI tests core and plugin crates together, but no separate architecture/layering gate is visible | Dependency boundaries can regress without failing behavior tests | Review existing boundary tests and identify missing rules |
| CQ0-05 | Public API docs and rustdoc warning policy are not checked in CI | Documentation can become stale while builds remain green | Review API documentation and consider rustdoc checks |
| CQ0-06 | No coverage or mutation-testing evidence is published by this workflow | Passing tests do not reveal untested critical behavior | Map behavioral coverage in CQ5; do not optimize for a raw percentage |

## 7. Scope and active work

The repository has multiple open, mostly draft pull requests and parallel feature branches, including Smart Controller persistence/evaluation work and a newer deformation-model track. The default branch is therefore a stable reference snapshot, not a claim that all current development is represented in `main`.

For subsequent CQ phases:
- Keep audit findings tied to an exact branch and commit.
- Review active feature branches separately when their code is in scope.
- Do not refactor across concurrently changing feature branches without coordinating the base.
- Preserve the documented AniCraft/Forma Deform ownership boundary.

## 8. CQ0 exit criteria

- [x] Record reference branch and commit.
- [x] Inventory workspace and major code tracks.
- [x] Inspect current CI workflow.
- [x] Record latest successful CI run and observed test counts.
- [x] Record local execution limitations.
- [x] Record initial audit leads without prematurely labeling them defects.
- [ ] Establish a reproducible local/CI environment for additional quality checks.

## 9. Next step

Proceed to **CQ1 — Architecture and module boundaries**. Map crate/module responsibilities, public interfaces, dependency directions, host adapters, and current architectural tests. Do not begin broad refactoring until CQ1 findings are reviewed.
