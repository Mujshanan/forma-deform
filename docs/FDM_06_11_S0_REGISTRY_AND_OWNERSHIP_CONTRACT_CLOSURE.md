# FDM-06.11 — S0 Registry and Ownership Contract Closure

**Status:** Command-source registry completed; ownership and effect-class contract defined; overall S0 implementation gate remains blocked  
**Parent audit:** FDM-06.10, PR #75  
**AniCraft repository:** `Mujshanan/anicraft`  
**Verified host baseline:** `main` = `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Tree SHA:** `80071fb3f714944e49608668871d4cf53a6159a5` (`truncated=false`)  
**Scope:** Close the command-source/ID inventory, classify command entry effects, define the enforceable Session ownership contract, reconcile known non-command mutators, and state the remaining S0 evidence gaps.  
**Change policy:** Documentation and CSV registry only. No AniCraft runtime code, no FDM runtime code, no merge, and no implementation authorization.

---

## 1. Executive result

The command registry has now been enumerated from the exact `commands()` registration in the pinned AniCraft source:

| Measure | Result |
|---|---:|
| Registered command-spec sources | 52 |
| Command entries enumerated | 395 |
| Unique command IDs | 395 |
| Duplicate command IDs in the enumerated registry | 0 |
| `mutates: true` declarations | 264 |
| `mutates: false` declarations | 131 |
| Unparsed/unknown mutation flags | 0 |

The 395 rows are stored as six machine-readable CSV segments under `docs/fdm-06-11/`. Every row carries command ID, source module, declared `mutates` value, initial effect class, entry owner, and review status.

**Important:** the old `mutates` boolean is not an effect manifest. The 131 false-flag commands include document replacement, history restoration, filesystem writes/deletes, preference/plugin state, context changes, clipboard/selection state, and read-only queries. It cannot safely decide whether a command needs a transaction, history, invalidation, or external-effect protocol.

The S0 ownership contract is now explicit: one Session owner serializes all mutation; public mutable access and callable command callbacks must be removed from the external API; command settlement must complete before atomic publication; nested plugin/macro commands share the outer transaction; workers receive immutable snapshots only.

**S0 is not fully green.** The command-source inventory and owner contract are defined, but the complete workspace consumer migration inventory is not yet verified, and the pinned AniCraft commit has no GitHub Actions workflow run or check-run evidence. No passing baseline is claimed.

---

## 2. Exact source baseline

| Item | Verified value |
|---|---|
| Repository | `Mujshanan/anicraft` |
| Branch | `main` |
| Commit | `2df7775907d1458ce653e7e6314ec8a5d7d565f5` |
| Commit message | Merge PR #525, principal review of AniCraft's own decisions |
| Tree SHA | `80071fb3f714944e49608668871d4cf53a6159a5` |
| Recursive tree complete | Yes (`truncated=false`) |
| Rust files in tree | 413 |
| Command-spec sources in `commands()` | 52 |

The current `main` ref was fetched again during this milestone and still resolves to the same commit. The FDM-06.10 host pin is therefore unchanged.

### 2.1 The 52 registered spec sources

The order below is the actual `commands()` registration order in `crates/engine/src/lib.rs`:

1. `cmds_file`
2. `plugin_export`
3. `cmds_envelope`
4. `publish`
5. `cmds_recent`
6. `cmds_desktop`
7. `cmds_drop`
8. `cmds_recovery`
9. `cmds_timeline`
10. `cmds_layer`
11. `cmds_camera`
12. `parenting`
13. `cmds_bone`
14. `cmds_warp`
15. `cmds_motion`
16. `cmds_retime`
17. `cmds_ease`
18. `cmds_hints`
19. `cmds_scene`
20. `cmds_frames`
21. `onion`
22. `multi`
23. `cmds_draw`
24. `cmds_view`
25. `cmds_group`
26. `cmds_library`
27. `merge`
28. `cmds_modify`
29. `cmds_path`
30. `cmds_raw`
31. `cmds_rawsub`
32. `cmds_rawpoints`
33. `editing`
34. `find`
35. `explorer`
36. `cmds_brush`
37. `cmds_filter`
38. `cmds_erase`
39. `cmds_paint`
40. `cmds_swatch`
41. `cmds_data`
42. `cmds_text`
43. `cmds_width`
44. `cmds_stroke`
45. `cmds_shape`
46. `cmds_plugin`
47. `cmds_sound`
48. `cmds_audio_keys`
49. `cmds_video`
50. `cmds_actions`
51. `cmds_script`
52. `cmds_macro`

### 2.2 Registry artifacts

The registry is split only to keep the files reviewable. The segments are a single logical inventory and together contain all 395 enumerated command entries.

| Segment | Registered source modules | Rows |
|---|---:|---:|
| [command-registry-01.csv](command-registry-01.csv) | 9 | 76 |
| [command-registry-02.csv](command-registry-02.csv) | 9 | 74 |
| [command-registry-03.csv](command-registry-03.csv) | 9 | 89 |
| [command-registry-04.csv](command-registry-04.csv) | 9 | 54 |
| [command-registry-05.csv](command-registry-05.csv) | 9 | 64 |
| [command-registry-06.csv](command-registry-06.csv) | 7 | 38 |
| **Total** | **52** | **395** |

The combined registry was fetched back from the FDM branch and independently recounted: 395 rows, 395 unique IDs, zero duplicates, 264 true flags, 131 false flags, and zero unknown flag values.

### 2.3 Initial effect-class distribution

| Initial effect class | Commands |
|---|---:|
| Persistent document/host-state write (`mutates=true`) | 262 |
| Nested plugin command transaction | 1 |
| Nested macro command transaction | 1 |
| Document replacement (`mutates=false`) | 5 |
| History restore (`mutates=false`) | 2 |
| External file side effect (`mutates=false`) | 29 |
| Preferences or plugin state (`mutates=false`) | 17 |
| Session context/tool state (`mutates=false`) | 22 |
| Session clipboard state (`mutates=false`) | 5 |
| Session selection state (`mutates=false`) | 7 |
| Read-only or session query (`mutates=false`) | 44 |
| **Total** | **395** |

The CSV classifications are an initial source-level classification, not proof that each callback's full transitive effect graph has been tested. They identify the required owner/transaction family and provide a canonical list for subsequent callback-level conformance work.

---

## 3. Why `CommandSpec.mutates` cannot remain the transaction authority

The false-flag group includes several materially different behaviors:

| Behavior | Representative command IDs | Required host treatment |
|---|---|---|
| Replace current document | `file.new`, `file.new_sample`, `file.open`, `file.open_recent`, `file.recover` | Document replacement transaction; new session/document identity rules |
| Restore history | `edit.undo`, `edit.redo` | Persistent-bundle restore; advance runtime evaluation epoch |
| Write or delete files | `file.save`, `file.save_as`, exports, `file.publish`, `file.autosave`, `file.discard_recovery` | Staged external-effect protocol; explicit save/recovery semantics |
| Modify desktop integration | `desktop.integrate`, `desktop.remove` | External filesystem/process effect; not document history |
| Modify preferences or plugin state | `prefs.recent_*`, `brush.preset_*`, `filter.preset_*`, `plugin.install/remove/reload` | Separate settings/plugin owner; never silently classify as document no-op |
| Change session/editor context | `timeline.goto`, `scene.select`, `symbol.edit`, `layer.select`, view/tool toggles | Context generation or transient-state revision, not document dirty state by default |
| Change clipboard/selection | `edit.copy`, `timeline.copy_frames`, `frames.copy`, selection commands | Session-transient state; no document history unless settlement changes artwork |
| Query | `doc.info`, `library.items`, `bone.info`, `warp.info`, `view.aids` | Read-only snapshot/query path |

The 131 false-flag command IDs and their initial classes are present in the CSV files. They must not be collapsed into one `NoMutation` branch.

### 3.1 Required replacement for the boolean

Replace the boolean's authority with a typed effect declaration, with at least these categories:

- `DocumentDraftMutation`
- `DocumentReplacement`
- `HistoryRestore`
- `SessionContextMutation`
- `SelectionOrClipboardMutation`
- `PreferenceOrPluginMutation`
- `ExternalFileEffect`
- `ReadOnlyQuery`
- `NestedCommandTransaction`
- `UnknownEffect`

The descriptor is an auditable declaration, not a substitute for runtime effect recording. Every command transaction also records typed effects as it executes. An undeclared or unknown persistent effect must fail closed or conservatively invalidate; it must never be silently treated as read-only.

---

## 4. Enforceable Session ownership contract

### 4.1 Ownership rule

A live `Session` has exactly one mutation owner. All persistent and session-state mutations must execute on that owner. The owner is the GUI event/update loop for the desktop app, or the calling execution thread for a headless/CLI Session.

Current call patterns support this model but do not enforce it. The Session API currently has no owner token or runtime thread assertion.

### 4.2 Recommended enforcement

1. Capture an opaque `SessionOwner` identity when the Session is constructed.
2. Keep it private and preserve it across document replacement; replacing a document must not silently transfer ownership.
3. Check the owner at every mutation entry point: `execute`, gesture begin/update/end/cancel, undo/redo/history travel, document replacement, recovery restore, and transaction commit.
4. Return a deterministic ownership error in release builds; use debug assertions as an additional diagnostic, not the sole protection.
5. Do not make a blanket `!Send`/ `!Sync` change until the complete workspace consumer and embedding inventory has been compiled and reviewed. A runtime owner token is the initial compatible enforcement direction.
6. Worker threads receive immutable document/persistent-state snapshots only. They return immutable results/messages to the Session owner; they never receive `&mut Session`, `&mut Document`, or an identity-ledger mutation capability.

### 4.3 Observed dispatch paths

| Path | Pinned source evidence | Contract |
|---|---|---|
| GUI | `src/app.rs` owns the live Session and runs command execution from the application update path | UI mutations remain on the app owner |
| GUI remote/MCP | `src/remote.rs` routes requests through the app's polling/dispatch path; the remote test client is a separate thread | Remote requests enqueue; app owner executes |
| CLI/control | `crates/cli/src/lib.rs` handlers take `&mut Session`; the JSON-lines serve loop processes requests sequentially | The CLI caller owns the Session for the serve loop |
| Plugin host | `crates/engine/src/cmds_plugin.rs` routes nested plugin requests through `SessionHost` and `execute_nested` | Nested calls share the outer command transaction |
| Autosave | `src/autosave.rs` sends a cloned Document snapshot to the worker | Worker receives immutable snapshot, never Session |
| Render | `crates/engine/src/render_service.rs` works from cloned document/display inputs | Worker returns render output; no persistent mutation authority |

These are observed usage patterns. They do not establish that the public API currently prevents a caller from mutating Session on another thread.

---

## 5. Mutation and transaction boundary contract

### 5.1 Persistent bundle

The atomic persistent unit is:

- artwork/document state;
- persistent `DocumentId`;
- target-incarnation ledger;
- versioned host extension envelope;
- FDM authored payload and supported bake provenance.

The same unit is used by history snapshots, save/load, autosave, recovery, dirty-state calculation, and document replacement. Runtime session identity, evaluation epoch, active jobs, render caches, selection and transient clipboard state are not serialized into the persistent bundle unless separately specified.

### 5.2 Command transaction protocol

The owner-thread transaction protocol is:

1. Validate owner identity and command effect declaration.
2. Create a private draft of the persistent bundle and the required transient/context draft.
3. Execute the command callback using a scoped mutation capability; no public mutable Session or callable external `RunFn`.
4. Execute all command-owned settlement before commit: cut healing, floating-paste settlement, rig tracking/pose settlement, motion auto-key completion, group-frame settlement, and any other callback-deferred write.
5. Collect target transitions and typed dependency effects.
6. Validate artwork, identity ledger, extension envelope, remaps, and checked counters.
7. Detect a true no-op. A no-op publishes no document revision, StateId, or evaluation epoch change.
8. Prepare history and dirty-state updates before publication; no fallible persistent work remains after the publication point.
9. Atomically publish the complete persistent bundle, identity ledger, StateId/history state, and monotonic runtime evaluation epoch on the owner.
10. Publish one authoritative transaction receipt. Event delivery failure cannot erase the committed epoch; consumers can always read the current epoch.

Failed transactions publish none of the draft, identity transitions, history, dirty-state changes, or epoch increments.

### 5.3 Settlement ordering is part of the transaction

The current `Session::execute` path settles cut/floating/rig/motion/group state around the callback. Some settlement currently occurs after the main document equality/history point. The non-mutating branch also invokes settlement helpers.

Required change:

- No persistent settlement after the transaction receipt.
- No `mutates=false` bypass around settlement.
- Every command goes through the same outer transaction orchestrator; its typed effect declaration decides which draft capabilities it receives.
- If a nominally context-only command causes persistent settlement, the runtime journal upgrades the transaction to a document mutation before publication.
- Gesture updates are individually committed visible states (and advance the evaluation epoch) even if their history entries are coalesced. Gesture cancellation restores the persistent snapshot and advances runtime freshness.

### 5.4 Nested commands and macros

- `plugin.run` and `macro.run` share the outer transaction draft, effect journal, and one history boundary.
- Nested commands do not independently publish, increment epochs, or create history entries.
- A nested failure aborts the outer transaction unless an explicitly designed recoverable operation is added.
- Plugin-facing APIs expose constrained command requests and immutable read snapshots, not mutable Document access or the transaction's private identity ledger.

### 5.5 External file effects

Filesystem writes, exports, desktop integration, recovery deletion, and preference/plugin file operations cannot be rolled back by restoring an in-memory Document snapshot.

They require explicit staged protocols:

- validate parameters, destination and overwrite policy before persistent changes;
- prepare output in a temporary/isolated location where possible;
- perform the atomic external commit at a defined boundary;
- update path/saved-state metadata only after successful file replacement;
- report partial external failures explicitly and make retries/idempotency rules clear;
- never mark a document saved when the corresponding persistent bundle was not successfully written.

File I/O is not a reason to bypass the Session owner; it is a separate typed effect coordinated by the owner.

---

## 6. Non-command mutation entry-point inventory

The command registry is not sufficient by itself. These paths are also part of the mutation boundary and must be included in the machine-checkable coverage manifest.

| Entry point | Current exposure/path | Required treatment |
|---|---|---|
| `Session.doc` | Public mutable field | Make private; expose immutable views/snapshots |
| Other public Session fields | Scene, editing, frame, layer, selection, state, revision, path and editor settings | Split persistent, context, transient and runtime state; controlled methods |
| `RunFn` / `CommandSpec.run` | Public callback accepting `&mut Session` | Make callback private/internal; public descriptors contain metadata only |
| Document public fields | Public persistent tree, including scenes/layers/library/assets and allocator | No mutable authoritative Session access may escape |
| `Document::alloc_id` | Public saturating allocator | Host-owned checked allocator; exhaustion aborts before publish |
| `Document::timeline_mut`, `Timeline::layer_mut`, `Layer::keyframe_mut`, `Keyframe::elements_mut_where` | Public nested mutable APIs | Restrict authoritative use to scoped host transaction capability |
| `editing::edited_mut`, `Session::layer_mut` | Internal helper routes used by command modules | Require an active transaction draft; no free-standing mutation |
| Cut/floating settlement | `settle_cut`, `heal_cut`, `settle_floating` | Execute before transaction validation/publication |
| Rig/motion settlement | Rig tracking and motion auto-key completion | Same transaction; explicit typed dependencies |
| Gestures | Begin/update/end/revert | Owner-checked transaction updates; coalesced history but monotonic epoch |
| History | Undo, redo, travel-to, snapshot restore | Restore entire persistent bundle; runtime epoch advances |
| Document replacement | Open/new/recovery/replace-document | Atomic replacement and fresh runtime SessionInstanceId; persistent identity semantics explicit |
| Transfer/import | Clipboard, file import, scene duplication, Library transfer | Explicit source-to-destination identity/remap table |
| Autosave/recovery | Cloned Document snapshot and separate recovery format | Persist the complete bundle with a matching bundle revision |
| Render/evaluation | Display-list and render worker snapshots | Immutable input only; acceptance is owner-side |
| External automation/plugin | Public command callbacks and nested plugin execution | Same owner and transaction; no mutation bypass |

### 6.1 Identity transition mapping remains required

The registry identifies commands, but the implementation must still provide explicit target identity transitions for:

- create and delete;
- duplicate and paste;
- same-document move versus cross-document import;
- scene duplication;
- group and ungroup;
- shape/text/symbol/bitmap conversion;
- raw geometry merge/split;
- Library symbol duplication/deletion;
- bone/armature and native Warp edits;
- undo/redo and failed transaction rollback.

No transition may be inferred from a reused numeric ID, name, list order, selection, or geometry similarity.

---

## 7. Persistent state and serialization reconciliation

The current pinned host source still has these gaps:

1. `History::Snapshot` stores `Document`, StateId and editor context, but not a DocumentId, target-incarnation ledger, or extension envelope.
2. The private `DocumentFile` decoder enumerates known fields and has no flattened unknown-extension map. Unknown fields are not retained as an opaque payload by the current path.
3. `format::load/save` serializes Document only. Its temp-file-and-rename replacement strategy should be preserved while widening the serialized persistent unit.
4. `Session::is_dirty` is StateId-based and does not represent a future envelope/identity ledger unless those fields join the same persistent history unit.
5. Autosave/recovery stores Document snapshots and tracks StateId/revision; it must move to the persistent bundle.
6. Save As and Save Copy must be explicit: Save As retains DocumentId; Save Copy creates a new DocumentId and fresh target identities with internal references remapped.
7. Unknown extension versions must be preserved losslessly or the host must refuse a destructive save. A lossy round trip is not acceptable.

---

## 8. S0 gate matrix

| Gate | Result | Evidence / remaining work |
|---|---|---|
| S0-A: exact host source baseline | <badge color="success">Closed</badge> | Main SHA, tree SHA and complete tree verified |
| S0-B1: all registered spec sources | <badge color="success">Closed</badge> | Exact 52-source registration list |
| S0-B2: command IDs and declared flags | <badge color="success">Closed</badge> | 395 rows; unique IDs; 264 true / 131 false; zero parse gaps |
| S0-B3: callback-level transitive effects and test ownership | <badge color="warning">Partial</badge> | Initial effect classes exist; per-callback write graph and conformance test IDs still need implementation-level trace |
| S0-C: owner and dispatch contract | <badge color="success">Design closed</badge> | Single-owner contract and enforcement recommendation specified; runtime enforcement not implemented |
| S0-D: exact host CI baseline | <badge color="warning">Unavailable</badge> | Pinned commit has 0 Actions runs and 0 check runs reported; do not claim a pass |
| S0-E: complete workspace public-API consumer inventory | <badge color="warning">Partial</badge> | Tree contains 413 Rust files; key app/CLI/MCP/autosave/plugin/render consumers sampled. GitHub code-search endpoint returned no results with `incomplete_results=true`; this is not evidence of no consumers. Full source/compile inventory remains required |
| S0-F: non-command mutation coverage | <badge color="warning">Partial</badge> | Major Session/Document/history/gesture/settlement/persistence paths identified; complete compile-enforced coverage remains open |

**Disposition:** S0 registry source enumeration is closed. S0 owner contract is design-closed. The overall S0 gate remains blocked by S0-B3, S0-D, S0-E, and S0-F. Do not begin S1 runtime implementation on the basis of this document alone.

---

## 9. CI and verification evidence

The pinned host workflow file is `/.github/workflows/ci.yml` (blob `0a4733afb72f12bfd6a2b0eb81967a37b954c104`). It configures:

```sh
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
cargo deny --locked check licenses bans sources
```

For exact AniCraft commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`, the GitHub Actions API returned:

- workflow runs: **0**
- check runs: **0**

No local cargo commands or host tests were executed by this read-only milestone. This is an unavailable baseline, not a passing baseline and not a known failing baseline.

Before runtime implementation, run the configured checks on a controlled host branch and record toolchain, platform, exact SHA, all failures, and the run URL. Do not hide pre-existing failures by changing the CI gate as part of the FDM transaction work.

---

## 10. Implementation order after S0

The ownership contract translates into this implementation sequence, subject to a separate authorization:

1. **S1 — Public API containment:** remove mutable Session/document escape hatches and public callback invocation; migrate internal consumers to read views.
2. **S2 — Owner enforcement:** add owner identity and checks to all mutation entry points; route GUI, remote, CLI and plugin requests to owner.
3. **S3 — Transaction draft/effect journal:** typed command effects, private mutation capability, rollback and no-op semantics.
4. **S4 — Settlement integration:** move cut/floating/rig/motion/group settlement inside the transaction before validation.
5. **S5 — Identity ledger:** checked allocation, incarnation transitions and operation-specific remap maps.
6. **S6 — Persistent extension envelope:** DocumentId, target ledger and versioned FDM payload in the actual file format.
7. **S7 — History/dirty/save/recovery:** complete bundle in snapshots, save/load, autosave and recovery; Save As/Copy distinction.
8. **S8 — Runtime freshness:** checked evaluation epoch and context generation; undo/redo/replacement freshness.
9. **S9 — Immutable snapshot and synchronous FDM evaluation:** owner-side snapshot and result acceptance. Async remains separately reviewed and is not authorized here.

The registry CSVs become the source list for S1–S4 compile/test migration. Each command ID must receive a callback-level effect owner, identity/dependency effects, rollback/history expectations, and conformance tests before its implementation is considered covered.

---

## 11. Scope and decisions preserved

- This is documentation and registry data only.
- No AniCraft or FDM runtime code changed.
- No test, cargo, benchmark or CI pass is claimed.
- No PR was merged.
- No S1–S9 runtime implementation is authorized.
- Native Asset Warp remains host-owned and independent of FDM.
- Smart Controller and Form Deform evaluation remain independent under CCC-01.5.
- No asynchronous evaluation, multi-deformer composition, or expanded bitmap/text/symbol/group/nested deformation scope is introduced.

---

## 12. Recommended next milestone

**FDM-06.12 — Workspace Consumer Migration and Host Baseline Verification.**

It should close the remaining S0 blockers without changing runtime behavior:

1. Obtain a complete workspace source/consumer inventory for `Session.doc`, public Session fields, `CommandSpec.run`, `RunFn`, and public mutable Document methods.
2. Produce a compile-backed consumer migration map for GUI, CLI, MCP, plugins, render, autosave, tests, and any downstream crates.
3. Audit all 131 false-flag callbacks and the transitive settlement helpers for exact effects; identify every callback that replaces a document, mutates context, writes preferences, or performs file I/O.
4. Assign each of the 395 command IDs an explicit effect owner and conformance-test ID.
5. Record a real CI baseline for a controlled host branch, or explicitly agree on a documented no-baseline fallback.
6. Close S0 with a final evidence matrix. Only then request separate authorization for S1.

**Implementation gate:** FDM-06.11 closes the command registry inventory and owner contract design, not the full S0 implementation gate. No runtime work is authorized.
