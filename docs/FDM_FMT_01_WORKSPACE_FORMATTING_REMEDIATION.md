# FDM-FMT-01 — Workspace Formatting Remediation

**Status:** Baseline recorded; formatting not yet executed.
**Baseline source:** GitHub Actions run [#45170154737](https://github.com/Mujshanan/forma-deform/actions/runs/45170154737)
**Baseline commit:** `f2df76d807a7375a4eb96c20432f4ad2ed0b30b8` (PR #63)
**Maintenance branch:** `chore/fdm-fmt-01-workspace-formatting-remediation`
**Base branch:** `chore/pr-47-2-workspace-formatting-ownership-audit`
**Base SHA:** `375fa15f2b9107d2f50b318547c2bad634b4f536`

## 1. Scope and invariants

This is a separately scoped, formatting-only maintenance effort. It does not expand PR #63.

- Preserve PR #63 open and draft; do not change its head, files, or metadata.
- Preserve PR #62 and PR #61 open and draft.
- Do not modify PR #47, #37, or #40.
- Do not merge any PR or mark any PR ready.
- Do not change behavior, APIs, tests, lint policy, or workspace membership.
- Keep `cargo fmt --all -- --check` as a strict workspace-wide CI gate.
- Do not use formatting remediation to waive or hide CI failures.
- CCC-01.5 remains unchanged: Smart Controller evaluation and Form Deform pin-deformation evaluation remain independent.

## 2. Exact 69-file baseline

The following paths were extracted from the rustfmt diff output of CI run #45170154737. This is the exact file set for that run, not the earlier 71/74 audit count.

### `crates/smart-controller-core` — 36 files

- `src/canonical.rs`
- `src/composition.rs`
- `src/contribution.rs`
- `src/diagnostic_codes.rs`
- `src/endpoint.rs`
- `src/error.rs`
- `src/evaluator.rs`
- `src/graph.rs`
- `src/ids.rs`
- `src/input.rs`
- `src/legacy.rs`
- `src/lib.rs`
- `src/mapping.rs`
- `src/mapping/canonical.rs`
- `src/mapping/easing.rs`
- `src/mapping/frame.rs`
- `src/mapping/range.rs`
- `src/migration.rs`
- `src/model.rs`
- `src/pose.rs`
- `src/preview.rs`
- `src/schema.rs`
- `src/validation.rs`
- `src/value.rs`
- `tests/canonical_document_tests.rs`
- `tests/canonical_mapper_tests.rs`
- `tests/dependency_boundary_tests.rs`
- `tests/diagnostic_registry_tests.rs`
- `tests/foundation_types_tests.rs`
- `tests/graph_tests.rs`
- `tests/mapping_tests.rs`
- `tests/migration_tests.rs`
- `tests/pose_angle_contract_tests.rs`
- `tests/range_mapping_tests.rs`
- `tests/v1_characterization_tests.rs`
- `tests/validation_tests.rs`

### `crates/form-deform-core` — 23 files

- `benches/curve_sampling.rs`
- `src/binding.rs`
- `src/binding_evaluation.rs`
- `src/cage_binding.rs`
- `src/cage_binding_persistence.rs`
- `src/composition_evaluation.rs`
- `src/composition_graph.rs`
- `src/curve_sample_evaluation.rs`
- `src/curve_sampling.rs`
- `src/embedding.rs`
- `src/evaluation_contract.rs`
- `src/geometry.rs`
- `src/host_contract.rs`
- `src/irregular_cage.rs`
- `src/lattice_edit.rs`
- `src/lib.rs`
- `src/outside_policy.rs`
- `src/path2d.rs`
- `src/persistence.rs`
- `src/point_location.rs`
- `src/regular_lattice.rs`
- `src/topology.rs`
- `src/topology_edit.rs`

### `crates/anicraft-plugin-sdk` — 1 file

- `src/lib.rs`

### `plugins/bone-query` — 1 file

- `src/lib.rs`

### `plugins/form-deform` — 1 file

- `src/lib.rs`

### `plugins/smart-controller` — 7 files

- `src/adapter.rs`
- `src/compatibility.rs`
- `src/input_provider.rs`
- `src/lib.rs`
- `src/preview_plan.rs`
- `src/targets.rs`
- `tests/plugin_suite.rs`

**Count check:** 36 + 23 + 1 + 1 + 1 + 7 = 69 files.

## 3. Ownership grouping

| Area | Baseline files | Owner boundary |
|---|---:|---|
| `smart-controller-core` | 36 | Smart Controller core |
| `form-deform-core` | 23 | Form Deform core |
| `anicraft-plugin-sdk` | 1 | Shared plugin SDK |
| `bone-query` | 1 | Bone Query plugin |
| `form-deform` | 1 | Form Deform plugin |
| `smart-controller` | 7 | Smart Controller plugin |

Formatting changes must be mechanical and limited to rustfmt output. No semantic cleanup, API edits, lint refactors, or test rewrites are in scope.

## 4. Active PR overlap and lineage

A changed-file comparison against open PRs identified six baseline paths also changed by PR #61:

- `crates/form-deform-core/src/cage_binding.rs`
- `crates/form-deform-core/src/cage_binding_persistence.rs`
- `crates/form-deform-core/src/curve_sampling.rs`
- `crates/form-deform-core/src/embedding.rs`
- `crates/form-deform-core/src/host_contract.rs`
- `crates/form-deform-core/src/regular_lattice.rs`

PR #61 is currently open/draft at `61c8320c484b61f0ca6f8a0b0088c3e1664b978b`. PR #62 is based on PR #61 and is open/draft at `375fa15f2b9107d2f50b318547c2bad634b4f536`. PR #63 is based on PR #62 and remains open/draft at `f2df76d807a7375a4eb96c20432f4ad2ed0b30b8`.

The maintenance branch is intentionally based on PR #62's head, not `main`: comparison showed the PR #62 lineage is 326 commits ahead of `main`. Using `main` would create an unrelated, very large integration diff. The new maintenance branch is therefore a dependent branch targeting PR #62's branch.

Defer the six PR #61-overlap files until PR #61's review/branch state is stable. Before formatting them, re-fetch PR #61, #62, and the maintenance branch heads and repeat changed-file overlap checks. Do not rebase, force-push, or update any of those existing PR branches.

PR #63's two CAS files are not in the 69-file baseline and must remain untouched.

## 5. Safe execution sequence

1. Format the 36 `smart-controller-core` files as a crate-scoped unit.
2. Format the 17 non-overlapping `form-deform-core` files; defer the six PR #61-overlap files.
3. Format the nine files in the SDK and plugin crates.
4. Re-check PR lineage and ownership before formatting the six deferred files.
5. Run scoped `cargo fmt --check` after each crate/unit and inspect each commit to confirm only intended files changed.
6. Run `cargo fmt --all -- --check` at the workspace level and all required functional CI checks.
7. Keep the strict workspace CI gate unchanged throughout.

Each execution unit should be recorded in its own formatting-only commit with a before/after path list and verification evidence. If the current CI toolchain or branch has moved, refresh the baseline rather than silently applying stale formatting output.

## 6. Current disposition

- Baseline captured: **69 files**.
- Ownership grouping recorded.
- Active PR overlap: **six files on PR #61**.
- Safe base selected: PR #62 head, SHA `375fa15f2b9107d2f50b318547c2bad634b4f536`.
- Maintenance branch created from that exact SHA.
- Formatting execution: **not yet performed**.
- Workspace CI gate: **strict; no waiver or weakening**.
