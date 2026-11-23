# FDM-06.15 — Callback effect graph, second source-tracing pass

**Disposition: PARTIAL / IN PROGRESS. S0 remains OPEN.**  
**Pinned AniCraft source:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Pinned tree:** `80071fb3f714944e49608668871d4cf53a6159a5`

This addendum records additional source-backed callback traces. It does not claim that all 36 inline/literal callback gaps or all 95 named callback graphs are closed. No AniCraft source, runtime behavior, manifests, workflows, or dependencies were changed.

## 1. Important semantic correction: what `CommandSpec::mutates` means

At `crates/engine/src/lib.rs`, the field documentation states that `mutates` means **whether the command changes the document and therefore records an undo step**. It is not a universal flag for every session field, preference, external file, or OS side effect.

Therefore the audit must classify effects separately:

- **D — document/history mutation:** document replacement/edit, undo/redo, revision/state changes.
- **S — session/editor state:** scene, frame, selection, style, aids, path, recent list, published set, etc.
- **P — persistent application state:** presets, preferences, plugin registry/configuration.
- **F — filesystem / OS effect:** read, write, delete, launcher/MIME integration.
- **R — read-only computation/query.**
- **C — conditional framework effect:** effects introduced by command dispatch/settlement depending on pending session state.

A command that writes a file or changes a view aid is not automatically a violation of the documented document-mutation flag. A command that replaces the document or restores history while marked false is a direct semantic discrepancy and requires explicit resolution.

## 2. Dispatcher and transaction edges — confirmed

Source: `crates/engine/src/lib.rs`, `Session::execute`, `settle_floating`, `hold_group_frame`, `settle_cut`, `heal_cut`, and `execute_nested` (approximately lines 766–979 at the pinned commit).

```text
Session::execute(id, params)
  ├─ access::check_params
  ├─ confirmation
  ├─ group_allows
  ├─ settle_cut [may call heal_cut]
  ├─ if mutates == false:
  │    ├─ callback
  │    ├─ settle_cut [may call heal_cut]
  │    ├─ hold_group_frame [may change frame/selection]
  │    └─ settle_floating [may merge floating artwork into Document]
  └─ if mutates == true:
       ├─ snapshot / rollback boundary
       ├─ callback
       ├─ rig/motion settlement
       ├─ document comparison and history recording
       └─ selection/group/floating settlement

Session::execute_nested(id, params)
  ├─ lookup + confirmation
  ├─ if mutates == false: callback directly (no nested snapshot/settlement)
  └─ if mutates == true: nested snapshot + callback + rig settlement
```

Confirmed conditional side-effect sinks:

- `settle_cut → heal_cut`: can restore `self.doc`, state, selection and saved-state bookkeeping, clear recorded commands, invalidate display cache, increment revision.
- `hold_group_frame`: can reset frame to zero and repair selection.
- `settle_floating → merge::settle`: can merge floating raw shapes into the document, change document state/revision, invalidate cache and repair selection.
- The false branch of `execute_nested` calls the callback directly. It does not get the outer `execute` settlement path or the nested mutating-command snapshot/rollback path.

These are **conditional framework edges**, not proof that every invocation mutates. The graph must carry the preconditions (live provisional cut, floating selection state, editing-group frame, nested dispatch context).

## 3. Newly source-traced callback families

| Command(s) | Callback / path | Actual effect class | Source-backed result |
|---|---|---|---|
| `file.new`, `file.new_sample` | `cmds_file::new_doc/new_sample → Session::replace_document` | D + S | Direct document replacement while `mutates=false`; discrepancy |
| `file.open` | `cmds_file::open → format::load / XFL import → replace_document`; may add recent entry | D + S + F(read) | Direct document replacement; discrepancy |
| `file.new_from_template` | `cmds_template::new_from_template → format::load or built-in builder → replace_document` | D + S + F(read, path form) | Direct document replacement; discrepancy |
| `edit.undo`, `edit.redo` | inline closures → `Session::undo/redo → end_gesture, heal_cut, history.undo/redo, restore, revision++, fix_selection` | D + S | Direct history/document restoration while `mutates=false`; discrepancy |
| `file.save`, `file.save_as` | `cmds_file::save → format::save; set path; mark_saved; recent.add` | F(write) + S | No document edit shown in callback; mutates session path/saved/recent state |
| `file.export_apng`, `file.export_webp` | inline closures → `anim::export` | F(write), render/read | Export callback; document is passed as read input, output is external |
| `file.export_audio` | `cmds_sound::export_audio` | F(write), render/audio read | Export callback; further internal helper chain remains to be traced |
| `file.export_plugin` | `plugin_export::export_cmd → plan → run → Plugin::export(FrameHost) → vfs::write` | F(write) + plugin execution | Exporter receives document through a read-only frame host; output is written to path |
| `file.export_formats` | inline closure → `formats(s) → exporters(s) → plugin manifests/descriptors` | R | Reads installed exporter metadata; no write sink found in this path |
| `file.open_recent` | `open_recent → exists → cmds_file::open` | D + S + F(read) | Delegates to document-opening path |
| `file.recent_list` | `list → recent.entries + filesystem existence checks` | R | Query only in callback |
| `prefs.recent_pin/remove/clear` | `pin/remove/clear → Session.recent` | S (persistent only if caller later saves preferences) | Mutates in-memory recent-file state; not a document edit |
| `file.templates` | `cmds_template::list → built-in builders + user_templates` | R + F(read/list) | Query/list path |
| `file.save_as_template` | `save_as_template → create_dir_all, format::save, picture_png, vfs::write` | F(write) | Writes template and thumbnail; open document unchanged |
| `file.delete_template` | `delete_template → vfs::remove_file (template + thumbnail), optional remove_dir` | F(delete) | Irreversible filesystem effect |
| `desktop.integrate` / desktop remove | `integrate/remove → install/uninstall → launcher/icons/file-type files; refresh` | F(write/delete) + OS refresh | External OS integration side effects |
| `find.search` | `search_cmd → Query::from_params → search(&s.doc, ...) → hit JSON` | R | Reads document; no mutation sink in callback body |
| `find.goto` | `goto_cmd → exit_all/open_scene/edit/walk; writes scene/layer/frame/selection/revision/cache` | S | Navigation/editor-state mutation; not a document edit in this callback |
| `explorer.tree` | `tree_cmd → tree(&s.doc, ...) → row JSON` | R | Read-only document traversal in callback |
| `sound.analyse` | `analyse → source → levels/visemes` | R | Computes analysis output from sound/document data; no document write in callback |
| `layer.properties` | inline closure → `layer_id → properties(&Session, LayerId)` | R | Reads selected layer properties |
| `scene.select/prev/next` | `select_scene/step_scene` and closures | S | Scene navigation; not a document edit |
| `timeline.goto/next_frame/prev_frame/first_frame/last_frame` | `goto/step/Session::set_frame` and closures | S | Playhead/frame and selection maintenance |
| `view.rulers/grid_show/grid_snap/guides_show/guides_lock/guides_snap/snap_pixels` | inline closures → `cmds_view::toggle → Session.aids` | S | Editor view/snap state mutation; not document mutation |
| `view.aids` | inline closure → `aids_json` | R | Query only |
| `modify.transform_origin` | `transform_origin` | S | Changes tool style state (`Session.style`), not document |
| `paint.pick` | `pick_style` | S | Reads display list and changes fill/stroke tool style |
| `paint.lock_fill` | `lock_fill` | S | Changes tool style setting |
| `raw.subselect`, `raw.subselect_rect` | `subselect/subselect_rect` | S; possible C | Selection/subselection state; must also trace provisional-cut/settlement preconditions |
| `raw.points` | `points_cmd` | R | Anchor/handle query from current document/selection |

The table is a source-trace index, not yet a complete transitive proof for every helper listed. In particular, `cmds_sound::export_audio`, `cmds_desktop::install/uninstall/refresh`, `cmds_view::toggle`, `cmds_modify::transform_origin`, `cmds_paint::pick_style/lock_fill`, raw-subselect helpers, and the remaining 95 named callback bodies still require callee-by-callee closure and cross-module sink review.

## 4. Coverage status

| Work item | Status |
|---|---|
| Pin source commit/tree | PASS |
| Confirm documented meaning of `mutates` | PASS |
| Trace dispatcher and conditional settlement edges | PASS for inspected methods |
| Trace direct document-replacement/history false-flag callbacks | PASS for listed commands |
| Trace representative read-only, navigation, view/tool-state and external-I/O callbacks | PARTIAL |
| Trace every one of the 36 inline/literal callback gaps | OPEN |
| Complete all transitive edges for the 95 named callbacks | OPEN |
| Reconcile every one of 131 false-flag commands under D/S/P/F/R/C taxonomy | OPEN |
| Trace all 264 `mutates=true` command callbacks | NOT STARTED |
| Compiler-backed consumer verification | OPEN |
| Successful host CI | OPEN |
| S0 closure | NOT CLOSED |

## 5. Required continuation

1. Recover and reconcile the exact 36-row callback gap manifest from FDM-06.14 so no entry is lost or silently substituted.
2. For each inline callback, record the closure body, called helper(s), all transitive callees, state fields read/written, document mutation, external sinks, error/rollback behavior and conditional preconditions.
3. For each of the 95 named callbacks, follow the call graph through helpers, function pointers, plugin/macro dispatch, filesystem/VFS, worker threads and framework settlement.
4. Expand from false-flag callbacks to all 395 commands; false-flag review alone is not a full registry effect graph.
5. Reconcile the command flag against its documented document/undo meaning separately from session, persistent-app and external effects. Do not label file I/O or view-state changes as a flag defect unless the documented contract says so.

## 6. Authorization boundary

This addendum is audit evidence only. No runtime implementation, command flag changes, evaluator wiring, adapter bypass, async evaluation, Asset Warp change, Smart Controller/FDM composition, merge, PR closure or release was performed or authorized. S0 remains OPEN.
