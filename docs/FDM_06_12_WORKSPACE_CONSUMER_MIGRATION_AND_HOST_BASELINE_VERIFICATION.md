# FDM-06.12 — Workspace Consumer Migration and Host Baseline Verification

**Status:** Consumer map established; migration plan and command effect/test ownership matrix published; host CI trigger PR opened but no run has appeared; S0 remains blocked  
**FDM parent:** FDM-06.11, PR #77, head `5ec4931b70efc1b6082f7e4516fe462017df5d4b`  
**Host baseline:** `Mujshanan/anicraft` `main` at `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Host tree:** `80071fb3f714944e49608668871d4cf53a6159a5`, complete, 413 Rust files  
**Change policy:** FDM documentation/CSV only. Host PR #1 adds one documentation file solely to request a CI baseline. No runtime code, workflow, dependencies, or release artifacts changed.

---

## 1. Executive result

FDM-06.12 advances S0 by establishing a source-grounded migration map across the host's primary mutation and consumer boundaries, and by assigning each of the 395 registered command IDs a proposed typed effect owner and a stable conformance-test identifier.

The exact host source still exposes the principal bypasses identified in FDM-06.10/06.11:

- `Session.doc` is public mutable state.
- `RunFn = fn(&mut Session, &Value)` is public, and `CommandSpec.run` is public.
- `Document::alloc_id`, `Document::timeline_mut`, `Timeline::layer_mut`, `Layer::keyframe_mut`, and `Keyframe::elements_mut_where` are public mutation APIs.
- GUI and CLI code have direct access to the live Session.
- At least one production GUI tool mutates `app.s.doc` directly instead of using the command/gesture transaction route.

A documentation-only host PR was opened to request a real CI baseline. It is open and unmerged, but the GitHub Actions API still reports **zero workflow runs and zero check runs** for its head at the time this report was assembled. Therefore no compile-backed migration result or host CI pass can be claimed.

**Disposition:** FDM-06.12 does not close the full S0 gate. The source-level migration map and planned test ownership are established; executable API migration, callback-body effect verification, compile proof, and actual host baseline remain open. S1 runtime implementation is still not authorized.

---

## 2. Pinned source and scan method

| Item | Value |
|---|---|
| Host repository | `Mujshanan/anicraft` |
| Base ref | `main` |
| Exact source SHA | `2df7775907d1458ce653e7e6314ec8a5d7d565f5` |
| Tree SHA | `80071fb3f714944e49608668871d4cf53a6159a5` |
| Tree truncated | `false` |
| Rust source files | 413 |
| Command spec sources registered | 52 |
| Registered command IDs | 395 |

The source tree and relevant blobs were fetched by their exact Git object IDs. The GitHub code-search endpoint returned no usable matches, including for known symbols. That failure was not treated as proof of absence; relevant files were fetched directly from the pinned tree and inspected.

### Evidence classification

- **Directly observed:** exact declarations, field visibility, method signatures, call sites, and worker/dispatch paths in fetched source.
- **Mapped:** the consumer's proposed replacement API and migration stage.
- **Not yet verified:** exhaustive workspace-wide type-use inventory, callback transitive write graph, compile impact, and runtime tests. These are explicitly left open rather than inferred from partial samples.

---

## 3. Public API containment inventory

### 3.1 Engine API exposed at the pinned commit

| API | Observed declaration/exposure | Required migration |
|---|---|---|
| `Session.doc` | `pub doc: Document` | Private persistent state; immutable `document()`/snapshot view |
| `RunFn` | Public function-pointer alias taking `&mut Session` | Private callback type; internal command execution only |
| `CommandSpec.run` | Public callback field | Public descriptor contains metadata only; no callable mutation capability |
| `CommandSpec.mutates` | Public boolean | Replace as authority with typed declared effects plus runtime effect journal |
| `Session::execute` | Public mutable entry point | Retain as owner-checked transaction entry point |
| `Session::begin_gesture/end_gesture` | Public mutable entry points | Replace with owner-checked gesture transaction API |
| `Session::replace_document` | Public mutable entry point | Route through atomic persistent-bundle replacement |
| `Session::undo/redo` | Public mutable entry points | Restore complete persistent bundle; advance runtime epoch |
| Session editor/context fields | Public fields include selection/state and context state | Split into controlled persistent, context, transient, and runtime accessors |
| Document nested mutable APIs | Public `alloc_id`, `timeline_mut`, `layer_mut`, `keyframe_mut`, `elements_mut_where` | Restrict live-document mutation to scoped host transaction draft |

The most important API proof is the pinned engine declaration:

```rust
pub type RunFn = fn(&mut Session, &Value);

pub struct CommandSpec {
    // metadata ...
    pub mutates: bool,
    pub run: RunFn,
}

pub struct Session {
    pub doc: Document,
    // ...
}
```

The actual result type on `RunFn` is the engine's `Result<Value>`; the abbreviated excerpt above emphasizes the exposed capability. Public callback invocation and public mutable document access make `Session::execute` a convention, not an enforceable exclusive mutation boundary.

### 3.2 Document mutation APIs

The pinned `crates/doc/src/lib.rs` exposes:

- `Document::alloc_id(&mut self)`
- `Document::timeline_mut(&mut self, scene)`
- `Timeline::layer_mut(&mut self, layer_id)`
- `Layer::keyframe_mut(&mut self, index)`
- `Keyframe::elements_mut_where(&mut self, predicate)`

These APIs are useful for constructing/importing an offline Document and for pure document-model tests. They must not remain usable to mutate the authoritative live Document behind Session's back.

**Required distinction:** preserve a Document builder/edit API for unowned drafts and import construction, while the live Session exposes only immutable document views and a transaction-scoped mutable draft capability. Do not solve this by indiscriminately making every document-model builder API private; importers, format decoders, and document tests have legitimate offline construction use cases.

---

## 4. Workspace consumer migration map

### 4.1 Consumer families

| Consumer family | Pinned source evidence | Current dependency | Target boundary | Migration stage |
|---|---|---|---|---|
| GUI application owner | `src/app.rs`, `src/main.rs`, `src/tabs.rs` | Owns live `App.s`; many reads from `s.doc`; command dispatch and document lifecycle | Owner-only Session facade; immutable read view; transaction request API | S1–S2 |
| GUI panels and tools | `src/actions_panel.rs`, `src/panels.rs`, `src/library.rs`, `src/timeline.rs`, `src/stage.rs`, `src/gradient_tool.rs`, `src/bone_tool.rs`, `src/warp_tool.rs`, `src/motion_editor.rs`, `src/pathtools.rs`, `src/bind_tool.rs`, `src/swatches_panel.rs`, `src/rulers.rs`, `src/onion.rs`, `src/audio_keys.rs`, `src/data_ui.rs`, `src/shape_envelope.rs`, `src/widthtool.rs`, `src/scale_grid.rs`, `src/colour_panel.rs`, `src/drop.rs`, `src/convert_symbol.rs`, `src/ease_editor.rs`, `src/camera_tool.rs`, `src/publish_panel.rs`, `src/doc_settings.rs`, `src/find_panel.rs`, `src/start.rs`, `src/menus.rs`, `src/plugins_ui.rs`, `src/plugin_export_ui.rs`, `src/remote.rs` | Direct immutable reads of `app.s.doc`; gesture begin/end; command calls; editor state reads | `DocumentView`, controlled context/selection methods, owner-thread transaction and gesture requests | S1–S4 |
| CLI request handlers | `crates/cli/src/lib.rs` | Public handlers accept `&mut Session`; JSON-lines serve loop processes requests sequentially | Explicit CLI Session owner; request dispatch into the same transaction coordinator | S1–S3 |
| CLI MCP backend | `crates/cli/src/mcp.rs` | Implements host/backend traits over Session; reads `s.doc`; executes calls/tools through Session | Immutable query facade plus owner-routed command requests | S1–S3 |
| Remote GUI/MCP | `src/remote.rs`, `src/app.rs` | Remote requests are polled/routed by app; app owns actual Session execution | Queue requests to app owner; never transfer mutable Session across worker boundary | S2 |
| Engine command modules | All registered command-spec sources listed in §5 | Callbacks receive `&mut Session`; direct `s.doc` and nested mutable document APIs | Private callback execution over transaction draft; each callback declares effects and records actual effects | S1–S4 |
| Plugin execution | `crates/engine/src/cmds_plugin.rs`, `crates/plugins/src/runtime.rs` | `SessionHost` executes nested commands through `execute_nested` | Same outer draft, journal, history boundary and commit; no independently published nested mutation | S3–S4 |
| Macro execution | `crates/engine/src/cmds_macro.rs` | Executes command sequences through Session | Same outer transaction semantics as plugin nested calls | S3–S4 |
| Autosave/recovery | `src/autosave.rs`, `src/tabs.rs`, `crates/engine/src/recovery.rs` | Worker receives cloned Document and path; recovery state is Document-oriented | Immutable complete persistent-bundle snapshot with bundle revision | S6–S7 |
| Rendering | `crates/engine/src/render_service.rs`, `src/app.rs`, `src/test_movie.rs` | Render/evaluation work receives cloned Document/display inputs | Immutable snapshot only; owner-side result freshness acceptance | S8–S9 |
| File format | `crates/engine/src/format.rs`, `crates/engine/src/cmds_file.rs` | Serialize/deserialize Document; document replacement and save paths | Persistent bundle codec, opaque extension retention, atomic replacement | S5–S7 |
| Import/export builders | `crates/xfl/src/import.rs`, `crates/engine/src/transfer.rs`, `crates/engine/src/plugin_export.rs` | Construct or transform standalone Document/transfer payloads | Keep offline builders; route final publication/remap through owner transaction | S5–S7 |
| Tests and fixtures | `crates/engine/src/*_tests.rs`, `crates/engine/tests/*`, `src/*_tests.rs`, CLI tests | Many tests construct mutable Documents and inspect Session.doc | Separate offline Document builder tests from live Session API conformance tests; add test-only owner harness where needed | S1–S9 |
| Independent libraries | `crates/geom`, `crates/render`, `crates/svg`, `crates/text`, `crates/audio`, `crates/actions`, `crates/xml` | Consume document data or geometry/render APIs, not authoritative Session mutation | Keep host-independent immutable data APIs; do not add Session dependency | No Session migration expected |

This map is a migration ownership map, not a claim that every line-level use in every Rust file has already been rewritten. The 413-file tree is complete, but a compile-backed workspace-wide call-site report still requires running the host build after the API changes; those changes are not part of this documentation milestone.

### 4.2 Concrete GUI mutation bypass

The pinned `src/gradient_tool.rs` contains a production path that:

1. calls `app.s.doc.alloc_id()`;
2. pushes a bitmap directly into `app.s.doc.bitmaps`.

This is outside the ordinary command callback/gesture transaction boundary. It must be converted to a host command or owner-scoped transaction operation before the public mutable field can be removed.

Other GUI modules call `begin_gesture`/`end_gesture` directly. These are not necessarily bypasses today, but they are API consumers that must migrate to the owner-checked gesture protocol. Their visible intermediate edits and cancellation behavior must remain correct.

### 4.3 CLI, MCP and remote ownership

- CLI `handle_request`/`serve` functions borrow `&mut Session` and process the request stream sequentially. This is a viable single-owner model if the API explicitly establishes the caller as the owner.
- MCP `Host`/`Backend` implementations are currently implemented for Session and read the live document. Query methods should receive immutable snapshots; mutation methods should enqueue or execute on the owning request thread.
- GUI remote requests are polled through the app. The remote client thread must not gain a mutable Session reference.
- Plugins and macros must not create a second transaction owner: nested execution joins the active outer transaction.

### 4.4 Worker boundary

Autosave and rendering already use cloned inputs rather than owning `&mut Session`. Preserve that separation and strengthen the payload from Document-only to the complete persistent bundle. Workers return results; the Session owner decides whether a result is still current and may publish.

---

## 5. Command registry and effect/test ownership

FDM-06.11 enumerated 395 unique command IDs from 52 registered command-spec sources. FDM-06.12 adds:

`docs/fdm-06-12/command-effect-coverage.csv`

This is a single combined, ID-sorted registry with:

- source module and declared `mutates` flag;
- initial source-level effect class;
- proposed typed effect contract;
- proposed effect owner;
- stable planned conformance test ID `FDM-HOST-CMD-001` through `FDM-HOST-CMD-395`;
- explicit coverage status.

### 5.1 Effect owner rules

| Effect contract | Proposed owner | Required runtime behavior |
|---|---|---|
| `DocumentDraftMutation` | Session transaction owner | Draft mutation, effect journal, validation, atomic publish/history/epoch |
| `DocumentReplacement` | Session document-replacement coordinator | Atomic full-bundle replacement and identity/session rules |
| `HistoryRestore` | Session history coordinator | Full persistent-bundle restore; monotonic runtime freshness |
| `SessionContextMutation` | Session context owner | Controlled context-generation update; no document dirty state unless settlement writes artwork |
| `ClipboardMutation` | Session transient-state owner | Transient state update; explicit source identity for paste |
| `SelectionMutation` | Session transient-state owner | Controlled selection update; no document history unless persistent settlement occurs |
| `PreferenceOrPluginMutation` | Preference/plugin-state owner | Separate persistence and rollback contract; not document history |
| `ExternalFileEffect` | Host external-effect coordinator | Staged output and explicit partial-failure semantics |
| `NestedCommandTransaction` | Outer Session transaction owner | Join active draft/journal; no nested commit/history/epoch |
| `ReadOnlyQuery` | Immutable Session/query owner | No mutable Session or Document capability |

### 5.2 Interpretation of the 395 test IDs

The IDs in the CSV are **assigned test identifiers**, not evidence that 395 tests already exist or pass. Each is marked planned, with callback-body verification and executable test pending.

For a command to be considered covered, its test must establish at least:

1. exact declared effect category and owner;
2. all direct and transitive persistent/session/external effects;
3. rollback and no-op behavior;
4. target identity transitions and remaps where applicable;
5. history and evaluation-epoch expectations;
6. settlement side effects, including when `mutates=false`;
7. error behavior and external-effect partial failure where relevant.

The 131 false-flag callbacks are the highest-priority manual audit set because their declared flag does not reliably describe their actual effect. Do not auto-convert all 131 to read-only.

---

## 6. Host CI baseline verification

### 6.1 Workflow source

The pinned `.github/workflows/ci.yml` supports:

- `push` to `main`;
- `pull_request`;
- `workflow_dispatch`.

The workflow delegates the quality gate to `storytold/craft-common-actions/.github/workflows/gate.yml@main` and configures:

```sh
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

Its independent license job configures:

```sh
cargo deny --locked check licenses bans sources
```

### 6.2 Documentation-only host PR

To request a real baseline without changing Rust code or workflow configuration, the following host PR was opened:

- [AniCraft PR #1 — establish host CI baseline](https://github.com/Mujshanan/anicraft/pull/1)
- Branch: `audit/fdm-06-12-host-baseline`
- Base: `main` at `2df7775907d1458ce653e7e6314ec8a5d7d565f5`
- Head: `2b8ba9fd6b5d00e698c1516087632f7a6a4d935e`
- Change: one documentation file, 32 additions; no runtime/workflow/dependency changes
- State: open, draft, unmerged

### 6.3 Actual result at report time

The GitHub Actions API returned:

| Evidence | Result |
|---|---:|
| Workflow runs for host PR head | 0 |
| Check runs for host PR head | 0 |
| Host PR merged | No |

This is **not a passing baseline** and not a failing test result. The workflow trigger has been requested through a PR, but GitHub has not reported a run yet. Possible causes include Actions scheduling/permission configuration or delayed event processing; this report does not guess which.

No local cargo build/test was run in this environment. Do not infer successful compilation from the source scan.

**Baseline gate remains open** until at least the workspace test/Clippy job and cargo-deny job have actual conclusions on the exact host PR head or another explicitly accepted pinned baseline.

---

## 7. Migration plan and compile-backed proof

The migration must proceed in a dependency-safe order. This milestone does not implement these steps.

| Stage | Work | Required proof |
|---|---|---|
| M1 — Consumer containment | Introduce immutable DocumentView and controlled Session read APIs; inventory and migrate external `Session.doc` reads | Workspace compile; no external mutable field access |
| M2 — Mutation ownership | Owner identity/checks on execute, gesture, history, replacement, recovery and all mutation entry points | Wrong-owner tests; GUI/CLI/MCP routing tests |
| M3 — Callback encapsulation | Make RunFn and callback field private; command descriptors expose metadata only | Compile all 52 spec sources and every registry consumer |
| M4 — Document draft capability | Keep offline Document builders; remove mutable live Document escape hatches | Compile-time API tests; no live mutable Document reference escapes |
| M5 — Transaction settlement | Execute cut/floating/rig/motion/group settlement before commit | Regressions for all settlement and mutates=false cases |
| M6 — Identity and persistence | Add DocumentId, incarnation ledger, opaque extension envelope and checked allocator | Round-trip, migration, remap, Save As/Copy and history tests |
| M7 — History/recovery bundle | Move history, dirty state, save, autosave and recovery to full persistent bundle | Crash/reopen/undo/redo and byte-accounting tests |
| M8 — Freshness | Session instance, monotonic epoch and context generation; immutable evaluation snapshot | Stale result rejection and context invalidation tests |
| M9 — FDM host adapter | Synchronous FDM snapshot/evaluate/accept/apply path | Host conformance matrix; no async until separately approved |

### 7.1 Compile-backed migration requirement

A consumer map is not compile evidence. Once M1–M4 are separately authorized and implemented, require:

- `cargo check --workspace`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo deny --locked check licenses bans sources`;
- Windows CI as a mandatory platform for the Forma ecosystem;
- compile-fail/API tests proving external code cannot access `Session.doc`, call `CommandSpec.run`, or mutate a live Document outside a transaction;
- exact commit SHA and workflow URLs in the evidence record.

Do not make the compile gate pass by reintroducing a public mutable escape hatch or by weakening existing CI checks.

---

## 8. Remaining S0 gates

| Gate | FDM-06.12 result | Closure condition |
|---|---|---|
| Source tree and baseline pin | Closed | Already pinned and complete |
| 52 spec sources / 395 command IDs | Closed | FDM-06.11 evidence |
| Proposed effect owner + test ID for 395 commands | Closed as planning data | CSV contains 395 unique planned IDs; callback verification and executable tests remain separate |
| Complete callback transitive-effect audit | Open | Inspect each callback and settlement path; resolve all 131 false-flag cases |
| Workspace consumer migration map | Partial | Main consumer families and concrete call sites mapped; full type-use inventory and compile migration proof not run |
| Owner/dispatch contract | Design closed | Runtime owner enforcement not implemented |
| Host CI baseline | Open | PR #1 has zero workflow/check runs at report time |
| Non-command mutation coverage | Partial | Direct GUI bypass and major APIs identified; exhaustive compile-enforced coverage remains open |
| Opaque extension/persistent-bundle behavior | Design only | No host implementation or round-trip tests |

**S0 remains blocked.** FDM-06.12 is an evidence and migration-planning milestone, not a green light for runtime implementation.

---

## 9. Scope and preserved decisions

- No AniCraft Rust runtime code changed.
- No FDM Rust runtime code changed.
- The only AniCraft change is a documentation-only CI-trigger PR, kept open and draft.
- FDM changes are documentation and registry CSV only.
- No local cargo tests, Clippy, cargo-deny, benchmarks, or CI passes are claimed.
- Neither PR #1 in AniCraft nor the FDM milestone PR is merged.
- No S1–S9 implementation is authorized by this report.
- Native Asset Warp remains host-owned and independent of FDM.
- Smart Controller and Form Deform evaluation remain independent under CCC-01.5.
- No async evaluation, multi-deformer composition, or expanded shape scope is introduced.

---

## 10. Recommended next milestone

**FDM-06.13 — Callback-Level Effect Audit and S0 Evidence Closure.**

It should:

1. Trace the callback body and all transitive helpers for each command ID, prioritizing all 131 `mutates=false` entries.
2. Replace inferred CSV effect classes with verified effects and test ownership; keep unknown effects fail-closed.
3. Complete the direct mutable API consumer inventory, including every public Session field, callback invocation, and Document mutation method.
4. Resolve the direct GUI mutation bypass in `src/gradient_tool.rs` only after implementation authorization.
5. Obtain actual host CI results from AniCraft PR #1 or record a user-approved fallback if Actions remains unavailable.
6. Produce the final S0 go/no-go matrix.

**Implementation gate:** Do not start S1 runtime changes until callback effect coverage, complete consumer inventory, owner dispatch, and host baseline evidence are closed and separately authorized.
