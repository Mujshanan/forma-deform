# FDM-06.13 — S0 Closure Review Evidence Addendum

**Disposition: NOT READY FOR S0 CLOSURE. Submit this package for formal review as a blocked evidence submission.**  
**Host:** Mujshanan/anicraft  
**Pinned commit:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Tree:** `80071fb3f714944e49608668871d4cf53a6159a5`  
**Scope:** Reproducible source audit, non-command mutation-route inventory, correction of prior evidence, and explicit S0 gate disposition.  
**Change policy:** Documentation only. No runtime changes, no test-pass claims, no implementation authorization, and no merge authorization.

## 1. Review decision requested

Review this as the current S0 closure evidence submission. **S0 remains OPEN.** The source baseline and registry are verified, and material mutation-boundary risks are confirmed. The callback-level transitive effect audit, exhaustive consumer proof, compiler-backed migration evidence, and host CI baseline remain unproven. Do not approve S0 closure from this package.

FDM-06.6 remains design-only, not implemented or verified. Even after S0 closure, implementation remains blocked until FDM-06.6 is implemented and verified and the user explicitly authorizes the implementation stage.

## 2. Reproduced baseline

The uploaded AniCraft archive was extracted and independently checked:

| Check | Result |
|---|---|
| ZIP integrity | `unzip -t`: no errors |
| Embedded Git HEAD | `2df7775907d1458ce653e7e6314ec8a5d7d565f5` |
| Embedded Git tree | `80071fb3f714944e49608668871d4cf53a6159a5` |
| Working tree | clean |
| Rust source files | 413 |
| Cargo manifests | 21 |
| Providers in `engine::commands()` | 52 |

This matches the source SHA/tree pinned in FDM-06.10 through FDM-06.12.

## 3. Command registry and callback scan

The published FDM-06.11/06.12 registry contains 395 unique command IDs: 264 declare `mutates=true`, 131 declare `mutates=false`, with no duplicate IDs. The 52 provider list and 413 Rust-file count were independently reproduced from the archive.

A lexical callback-body locator was run against the 395-row registry. It located 95/131 false-flag callback bodies and 151/264 true-flag callback bodies. This is only source triage: closures, literal `CommandSpec` forms, imported callbacks, helper calls, and transitive effects are not fully resolved. A located callback with no direct-write token is not proof of purity. The complete row-level lexical scan is included in the downloadable evidence bundle, not represented as compiler-backed proof.

### 3.1 Concrete false-flag counterexamples

The `mutates` boolean is not a safe transaction/effect classification:

- `desktop.integrate` and `desktop.remove` are registered with `mutates: false` in `crates/engine/src/cmds_desktop.rs`, while their documented behavior writes/deletes user desktop files and refreshes desktop databases.
- False-flag commands including brush/filter preset save, rename and export; swatch export; macro rename/delete; recent-file preference changes; plugin install/remove/reload; undo/redo; document open/new/recovery; and playhead/selection commands are not equivalent to read-only queries.
- `Session::execute` enters the false-flag branch at `crates/engine/src/lib.rs:767-772`, runs the callback, then calls `settle_cut`, `hold_group_frame`, and `settle_floating` before returning without the ordinary mutating-command snapshot/rollback/history block.
- `settle_floating` at `crates/engine/src/lib.rs:844-889` can merge items into `self.doc` and increment `state`/`revision`; it is reached from the false-flag branch.

Therefore all 131 false-flag commands require individual typed effect classification. The boolean must not remain the transaction authority.

## 4. Non-command mutation routes confirmed

| Surface | Pinned source evidence | Required closure proof |
|---|---|---|
| Live document escape | `Session.pub doc: Document`, `engine/src/lib.rs:391-393` | Remove unrestricted mutable access |
| Callable command escape | Public `RunFn = fn(&mut Session, &Value)` and `CommandSpec.pub run`, `129-145` | Prevent callback invocation outside owner transaction |
| Gesture preview/finalize/cancel | Public `begin_gesture/end_gesture`, `648-679`; `revert_gesture`, `1019-1033` | One transaction/freshness model for preview, commit and cancel |
| Undo/redo | Public methods, `1065-1088` | Restore persistent bundle while advancing non-restorable runtime freshness |
| Document replacement | Public `replace_document`, `696-718` | Atomic document/identity/extension replacement |
| Evaluation context | Public `set_frame`, `1188-1191`, and public scene/frame/editing fields | Separate context generation/stamp; document revision alone is insufficient |
| Nested document mutation | Public `Document::alloc_id`, `doc/src/lib.rs:2103-2107`; `timeline_mut`, `2123-2130`; `Timeline::layer_mut`, `1658-1660`; `Layer::keyframe_mut`, `1581-1584` | Restrict live-document mutation to transaction-scoped host capability |
| Settlement | `settle_cut`, `heal_cut`, `settle_floating`, rig/motion settlement in `engine/src/lib.rs` | Include all settlement before commit/rollback and receipt publication |
| External effects | Desktop, file, recovery, plugin, macro, brush/filter/swatch commands | Stage/compensate effects and classify ownership |
| Workers | Render/recovery/autosave and stage consumers | Prove immutable snapshots and serialized freshness-checked result acceptance |

The accompanying evidence bundle includes a row-level mutation-entrypoint CSV. The inventory is source-grounded but not compiler-proven exhaustive.

## 5. Correction required to FDM-06.12 PR #79

PR #79's body states that there is a “production GUI direct mutation in `src/gradient_tool.rs`.” The pinned source does not support that statement:

- `src/gradient_tool.rs:312-313` allocates an ID and pushes a bitmap, but those lines are inside `#[cfg(test)] mod tests`, beginning at line 285.
- `src/trace_dialog.rs:156-158` and `src/mixed.rs:351-353` contain similar setup writes inside test modules.
- The production gradient-tool path uses `begin_gesture`, `Session::execute("paint.transform_fill", ...)`, and `end_gesture`; it is not a direct production document-tree write.
- A bounded grep of production `src/` found these three `alloc_id` call sites and no production call to `timeline_mut`, `layer_mut`, `keyframe_mut`, or `elements_mut_where`. This is not compiler-backed proof of absence.

The API-level bypass remains real because `Session.doc` and public Document mutators remain public. Correct the specific production-caller claim; do not discard the type-level risk.

## 6. Build and CI evidence

The audit environment has neither `rustc` nor `cargo` installed (both commands returned “command not found”). These were not run and no pass is claimed:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo deny --locked check licenses bans sources
```

GitHub reports zero workflow runs and zero status contexts for the pinned host commit. AniCraft documentation-only PR #1 is open at head `2b8ba9fd6b5d00e698c1516087632f7a6a4d935e`; that exact head also reports zero workflow runs and zero status contexts. There is no host Windows CI pass to review.

FDM PR #79's documentation-only CI run `45181005594` succeeded. That verifies only the FDM documentation branch, not the AniCraft host build/test baseline.

## 7. S0 gate matrix

| Gate | Evidence | Result |
|---|---|---|
| Source identity/integrity | ZIP check, embedded SHA/tree, clean working tree | **PASS** |
| Command registry aggregation | 52 providers; 395 unique IDs | **PASS at registry granularity** |
| Callback direct/transitive effects | 395-row registry and lexical scan; 36 false-flag callback bodies unresolved; transitive effects unproven | **OPEN** |
| False-flag semantic safety | External effects and document settlement reachable in false branch | **FAIL / OPEN** |
| Non-command mutation inventory | APIs, gestures, settlement, history, replacement, context, external effects, workers mapped | **PARTIAL** |
| Public API consumer proof | Source map/grep only; not compiler-backed | **OPEN** |
| Transaction ownership | Public mutable Session/Document APIs and false-flag settlement path remain | **FAIL / OPEN** |
| Host tests/Clippy/cargo-deny | Not run locally; no host workflow evidence | **OPEN** |
| Exact-baseline Windows CI | No run/status for base or host PR #1 | **OPEN** |
| S0 closure | Mandatory gates remain open | **NOT CLOSED** |

## 8. Required actions before S0 closure

1. Correct PR #79's production `gradient_tool` assertion.
2. Resolve all 36 remaining false-flag callback bodies and inspect helper calls transitively; complete equivalent effect review for all other commands.
3. Complete the non-command mutator inventory across gestures, import/paste/transfer, plugin/macro nesting, history, recovery/open/new, autosave and public Document APIs.
4. Obtain compiler-backed consumer proof for GUI, CLI, MCP, remote, plugin host, tests, autosave, history and rendering.
5. Obtain exact-baseline host tests, Clippy, cargo-deny and passing Windows CI, or submit a separately justified baseline exception for review.
6. Only then issue a distinct S0 closure decision.

## 9. Explicit non-authorization

This addendum authorizes no runtime changes, evaluator wiring, native host integration, adapter bypass, asynchronous evaluation, apply/bake, Native Asset Warp changes, Smart Controller/FDM composition, merge, or release. FDM-06.6 remains unimplemented and unverified. User authorization for implementation has not been granted.
