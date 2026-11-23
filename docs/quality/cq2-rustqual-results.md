# CQ2 rustqual pilot — results and manual review

**Status:** Automated pilot completed; artifact retrieved and manually compared with the ten CQ2 source-review findings.  
**Repository:** `Mujshanan/forma-deform`  
**Branch / audited revision:** `quality/cq2-tool-pilot` / `edb65d5d11296146fd0aebca424bef37bc28d52a`  
**Analyzer:** `rustqual 1.8.3`  
**Workflow run:** [45115723394](https://github.com/Mujshanan/forma-deform/actions/runs/45115723394)  
**Artifact:** `cq2-rustqual-report` (retrieved; SHA-256 `da7c93f10ceb832572b30a735623b13c36d6e58a2d12eac4761095ed69420e63`)  
**Execution:** completed successfully, 2026-11-23 05:06:38–05:08:56 UTC. Install, analysis, and artifact-upload steps all succeeded.

## 1. Executive result

The pilot analyzed **433 functions** and reported **150 findings**, with a tool-generated quality score of **53.1%**.

| Dimension | Reported result | Interpretation |
|---|---:|---|
| IOSP | 88.5%; 35 violations | Function-level structure/organization flags; review individually |
| Complexity | 86.6%; 14 complexity, 25 magic-number, 1 nesting, 12 long-function, 3 unsafe, 3 error-handling flags | Useful hotspot signals, not defects by themselves |
| DRY | 89.1%; 14 duplicate, 4 fragment, 22 dead-code, 2 dead-type, 5 boilerplate findings | Requires semantic review before removing or consolidating code |
| SRP | 98.6%; 6 module findings | Useful secondary signal for module cohesion |
| Coupling | 99.8%; 1 structural finding | Analyzer modeled only two top-level modules: `crates` and `plugins`; this is too coarse to validate the intended internal architecture |
| Test Quality | 99.3%; 3 untested findings | Call-graph-only result; tool itself says coverage data could resolve uncertainty |
| Architecture | 100.0% | Not evidence of architectural correctness: no project-specific architecture rules were configured, and only two top-level modules were modeled |

The 53.1% score and 150-finding total must **not** be read as 150 defects or as a release-readiness score. The report combines warnings, heuristics, duplicate candidates, dead-code candidates, and function-level thresholds. It includes test code in the 433-function count. The run is a successful tool execution, not a code-quality pass/fail gate.

## 2. Highest-value confirmed hotspots

The following findings align with the manual review and deserve human-led remediation proposals:

| Priority | Source | Analyzer evidence | Manual assessment |
|---|---|---|---|
| P1 | `crates/smart-controller-core/src/canonical.rs:78`, `Controller::validate_local` | Cognitive complexity 51, cyclomatic 86, 103 lines | Strong confirmation of CQ2-06. Split validation into named private phases by invariant family only if validation order and error behavior remain unchanged. |
| P1 | `crates/smart-controller-core/src/evaluator.rs:111`, `evaluate` | Cognitive 88, cyclomatic 36, nesting 5, 306 lines | Strong confirmation of CQ2-04. This is the clearest structural hotspot. Extract phases around already documented evaluation stages; preserve diagnostics, ordering, fallback, and admission semantics. |
| P1 | `crates/smart-controller-core/src/composition.rs:141`, `compose_weighted_blend` | Cognitive 24, cyclomatic 16, 88 lines, seven `expect` calls flagged | Strong confirmation of CQ2-09. Review each `expect` invariant before changing it; extract typed-value blend paths and policy predicates without altering conflict/diagnostic behavior. |
| P1 | `crates/smart-controller-core/src/migration.rs:36,189` | `CanonicalDocument::validate`: cognitive 22; `migrate_unversioned_params`: cyclomatic 16, 90 lines | Strong confirmation of CQ2-03. Extract validation/migration phases while retaining schema compatibility and lossless round-trip behavior. |
| P2 | `crates/smart-controller-core/src/validation.rs:4` | Cognitive 22, cyclomatic 17 | Additional validation hotspot; review alongside canonical validation to avoid duplicating invariants or creating competing validation paths. |
| P2 | `plugins/smart-controller/src/input_provider.rs:48` | Cognitive 19, nesting 4, one `expect` flagged | Confirms CQ2-10's host-input seam is a worthwhile readability target. Preserve missing/invalid/unavailable distinctions and invocation-scoped sampling. |
| P2 | `crates/smart-controller-core/src/mapping/canonical.rs:110,330` | Mapper evaluate 77 lines; pose-set interpolation flagged | Additional mapping hotspot. Keep mapping policy and typed interpolation behavior explicit. |
| P2 | `crates/smart-controller-core/src/legacy.rs:33-224` | Multiple validation/evaluation functions flagged | Review as compatibility maintenance, not as a reason to expand or modernize the legacy V1 path. |
| P2 | `plugins/bone-query/src/lib.rs` | 200 production lines and four function clusters | Supports CQ2-07's concern about mixed responsibilities. First decide whether this historical query plugin remains supported; if it does, separate response interpretation from report rendering. |
| P2 | `plugins/form-deform/src/lib.rs:40`, `run` | Function flagged; logic 6, 11 calls, cyclomatic 5 | Partial support for CQ2-08. It is an end-to-end adapter, but the score alone does not justify splitting it until the native-tool lifecycle and supported adapter status are verified. |

Other flagged functions include canonical graph construction, legacy interpolation/range helpers, preview helpers, angle/value interpolation, Form Deform numeric parsing, Smart Controller preview orchestration and bone-rotation sampling, and the historical Smart Controller V1 `run` entrypoint. See the raw artifact for the full list of 35 IOSP violations and per-function metrics.

## 3. Comparison with the ten manual CQ2 findings

| Manual finding | Result | Reason |
|---|---|---|
| CQ2-01 — compressed formatting in Form Deform core | **Not assessed by rustqual** | This pilot is not a formatter/style audit. `evaluate_pin_position` is flagged as a complexity/IOSP hotspot, but that does not verify whitespace or formatting. Run stable `cargo fmt --all -- --check` separately. |
| CQ2-02 — adapter imports separated from module header | **Not assessed** | rustqual did not evaluate import ordering/spacing. Retain the manual observation; do not infer it was cleared. |
| CQ2-03 — dense migration validation | **Confirmed** | `CanonicalDocument::validate` and `migrate_unversioned_params` are both flagged with elevated cognitive/cyclomatic complexity and/or length. |
| CQ2-04 — large evaluator | **Strongly confirmed** | The 306-line `evaluate` function has cognitive complexity 88, cyclomatic 36, nesting 5. The module-level SRP warning is a separate signal. |
| CQ2-05 — lifecycle labels across V1/canonical/native paths | **Not assessed** | Static function metrics cannot establish the product lifecycle or host integration traceability. This remains a documentation/evidence task. |
| CQ2-06 — canonical controller validation concentration | **Strongly confirmed** | `Controller::validate_local` has cognitive 51, cyclomatic 86, and 103 lines. |
| CQ2-07 — historical Bone Query responsibilities | **Partially confirmed** | rustqual reports four function clusters in a 200-production-line module. It supports reviewing cohesion, but cannot decide whether the plugin is retained or superseded. |
| CQ2-08 — Form Deform plugin end-to-end entrypoint | **Partially confirmed** | The `run` function is flagged, but its reported logic/cyclomatic values are modest. Treat this as a review prompt, not a mandate to add layers to a small adapter. |
| CQ2-09 — composition policy branches | **Confirmed** | `compose_weighted_blend` has cognitive 24, cyclomatic 16, 88 lines, and seven `expect` calls flagged. |
| CQ2-10 — adapter/input/preview seams | **Partially confirmed** | Host input collection and bone sampling are flagged; preview-plan functions are not. The tool does not prove dependency direction or host mutation boundaries. |

No manual finding is invalidated by this pilot. The two formatting observations and the lifecycle/host-traceability observation are simply outside rustqual's evidence scope.

## 4. DRY, dead-code, and test-quality findings: treat as candidates

rustqual reported 14 duplicate candidates, four repeated fragments, 22 dead-code candidates, two dead types, and five boilerplate patterns. Examples include:

- Two evaluator tests reported as exact duplicates: `unused_external_snapshot_samples_are_still_type_checked` and `controller_output_samples_cannot_be_injected_through_external_snapshot`.
- Two migration tests reported as exact duplicates despite distinct names and potentially distinct compatibility intent.
- Near-duplicate helper candidates spanning legacy geometry, validation, input snapshot, and evaluator tests.
- Uncalled API candidates such as SDK `selected_elements` and `movie`, mapper pose-library accessors, and compatibility exports.
- Test-only candidates such as diagnostic builders, migration helpers, schema helpers, and adapter helpers.

**Do not delete these automatically.** In this project, public compatibility surfaces, host ABI exports, migration helpers, and tests protecting distinct historical input shapes can be intentionally unreferenced within the scanned call graph. Before any removal or consolidation:
1. Determine whether it is part of the public Rust API, WASM ABI, native-tool integration, or serialized compatibility contract.
2. Search all workspace and host consumers, including external consumers not in this repository.
3. Compare the test bodies and assertions; same call shape does not establish equivalent behavior.
4. Retain tests that prove different malformed-input, schema, identity, or fallback cases.
5. Only then consolidate genuinely redundant private code.

The three `anicraft-plugin-sdk` functions marked untested (`export`, `manifest_ptr`, `alloc`) are ABI/unsafe-boundary candidates. rustqual's own note says call-graph analysis can miss macro/trait-object/generated reachability and suggests an LCOV coverage report. Do not interpret these flags as proof the exports are untested; inspect ABI-level tests and add coverage evidence before changing them.

## 5. Tool limitations exposed by this run

- **Architecture result is not meaningful as a project architecture audit.** The analyzer reported only two modules (`crates`, `plugins`) and no circular dependencies. That does not validate the internal crate graph, core/adapter boundaries, native AniCraft ownership, or the historical three-plugin → unified product → native tool evolution.
- **No project-specific layer/forbidden-edge policy was supplied.** The 100% architecture score therefore must not be used as an approval signal.
- **No coverage input was supplied.** The three untested findings are call-graph-only and explicitly uncertain.
- **No Mozilla rust-code-analysis metrics were run.** The artifact contains rustqual output only.
- **No formatter, Clippy, tests, WASM build, dependency audit, or application source modifications were part of this workflow.**
- The analyzer's magic-number warnings include test fixture values and algorithm constants (for example numeric constants in deterministic RNG and interpolation code). Name constants where that improves domain clarity; do not replace meaningful mathematical constants with arbitrary names solely to silence a heuristic.
- The `expect` warnings in weighted blending require invariant review. A blanket conversion to `unwrap_or`, silent defaults, or swallowed errors would reduce correctness and diagnosability.

## 6. Remediation proposal

Keep this as a staged, behavior-preserving quality effort:

1. **Formatting-only patch:** run stable `cargo fmt --all -- --check`; if needed, produce a formatting-only diff with no semantic edits. This separately addresses CQ2-01/02.
2. **P1 private decomposition proposal:** `Controller::validate_local`, evaluator `evaluate`, and `compose_weighted_blend`. Extract named private helpers around existing semantic phases. No public API changes.
3. **Migration/validation proposal:** decompose `CanonicalDocument::validate` and `migrate_unversioned_params`; preserve legacy format acceptance, migration diagnostics, and round trips.
4. **Adapter readability proposal:** improve host input collection and sampling only where it makes host response states clearer; keep host queries and mutation at the adapter boundary.
5. **Compatibility/DRY review:** audit all dead-code/duplicate candidates against public API, ABI, external host use, and migration tests before any removal.
6. **Architecture evidence:** separately trace the actual native AniCraft Forma Deform entrypoint and host call path. Do not infer native integration from the historical plugin entrypoints or this analyzer's architecture score.
7. **Verification after approved changes:** run Rust workspace tests, stable formatting check, Clippy under an agreed policy, and release WASM builds. Preserve the current successful CI baseline for comparison.

### Proposed acceptance criteria for structural changes

- Public API and serialized schema remain unchanged unless separately approved.
- Legacy migration and compatibility tests remain intact and pass.
- Evaluation order, graph admission, deterministic ordering, fallback, diagnostics, and composition conflict behavior are unchanged.
- No new host mutation is introduced into pure core evaluation.
- Each extracted helper has one named responsibility and reduces caller complexity without hiding policy in generic abstractions.
- Tests cover behavior at the same semantic boundaries as before; test count alone is not an acceptance criterion.

## 7. Decision on candidate tools

| Tool | Decision after pilot |
|---|---|
| rustqual 1.8.3 | **Adopt for report-only CQ audits**, with pinned version and human triage. Do not make its aggregate score a CI gate. Configure architecture rules and coverage input before interpreting those dimensions. |
| Mozilla rust-code-analysis | **Still pending.** Not included in this artifact. Verify a reproducible pinned runner installation/build before adding a separate measurement job. |
| rust-oleum | **Defer.** Resolve lockfile policy and define project-owned ratchet thresholds before trial. |
| cargo-quality | **Do not adopt its formatter or blocking style gate.** Stable rustfmt remains authoritative; individual analyzers require a separate dry-run justification. |

## 8. Final CQ2 status

**Automated pilot: complete. Manual comparison: complete. Remediation: not started.**

This report records tool evidence and a prioritized proposal; it does not claim code fixes or local reproduction. The application source and normal CI workflow were not modified by the pilot. The report-only workflow remains on `quality/cq2-tool-pilot` and is not merged to `main`.

Next CQ2 work should begin with the separate formatting check and a reviewable, formatting-only patch proposal, followed by a narrowly scoped decomposition proposal for the three strongest canonical-core hotspots. Native integration traceability must be established before changing plugin lifecycle or ownership boundaries.
