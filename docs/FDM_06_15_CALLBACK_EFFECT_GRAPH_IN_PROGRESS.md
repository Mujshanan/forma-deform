# FDM-06.15 — Callback effect graph: first evidence pass

**Disposition: IN PROGRESS. S0 remains OPEN.** Source baseline: AniCraft commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`; tree `80071fb3f714944e49608668871d4cf53a6159a5`. No source files were changed.

## 1. Scope completed in this pass

- Carried forward all **131** commands declared `mutates=false`.
- Reconciled callback source mapping: **95 named callback bodies** are locatable by the prior lexical scan; **36** are inline closures, literal `CommandSpec` callbacks or otherwise require targeted manual extraction.
- Added a one-row-per-command callback evidence index and source-backed direct-effect findings in the FDM-06.15 evidence bundle.
- Inspected the central execution and settlement paths in `crates/engine/src/lib.rs`.

This is a first effect-graph pass, **not a completed transitive graph**. No command is marked transitively closed.

## 2. Confirmed mutation-flag discrepancies

The following are direct, source-supported examples where `mutates=false` does not mean “does not mutate document/session/application state”:

| Command | Direct effect | Disposition |
|---|---|---|
| `file.new` | Replaces active document via `Session::replace_document` | FAIL |
| `file.new_sample` | Replaces active document via `Session::replace_document` | FAIL |
| `file.open` | Replaces active document and adds to recent list | FAIL |
| `file.new_from_template` | Replaces active document | FAIL |
| `edit.undo` / `edit.redo` | Restores history snapshots and increments revision | FAIL |
| `prefs.recent_pin` / `prefs.recent_remove` / `prefs.recent_clear` | Mutates recent-file state | FAIL |
| `file.delete_template` | Deletes template and thumbnail files | FAIL |
| `file.save_as_template`, `file.save`, `file.save_as`, export commands, `file.publish` | Write external files; some also update session metadata | REVIEW |
| brush/filter preset save/rename/delete; macro rename/delete; plugin and desktop integration commands | Mutate persistent app/OS state or files | REVIEW |

`FAIL` here means the declared flag is inconsistent with the observed effect if the flag is intended to represent all state mutation. `REVIEW` means the effect is proven or strongly indicated, but the correct classification depends on a clarified effect taxonomy (document vs session vs persistent app state vs external I/O).

## 3. Central transaction/settlement effect graph

`Session::execute` is decisive: at `crates/engine/src/lib.rs:766-772`, the `mutates=false` branch calls `settle_cut` before the callback, runs the callback, then calls `settle_cut`, `hold_group_frame`, and `settle_floating` before returning.

Those helpers are not all observational:

- `settle_cut` may call `heal_cut` (`lib.rs:930-939`).
- `heal_cut` restores `self.doc = p.before.doc`, restores state/selection and increments revision (`lib.rs:944-964`).
- `hold_group_frame` can change the playhead and repair selection (`lib.rs:894-898`).
- `settle_floating` may merge floating shapes into the document, assign `self.doc`, refresh state/revision and repair selection (`lib.rs:848-880`).

Thus even a read-only callback can cause document/session mutation through the outer execution path. The mutation flag is not a reliable guard for the full effect of `Session::execute`.

`Session::execute_nested` differs: for `!spec.mutates`, it directly invokes `(spec.run)(self, params)` and skips the outer settlement (`lib.rs:970-979`). This creates a second semantic boundary that must be included in the graph. A false-flag callback that mutates document state can therefore be invoked without the ordinary mutating-command snapshot/rollback path.

## 4. High-priority callback traces still required

1. All 36 inline/literal callbacks, especially `edit.undo`, `edit.redo`, `file.export_apng`, `file.export_webp`, `file.export_audio`, `file.export_formats`, `desktop.integrate`, `find.search`, `find.goto`, `layer.properties`, `modify.envelope`, `modify.transform_origin`, `paint.sample`, `raw.subselect`, `raw.subselect_rect`, `scene.next`, `scene.prev`, `sound.analyse`, `symbol.edit`, timeline navigation and view toggles.
2. All 95 named callbacks, including local helper chains and cross-module/cross-crate callees.
3. Nested plugin/macro command dispatch, command recursion and trait/function-pointer dispatch.
4. File-system and OS integration sinks, persistent preset/recent/plugin state, and background/async work.
5. All paths through `execute`, `execute_nested`, gestures, provisional cuts and settlement.

## 5. Exit criteria and current gate

- 131 false-flag callback entries indexed: **PASS (inventory only)**.
- Source-backed direct mutation discrepancies found: **FAIL / OPEN**.
- Central transaction/settlement path identified: **FAIL / OPEN**.
- Full transitive callback graph for 131 false-flag commands: **OPEN**.
- Full graph for remaining 264 commands: **NOT STARTED**.
- Compiler-backed verification: **OPEN**.
- Host CI: **OPEN**.
- S0 closure: **NOT CLOSED**.

## 6. Safety / authorization

This pass changed no AniCraft Rust source, manifests, workflows or runtime behavior. No runtime implementation, FDM evaluator, host integration, adapter bypass, async evaluation, Asset Warp change, Smart Controller/FDM composition, merge or release was performed or authorized.
