# FDM-06.16 — Uploaded Archive Reproduction and Execution-Path Audit

**Disposition: archive verified; additional execution-path findings confirmed; S0 remains OPEN.**  
**Host:** Mujshanan/anicraft  
**Pinned commit:** \`2df7775907d1458ce653e7e6314ec8a5d7d565f5\`  
**Tree:** \`80071fb3f714944e49608668871d4cf53a6159a5\`  
**Parent evidence:** FDM-06.13–06.15, PR #80  
**Change policy:** Read-only source audit. No AniCraft or FDM runtime files changed. No host tests or CI pass claimed. No implementation or merge authorization.

## 1. Uploaded archive verification

The user-provided \`anicraft.zip\` was extracted and verified as a Git working tree.

| Check | Observed result |
|---|---|
| Archive size | 25,593,223 bytes |
| Archive SHA-256 | \`745cc5dc8e4b797db4590919f02185821af4de3d0264cecdfe2914768dab1b40\` |
| Archive root | \`anicraft/\` |
| Embedded Git metadata | Present; \`.git/HEAD\` contains the pinned commit |
| Git origin | \`https://github.com/Mujshanan/anicraft.git\` |
| \`git rev-parse HEAD\` | \`2df7775907d1458ce653e7e6314ec8a5d7d565f5\` |
| \`git rev-parse HEAD^{tree}\` | \`80071fb3f714944e49608668871d4cf53a6159a5\` |
| \`git status --short\` | Empty; clean working tree |
| Tracked files | 911 |
| Tracked Rust files | 413 |
| Reachable commits across archived refs | 1,308 |
| \`git fsck --full --no-reflogs\` | Exit 0; no diagnostics |

**Result: PASS — the uploaded archive is the exact pinned source commit and tree.** This is source-integrity evidence, not a build or test result.

The archive is detached at the pinned commit. The extracted checkout was not edited. The GitHub host PR #1 also reports the same pinned commit as its base.

## 2. Additional source-level execution-path findings

The following were re-read directly from the verified archive. Line numbers refer to the pinned commit.

### 2.1 The outer non-mutating command path is not transactionally read-only

In \`crates/engine/src/lib.rs:753-772\`, \`Session::execute\` does this for \`spec.mutates == false\`:

1. Calls \`settle_cut\` before invoking the callback.
2. Calls the public command callback directly.
3. Calls \`settle_cut\` again.
4. Calls \`hold_group_frame\`.
5. Calls \`settle_floating\`.
6. Returns the callback result.

This branch takes no pre-command snapshot, has no callback-error rollback, and does not create a history record in this branch.

The called helpers are stateful:

- \`settle_cut\` at lines 930-940 can call \`heal_cut\`.
- \`heal_cut\` at lines 944-964 restores \`self.doc\`, state and selection, and increments revision.
- \`hold_group_frame\` at lines 894-898 can change the playhead and selection.
- \`settle_floating\` at lines 848-880 can merge floating shapes into \`self.doc\`; when the document changes it assigns a fresh state and increments revision.

The callback result is stored before the post-callback settlement. Therefore settlement is still attempted when the callback returns an error. If floating artwork is eligible to settle, the document can change even though the command result is an error. The early-return branch does not record that settlement as a new undo step.

**Finding:** \`mutates=false\` is not a safe read-only/effect-free branch. The full effect of a command includes pre- and post-callback settlement, including error paths.

### 2.2 Nested non-mutating execution has different semantics

In \`Session::execute_nested\`, lines 970-979, the non-mutating branch immediately calls \`(spec.run)(self, params)\`.

It skips the normal outer path's:

- group check;
- snapshot and rollback;
- cut settlement;
- group-frame settlement;
- floating-artwork settlement.

The plugin host calls this route from \`cmds_plugin.rs:84-99\` after applying \`plugin_may_call\`. Saved macros call it from \`cmds_macro.rs:252-271\` after applying \`macro_may_call\`. These allowlists restrict classes of commands, but they do not make all allowed false-flag callbacks read-only: timeline, scene, layer selection, and other document-related namespaces are allowed.

**Finding:** the same command callback can execute under materially different transaction and settlement semantics depending on whether it is called through \`Session::execute\` or \`execute_nested\`. The effect inventory must model the caller path, not just the callback ID.

### 2.3 Public callback and live-document mutation capabilities remain exposed

The pinned declarations confirm:

- \`RunFn = fn(&mut Session, &Value) -> Result<Value>\` is public at \`crates/engine/src/lib.rs:129-130\`.
- \`CommandSpec.run\` is a public function-pointer field at lines 132-146.
- \`Session.doc\` is public at lines 391-394.
- Session context and state fields including \`scene\`, \`editing\`, \`frame\`, \`layer\`, \`selection\`, \`revision\` and \`state\` are public in the Session declaration.
- \`Session::replace_document\` is public at lines 696-720.
- Gesture entry points \`begin_gesture\` and \`end_gesture\` are public at lines 648-679; \`revert_gesture\` is public at lines 1017-1033.
- \`undo\` and \`redo\` are public at lines 1065-1089.
- \`Document\` exposes public persistent fields at \`crates/doc/src/lib.rs:1969-2000\` and public mutable helpers including \`alloc_id\` and \`timeline_mut\` at lines 2101-2130. Nested public mutators include \`Keyframe::elements_mut_where\` (1314), \`Layer::keyframe_mut\` (1582), and \`Timeline::layer_mut\` (1659).

This confirms that the current owner-thread model is a calling convention rather than an enforced capability boundary. The source-level consumer map in FDM-06.14 is useful migration evidence, but it does not prove that all these capabilities can be removed without compile errors.

### 2.4 Publisher callback invocation is isolated, but still a separate call path

\`Publisher::new\` at \`crates/engine/src/publish.rs:424-439\` constructs a separate \`Session\` from a Document snapshot. Its \`run\` method at lines 461-488 directly invokes \`(spec.run)(&mut self.s, ...)\` after setting the publish scene/editing/frame context.

This is not a mutation of the editor's live Session: the publisher owns its own Session. It is nevertheless another production callback invocation path that must be considered when encapsulating \`RunFn\` or adding owner checks. It should be explicitly classified as an isolated snapshot/export execution context, not silently routed through the live editor transaction.

### 2.5 The mutating outer path also needs external-effect separation

The \`mutates=true\` branch of \`Session::execute\` snapshots and can restore the Document on callback error. That rollback cannot undo filesystem, preference, plugin-registry, desktop-integration or other OS effects already performed by a callback. Those effects require a separate staging/commit/compensation policy; a Document snapshot is not a complete transaction over all state.

## 3. Consumer audit reconciliation

FDM-06.14's targeted lexical scan remains the current quantitative consumer evidence:

| API surface | Candidate lines |
|---|---:|
| \`Session.doc\` access | 382 |
| \`CommandSpec\` / \`RunFn\` references | 9 |
| \`.execute(...)\` calls | 68 |
| Gesture begin/end calls | 98 |
| \`replace_document(...)\` calls | 1 |
| Undo/redo calls | 10 |
| Public nested Document mutator calls | 208 |
| **Total** | **762 across 94 files** |

These are candidate source lines, not unique semantic call sites. The scan includes test code and does not prove borrow mutability, call graph closure, production/test classification, or compiler impact.

The earlier FDM-06.12 claim that \`src/gradient_tool.rs\` contained a production direct Document write was corrected by FDM-06.13: the cited direct writes are in a test module. Do not carry that claim forward as a production bypass.

**Consumer disposition:** source-indexed and materially improved; not compiler-backed or complete enough to close S0.

## 4. Host CI and local execution evidence

The pinned workflow definitions establish these required checks:

- \`.github/workflows/ci.yml\`: \`cargo test --workspace\`, \`cargo clippy --workspace --all-targets -- -D warnings\`, and \`cargo deny --locked check licenses bans sources\`.
- \`.github/workflows/platforms.yml\`: macOS and Windows workspace build, test and Clippy.

At this audit, GitHub reports:

| Commit | Workflow runs | Commit status contexts |
|---|---:|---:|
| Pinned host baseline \`2df7775907d1458ce653e7e6314ec8a5d7d565f5\` | 0 | 0 |
| Current host PR #1 head \`c1c034df9f0308174b4708fafe00d05259226bca\` | 0 | 0 |

The local analysis environment has no \`cargo\` or \`rustc\` executable available, so no host Cargo command was run. The GitHub connector exposes no workflow-dispatch action. No Linux, macOS or Windows pass is claimed.

The user can manually run the existing \`CI\` and \`CI (macOS, Windows)\` workflows using the \`workflow_dispatch\` trigger on branch \`audit/fdm-06-12-host-baseline\`; the workflow definitions are already present in the pinned source. The result must be captured against that branch head and the pinned source base.

## 5. Updated S0 gate decision

| Gate | Result |
|---|---|
| Uploaded archive matches exact commit and tree | **PASS** |
| Git object database integrity | **PASS** |
| 52 command-spec providers / 395 IDs | PASS at registry-enumeration level |
| 131 false-flag callback locations and first-order triage | PASS at location/triage level only |
| Full transitive graph for all 131 false-flag callbacks | OPEN |
| Full callback graph for the other 264 commands | NOT COMPLETE |
| Outer and nested transaction-path equivalence | **FAIL / OPEN** |
| Non-command mutation-route inventory | PARTIAL |
| Consumer classification and compiler-backed proof | OPEN |
| Host tests, Clippy and cargo-deny | OPEN |
| macOS and Windows CI | OPEN |
| FDM-06.6 implementation and verification | NOT DONE |
| S0 closure | **NOT CLOSED** |

## 6. Required next work

1. Complete the callback-to-helper effect graph for all 131 false-flag callbacks, including both outer and nested invocation paths.
2. Complete the remaining 264 callback graphs and identify every filesystem, preference, plugin, clipboard, context and document effect.
3. Turn the 762 candidate consumer lines into a typed inventory: production/test, read/write, owner, transaction route, and replacement API; then obtain compiler-backed proof.
4. Complete non-command routes: direct Session/Document mutation APIs, gesture lifecycle, provisional cut/floating settlement, import/transfer, history, recovery, document replacement, autosave, render and publisher snapshots.
5. Obtain actual host Linux and macOS/Windows CI results, or record a separately approved baseline alternative.
6. Conduct a formal S0 closure review only after all gates have reproducible evidence.

**Hard gate preserved:** S0 must formally close; FDM-06.6 must then be implemented and verified; and the user must explicitly authorize the implementation stage. This document grants none of those permissions. No FDM runtime implementation, AniCraft production integration, evaluator wiring, adapter bypass, asynchronous evaluation, merge or release was performed or authorized.


### 2.6 Evaluation context changes are not equivalent to document revision

The Session declaration exposes `scene`, `editing`, `frame`, `layer` and `selection` as public fields. `Session::set_frame` at `crates/engine/src/lib.rs:1188-1192` changes the playhead and repairs selection without incrementing `revision`. Scene navigation and symbol/group editing commands also update context fields through their callbacks.

The display-list cache already keys on `state`, `scene`, `frame` and the open-edit context (see `display_list` at lines 1119-1133), but that does not provide a monotonic context generation for an independent FDM snapshot/result-acceptance boundary.

**Finding:** an FDM freshness stamp must explicitly include frame, scene, edit-context and relevant selection/dependency state, or use a separately advanced context generation. Document revision alone is insufficient. This is source-level confirmation only; no runtime generation mechanism exists in the pinned host.
