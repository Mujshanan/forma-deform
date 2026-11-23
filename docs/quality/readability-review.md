# CQ2 — Readability and Code Consistency Review

**Status:** Expanded source review complete; remediation proposal pending.  
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


## Expanded inspection — core and all three historical plugin crates

The review was extended beyond the initial Smart Controller adapter pass. The repository's plugin directory contains the three historical plugin tracks `bone-query`, `form-deform`, and `smart-controller`. They are assessed here as historical/prototype adapter surfaces in the product's evolution, not as three independent current products. The audit also inspected the canonical controller model, composition layer, host input provider, target preview planner, and compatibility adapter.

### CQ2-06 — Canonical controller validation concentrates many rules in one method

**Priority:** P1 (reviewability / change safety)  
**Evidence:** `crates/smart-controller-core/src/canonical.rs:75-170` (the `Controller::validate_local` implementation).

The method validates controller identity relationships, bindings, composition-mode combinations, target-specific constraints, and mapping-specific constraints. Several match arms combine multiple predicates and error construction on one line. The implementation is deterministic and uses explicit typed errors, but the number of distinct invariants in one method makes it difficult to review a targeted change without mentally tracking unrelated rules.

**Recommendation:** Keep validation owned by the canonical model, but separate private validation phases by invariant family: controller/input references, binding/composition policy, target mapping, and controller mapping. Preserve validation order where it affects which error is returned. Avoid introducing a generic validation framework for this one type.

**Acceptance:** Each phase has a narrow name and focused tests; error variants/messages and first-error behavior remain stable unless deliberately versioned.

### CQ2-07 — Historical Bone Query plugin combines host query, response interpretation, and presentation

**Priority:** P2 (separation of concerns)  
**Evidence:** `plugins/bone-query/src/lib.rs:29-230`.

The file owns parameter interpretation, `bone.info` invocation, response traversal, fallback formatting, and user-facing log presentation. Helper functions do provide some decomposition, but the report functions still mix extraction/defaulting with output formatting, and `format_bone` constructs a very long multi-property line. This is understandable for a compact diagnostic command, but makes response-contract changes and presentation changes harder to review independently.

**Recommendation:** Keep the plugin thin and historical. If this query remains a maintained adapter, introduce a small pure response-to-report model and a separate renderer; do not move host SDK calls into core crates. If it is no longer part of the native workflow, document its support status instead of expanding its architecture.

**Acceptance:** Response parsing can be tested without SDK logging; output wording can change without changing host response interpretation; no host calls are added to core.

### CQ2-08 — Form Deform plugin entrypoint is a single end-to-end workflow module

**Priority:** P2 (navigation / separation of concerns)  
**Evidence:** `plugins/form-deform/src/lib.rs:1-92`.

The entrypoint contains the plugin manifest, parameter parsing, host bone query, evaluated-angle selection, warp mesh/pin verification, core deformation invocation, logging, and optional mutation command. The workflow is linear and small enough to follow, and it correctly keeps the math in `form-deform-core`; however, input resolution, preview reporting, and application policy are coupled in one function.

**Recommendation:** Do not split solely for line count. If this adapter remains supported, extract named private steps for resolving the bone angle, verifying the target pin, calculating the target, and applying the result. Keep the apply/preview distinction explicit and retain the host command as the only mutation boundary. If the native tool supersedes this adapter, retain it as a compatibility probe and avoid feature growth here.

**Acceptance:** `run` reads as orchestration; each host interaction and the apply gate is independently testable; core remains host-independent.

### CQ2-09 — Composition code has dense policy branches despite a clear top-level pipeline

**Priority:** P2 (reviewability)  
**Evidence:** `crates/smart-controller-core/src/composition.rs:46-295`.

The public composition flow is concise and the policy is decomposed into named helpers (`compose_replace`, `compose_additive`, `compose_weighted_blend`, `compose_priority_override`, and compatibility helpers). That is a good structure to preserve. Within weighted angle handling and some compatibility branches, however, several semantic checks and calculations are compressed into nested expressions and long lines.

**Recommendation:** Preserve the current policy-per-helper structure. Extract only named predicates/calculations where they make wrapped-angle semantics, zero-weight behavior, or conflict conditions easier to inspect. Do not merge composition back into the evaluator.

**Acceptance:** Every composition mode remains independently readable and covered by its existing behavior tests; no change to deterministic ordering or conflict semantics.

### CQ2-10 — Plugin adapter module boundaries are clearer than their entrypoint-to-native lifecycle documentation

**Priority:** P2 (maintainer orientation)  
**Evidence:** `plugins/smart-controller/src/adapter.rs:1-16,60-90`; `plugins/smart-controller/src/input_provider.rs:1-55`; `plugins/smart-controller/src/preview_plan.rs:1-105`; `plugins/smart-controller/src/targets.rs:1-103`.

The canonical Smart Controller adapter has useful separation: input collection, pure evaluation adaptation, target preview planning, and target capability assessment are distinct modules. The input provider explicitly requires host identity mappings; the preview planner preserves typed outputs and does not mutate; target capability descriptions distinguish calculation support from verified application. This is a strong maintainability pattern. The adapter's imports are split midway through the file, and the relationship between these modules, the V1 plugin entrypoint, and the native AniCraft workflow is not yet stated consistently.

**Recommendation:** Consolidate adapter imports (overlaps CQ2-02) and add a concise lifecycle/support note at the module boundary only after the native call path is verified. Preserve the existing read-only seams and the separation between core calculation and host mutation.

**Acceptance:** A maintainer can follow the path from host input collection to canonical evaluation to preview planning, and can tell whether the plugin adapter is a supported integration path or a test/prototype surface without guessing.

## Expanded review conclusions

- The canonical Smart Controller implementation is generally better structured than the historical plugin entrypoints: model, graph, evaluator, composition, input, mapping, migration, and diagnostics have distinct modules.
- The best existing pattern is the separation of pure core evaluation from host collection and read-only target preview. Preserve this as the native tool is integrated.
- The most concentrated readability risk is invariant density in `Controller::validate_local`, followed by the evaluator's size and the older adapter entrypoints.
- The three historical plugin folders should not be refactored into three independently evolving products. Any cleanup should either support the unified native tool or explicitly maintain a small compatibility/test adapter.
- No behavior defects were established by this readability pass. These are maintainability findings, not correctness bug reports.
- No source code was changed during CQ2. This is an audit/documentation update only.

## CQ2 status and exit criteria

**Status:** Expanded source review complete; remediation proposal pending.

Completed:
- [x] Inspect Form Deform core and canonical Smart Controller model.
- [x] Inspect evaluator/composition/migration responsibilities.
- [x] Inspect Smart Controller host adapter, input provider, preview planner, and target capability layer.
- [x] Inspect all three historical plugin entrypoints/modules.
- [x] Record findings with file/line references and behavior-preserving recommendations.
- [x] Separate product lifecycle observations from code-quality findings.

Still required before CQ2 can be closed:
- [ ] Run `cargo fmt --all -- --check` against the audit branch in CI or a Rust-enabled environment.
- [ ] Produce a formatting-only patch proposal, separately from any structural refactor.
- [ ] Verify the native integration call path before changing adapter lifecycle labels or removing historical plugin code.
- [ ] Re-run Rust tests and WASM builds after any remediation.



## Candidate automated quality tools — suitability assessment

The following external tools were evaluated as possible instruments for CQ2 and the wider CQ audit. This is a suitability review only: none has been installed, run against this repository, or added to CI. Tool output must be treated as evidence to review, not an automatic refactoring mandate.

### Tool comparison

| Tool | Best use in Forma Deform | Risks / limitations | Recommendation |
|---|---|---|---|
| `mozilla/rust-code-analysis` | Establish quantitative baselines for cyclomatic/cognitive complexity, Halstead metrics, maintainability index, function size, and nesting. | Metrics identify hotspots, not design defects; thresholds need interpretation and comparisons must use the same scope/version. | **Use for CQ2 measurement.** |
| `saschaontour/rustqual` | Structural scan for complexity, duplication, SRP, coupling, test quality, and configurable architecture rules. | Syntactic analysis; IOSP classification is opinionated and may not suit every Rust idiom. Workspace layer mapping and suppressions need careful calibration. | **Pilot on a non-gating report first.** |
| `AgentWorkforce/rust-oleum` | Ratchet to stop quality debt increasing after an agreed baseline; can combine source metrics with Clippy, coverage, and mutation evidence. | Repository guide requires a lockfile for its measurement workflow; current audit baseline found no `Cargo.lock`. Default policy is strict and includes 100% coverage, zero panic/suppression targets, and opinionated lint groups. It can create noise or pressure unsafe/incorrect cleanups if adopted without policy review. | **Consider later as a regression gate**, after lockfile and baseline policy decisions. |
| `RAprogramm/cargo-quality` | Optional checks for import style, format argument style, blank lines, and inline comments. | Its embedded house style is not Rust community convention: it flags blank lines inside functions and inline comments, and its formatter uses nightly/unstable options including `trailing_comma = "Never"`. This conflicts with the project's preference for standard, readable Rust formatting. | **Do not use as a formatter or blocking gate.** Individual analyzer experiments only if they demonstrate value. |

### Tool-specific notes

- **Mozilla rust-code-analysis:** Its documented metric set includes ABC, cyclomatic complexity, cognitive complexity, Halstead, maintainability index, logical/physical lines, argument count, exit count, and function/method counts. Use the exported metrics to rank hotspots; do not convert a single maintainability score into a pass/fail quality verdict.
- **rustqual:** Its seven dimensions and configurable layer/forbidden-edge rules make it the closest candidate to the architecture and maintainability goals of this audit. Its adapter call-parity feature is not directly relevant to the three historical plugin tracks unless they are deliberately maintained as peer adapters to the same application surface. Do not configure parity merely because three plugin directories exist; product evolution says these are historical tracks leading to one native tool.
- **rust-oleum:** Its ratchet model is valuable for preventing regression once debt is measured and accepted. Before adoption, decide whether to commit a `Cargo.lock`, define `rust_roots`, explicitly set lint groups, choose realistic coverage/mutation policy, and review every default target. In particular, do not adopt a 100% coverage target or zero-`unsafe` policy blindly in a host SDK integration workspace.
- **cargo-quality:** Do not run its formatter over the repository. Stable `cargo fmt` remains the formatting authority. Its analyzers should be considered independently and only after dry-run output is reviewed.

### Recommended staged use

1. **Measurement only:** Run Mozilla rust-code-analysis on the same source scope and archive the raw output as the CQ2 baseline. Prioritize the known large evaluator, canonical validation, migration, composition, and historical adapter entrypoints.
2. **Structural pilot:** Run rustqual in report/non-failing mode. Inspect findings around pure core vs host adapter boundaries, tests, module cohesion, and duplication. Configure architecture layers to reflect the actual workspace and native-tool composition root—not the historical three-plugin layout.
3. **Ratchet evaluation:** Trial rust-oleum after resolving the lockfile decision and defining a project-owned policy. Compare its baseline behavior with current CI and ensure only new regressions fail.
4. **Style experiment:** If desired, run selected cargo-quality analyzers without auto-fixes or formatter changes. Reject rules that penalize idiomatic blank-line grouping or useful local comments.
5. **Adoption decision:** Add only checks whose findings are reproducible, actionable, and aligned with the product architecture. Pin tool versions in CI and preserve raw reports for comparison.

### Proposed CQ2 evidence outputs

- Raw complexity/maintainability metrics and a ranked hotspot list.
- rustqual findings reviewed and classified as valid, false positive, or policy mismatch.
- A ratchet proposal defining accepted baseline, thresholds, exclusions, and update procedure.
- A written decision for each candidate tool: adopt, limited use, or reject.

**Status at initial tool assessment:** Recommendations documented; no candidate tool execution or CI integration had yet been performed. The subsequent report-only pilot is recorded below.


## CQ2 tool pilot — CI wiring

**Status:** Workflow added; automatic audit-branch execution configured, run result not yet verified.

- Workflow: `.github/workflows/cq2-tool-pilot.yml`
- Branch: `quality/cq2-tool-pilot`
- Trigger: automatic push to `quality/cq2-tool-pilot`, plus manual `workflow_dispatch`; the manual run accepts a branch/tag/commit.
- Analyzer: `rustqual 1.8.3`, installed with `--locked`.
- Mode: `--no-fail --verbose`; findings are reported but do not fail the job. Installation/runtime errors still fail the job.
- Output: `quality-reports/rustqual-report.txt` and tool version, uploaded as a 30-day Actions artifact.
- Permissions: read-only repository contents.
- No application source, normal CI workflow, lockfile, or analyzer configuration was changed.

### Why Mozilla rust-code-analysis is not wired yet

The Mozilla CLI remains a measurement candidate, but I have not added an unverified install/build step. The currently surfaced CLI release is `0.0.25`; the project documentation describes building the CLI from its source workspace, while the repository issue tracker includes an open report that `cargo install rust-code-analysis-cli` does not compile. That is a reproducibility risk for a first audit workflow. Before wiring it, verify a supported installation path (preferably a maintained release binary or a pinned source build) on the exact Ubuntu runner and record the CLI version. The metrics command itself supports directory analysis and JSON export; the intended source scope is the workspace Rust crates, with raw output retained as an artifact.

### Execution result and manual review

**Status:** Automated pilot completed successfully; artifact retrieved and compared with all ten manual CQ2 findings.

- Workflow run: [45115723394](https://github.com/Mujshanan/forma-deform/actions/runs/45115723394), completed successfully on 2026-11-23.
- Job: `rustqual report-only analysis`; checkout, stable Rust setup, pinned rustqual install, analysis, and artifact upload all succeeded.
- Artifact: `cq2-rustqual-report`; rustqual version `1.8.3`.
- Summary: 433 functions, 150 findings, tool-generated score 53.1%. This is not a quality verdict or a count of defects.
- Manual comparison: CQ2-03, CQ2-04, CQ2-06, and CQ2-09 confirmed; CQ2-07, CQ2-08, and CQ2-10 partially confirmed; CQ2-01, CQ2-02, and CQ2-05 are outside rustqual's evidence scope. No manual finding was invalidated.
- Full triage, limitations, prioritized hotspots, and remediation proposal: [CQ2 rustqual pilot results](cq2-rustqual-results.md).

The report-only workflow remains on `quality/cq2-tool-pilot`; application source and normal CI were not modified. Remediation has not started. Mozilla rust-code-analysis was not included, and no Mozilla metrics are claimed.
