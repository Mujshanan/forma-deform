# CQ2 — Readability and Code Consistency Review

**Status:** Initial source inspection; not a complete repository-wide audit.  
**Code reference:** `main` at `e7c7fef4e82345af311370426450146a4795523d`  
**Audit branch:** `quality/cq0-audit-baseline`  
**Review date:** 2026-11-23

## Scope and context

This review evaluates the Forma Deform code in the context of its product evolution: three original plugins were consolidated into one plugin, and the unified product was then developed into a native AniCraft tool. The native host already has built-in Asset Warp, Bone, and Bind tools. Therefore, readability findings distinguish the historical/plugin adapter surface from the canonical/native product path; no recommendation here assumes the old plugin entrypoints are the main product composition root.

This is an initial pass over the deformation core, Smart Controller core, and Smart Controller adapter. It does not claim every crate or file has been reviewed.

## Findings

### CQ2-01 — Form Deform core uses compressed, inconsistent Rust formatting

**Priority:** P1 (readability / maintainability)  
**Evidence:** `crates/form-deform-core/src/lib.rs:3-5, 17-18, 30-42, 48-70`.

Several public structs, enum variants, conditionals, and test assertions are compressed onto single lines. The function body also places the input array and finite-value checks in a compact layout that is harder to scan than the surrounding Rust code in the Smart Controller core.

**Recommendation:** Apply standard rustfmt formatting to this crate and keep the same formatting policy across the workspace. Do not alter behavior as part of the formatting change.

**Acceptance:** `cargo fmt --all -- --check` passes in CI; formatting-only diff for this file is behavior-neutral and tests remain unchanged.

### CQ2-02 — Smart Controller adapter has imports separated from the module header

**Priority:** P2 (file navigation / consistency)  
**Evidence:** `plugins/smart-controller/src/adapter.rs:1-16` and `adapter.rs:60-69`.

The file begins with its SDK/core imports, defines several public adapter functions, then introduces a second group of imports (`serde_json::Value` and additional core types) partway through the file. There are also multiple extra blank-line gaps between related functions.

**Recommendation:** Consolidate imports at the top and group them by dependency/module role. Keep functions grouped by workflow: pure snapshot evaluation, host collection/query/parsing, then first-party mapper convenience wrappers. This is a readability-only change; retain public signatures.

**Acceptance:** All imports are at the top; the module's section order follows the call/data flow and existing tests pass.

### CQ2-03 — Migration validation contains dense, difficult-to-scan expressions

**Priority:** P2 (reviewability)  
**Evidence:** `crates/smart-controller-core/src/migration.rs:29-57`.

The validation routine builds a controller lookup map, searches inputs repeatedly, and nests controller-output dependency checks in a compact block. The logic is important and correct-looking at first inspection, but its control flow and repeated lookups are harder to audit than necessary.

**Recommendation:** In a separate refactoring proposal, consider named private helpers for validating input references and controller-output references. Before changing it, profile/measure only if performance is a concern; the immediate goal is to expose validation rules clearly, not optimize speculatively. Preserve exact error categories and messages unless a deliberate compatibility change is approved.

**Acceptance:** Each validation rule is independently understandable and covered by focused tests; no migration behavior or diagnostic contract changes.

### CQ2-04 — Evaluator size raises navigation cost, but its phase documentation is a strength

**Priority:** P2 (maintainability)  
**Evidence:** `crates/smart-controller-core/src/evaluator.rs:1-130`; file is approximately 1,071 lines.

The module has useful top-level documentation, an explicit four-stage evaluation contract, and clear separation through the `ControllerMapper` trait. Its size still makes it expensive to navigate and review.

**Recommendation:** Do not split it merely to reduce line count. First inventory the private helpers and phase boundaries through the rest of the file. Extract only cohesive private units with stable invariants, and preserve the single public evaluation contract.

**Acceptance:** A proposed decomposition maps each helper to one named stage and identifies the tests that protect it before code is moved.

### CQ2-05 — Legacy V1 entrypoint and canonical adapter need explicit labels in source documentation

**Priority:** P2 (maintainer orientation)  
**Evidence:** `plugins/smart-controller/src/lib.rs:1-3, 43-61`; `plugins/smart-controller/src/adapter.rs:1-4, 27-60).

The entrypoint clearly says that V1 is a preview-only Slider workflow. The adapter separately exposes canonical document evaluation and host-input preview. The code is readable locally, but the relationship to the native AniCraft tool and to the original three-plugin/unified-plugin lifecycle is not yet stated consistently at the module boundary.

**Recommendation:** After the native host integration trace is complete, add a short module-level lifecycle note identifying whether each surface is current native integration, compatibility support, test seam, or historical plugin workflow. Do not label canonical adapter code as dormant or legacy until actual call sites and product intent are verified.

**Acceptance:** A new contributor can identify the authoritative native workflow and the support status of the V1 plugin path without inferring it from function names.

## Positive readability practices to preserve

- Core module documentation explains determinism and host independence.
- `ControllerMapper` makes the evaluator's algorithm boundary explicit.
- Evaluator stages and preflight guarantees are documented before the implementation.
- Adapter functions clearly state when they are read-only and what context is supplied by the caller.
- Tests use behavior-oriented names and exercise deterministic/non-accumulating behavior.

## Audit limitations

- This pass is based on GitHub source inspection; no local formatter, compiler, or test execution was performed.
- The CI baseline passed the Rust test suite, Bun tests, TypeScript typecheck, browser build, and release WASM builds, but the recorded workflow did not enforce `cargo fmt --all -- --check`.
- No broad claim is made about Clippy, coverage, or all modules.
- Native host evidence is documented separately in `architecture-map.md`; the exact relationship between the native Asset Warp implementation and the Forma Deform core crates remains an open integration-trace item.

## Next step

Complete CQ2 across the remaining core and plugin modules, then run a formatting-only proposal in a dedicated branch with CI enforcement. Keep formatting, structural refactoring, and behavior changes in separate commits.
