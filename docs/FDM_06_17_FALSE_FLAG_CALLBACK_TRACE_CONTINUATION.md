# FDM-06.17 — False-Flag Callback Trace Continuation

**Disposition: PARTIAL / IN PROGRESS. S0 remains OPEN.**  
**Pinned AniCraft source:** `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Pinned tree:** `80071fb3f714944e49608668871d4cf53a6159a5`  
**Parent evidence:** FDM-06.13–06.16, PR #80  
**Scope:** Additional source-located callback traces for session state, clipboard, external I/O, plug-in/macro lifecycle, navigation, and player scripts.  
**Change policy:** Audit/documentation only. No AniCraft runtime code, FDM runtime code, workflow, manifest, or dependency was changed.

## 1. Baseline and method

FDM-06.16 records the uploaded `anicraft.zip` verification: SHA-256 `745cc5dc8e4b797db4590919f02185821af4de3d0264cecdfe2914768dab1b40`, embedded HEAD `2df7775907d1458ce653e7e6314ec8a5d7d565f5`, tree `80071fb3f714944e49608668871d4cf53a6159a5`, clean working tree, and `git fsck` exit 0. The uploaded file metadata still reports 25,593,223 bytes. This addendum carries that prior integrity evidence forward; it does not claim a second extraction or hash computation.

This pass re-read the listed source ranges from the pinned GitHub commit. It records direct callback-to-helper paths and first-order sinks. It does **not** claim a compiler-derived call graph, exhaustive transitive closure, or executable conformance proof.

Effect labels used below:

- **D** — persistent document/history mutation.
- **S** — live Session/editor state (selection, frame, style, clipboard, path, recent list, etc.).
- **P** — persistent application/preference or plug-in registry state.
- **F** — filesystem or OS effect.
- **R** — read-only query/computation.
- **C** — conditional effect introduced by dispatcher settlement or caller context.

The documented meaning of `CommandSpec::mutates` remains “changes the document and therefore records an undo step”; it is not a universal side-effect flag.

## 2. Additional callback traces

### 2.1 Session clipboard and preference-like state

| False-flag command family | Source path | Observed effect |
|---|---|---|
| `edit.copy` | `cmds_modify::copy` (`cmds_modify.rs:22-35, 98-126`) | Copies selected evaluated elements and dependencies, then assigns `Session.clipboard`: **S** |
| `frames.copy` | `multi::copy_cmd` (`multi.rs:20-31, 364-389`) | Copies multi-frame elements and assigns the same `Session.clipboard`: **S** |
| `timeline.copy_frames` | `cmds_frames::copy_frames` (`cmds_frames.rs:22-29, 80-104`) | Builds a `FrameClipboard`, carries referenced assets, assigns `Session.frame_clipboard`: **S** |
| `filter.copy` | `cmds_filter::copy` (`cmds_filter.rs:15-25, 191-202`) | Copies selected filter lists into `Session.filter_clipboard`: **S** |
| `brush.preset_save/rename/delete/import` | `cmds_brush::preset_save/preset_rename/preset_delete/preset_import` (`cmds_brush.rs:452-520`) | Mutates `Session.brushes`; import reads a file, export writes a file: **S + F(read/write)** |
| `filter.preset_save/rename/delete/hide/import` | `cmds_filter.rs:235-304` | Mutates `Session.filter_presets`; hide/save/rename/delete are in-memory preset changes, import reads a file, export writes a file: **S/P + F(read/write)** |
| `prefs.recent_pin/remove/clear` | `cmds_recent.rs:175-215, 230-280` | Mutates `Session.recent`: **S**; preference persistence is a separate shell/consumer edge |
| `modify.transform_origin` | `cmds_modify::transform_origin` (`cmds_modify.rs:468-485`) | Changes `Session.style.transform_origin`: **S** |
| `paint.pick/paint.lock_fill` | `cmds_paint::pick_style/lock_fill` (`cmds_paint.rs:386-415, 810-815`) | Reads the display list or supplied option and changes `Session.style`: **S** |
| `view.rulers/grid_show/grid_snap/guides_show/guides_lock/guides_snap/snap_pixels` | `cmds_view::toggle` (`cmds_view.rs:1-75`) | Changes `Session.aids`; no document edit in these callbacks: **S** |

These are not automatically defects in the documented document-mutation flag. They are still mutations that a future ownership/capability design must assign to an explicit Session or preference owner. The preference persistence boundary for brush/filter/recent state is not closed by this callback inspection.

### 2.2 Navigation and selection context

| False-flag command family | Source path | Observed effect |
|---|---|---|
| `scene.select/prev/next` | `cmds_scene::select_scene/step_scene/open_scene` (`cmds_scene.rs:1-50, 175-195`) | Changes `scene`, closes open edit levels, resets frame/layer and clears selection: **S** |
| `timeline.goto/next_frame/prev_frame/first_frame/last_frame` | `cmds_timeline::goto/step` (`cmds_timeline.rs:813-824`) → `Session::set_frame` | Changes playhead and selection maintenance without a document edit: **S** |
| `raw.subselect/subselect_rect` | `cmds_rawsub.rs:299-410` | Changes active layer/selection and returns raw-point/anchor data: **S** |
| `edit.deselect/edit.select_all/element.select/element.pick` | Registry entries in `cmds_draw`; selection-oriented command paths | Session selection changes; callback/helper closure remains to be completed: **S (provisional)** |

The context fields are public and do not share one monotonic revision with document edits. FDM-06.16 already establishes that frame changes do not increment `Session.revision`; scene/edit-level changes likewise require explicit context stamping or a separately owned context generation for any future result-acceptance boundary.

### 2.3 Filesystem and OS sinks

| False-flag command family | Source path | Observed effect |
|---|---|---|
| `desktop.integrate` | `cmds_desktop::integrate → install → refresh` (`cmds_desktop.rs:1-38, 342-350`) | Writes launcher, MIME metadata and icons under the supplied XDG data directory; may run MIME/desktop database refresh: **F + OS** |
| `desktop.remove` | `cmds_desktop::remove → uninstall → refresh` (`cmds_desktop.rs:352-360`) | Deletes owned desktop integration files and may refresh OS databases: **F + OS** |
| `macro.save/import/rename/delete/move` | `cmds_macro.rs:1-100, 465-543` | Writes/reads/renames/deletes macro JSON and rewrites `.order.json`: **F** |
| `brush.preset_export/import` | `cmds_brush.rs:480-520` | Export writes JSON; import reads JSON and changes Session preset state: **F + S** |
| `filter.preset_export/import` | `cmds_filter.rs:281-324` | Export writes JSON; import reads JSON and changes Session preset state: **F + S/P** |
| `file.export_audio` | `cmds_sound::export_audio` (`cmds_sound.rs:439-465`) | Builds the requested soundtrack range and writes a WAV file: **F** |
| `swatch.export` | `cmds_swatch.rs:15-55, 295-303` | Writes the document's swatches as JSON: **F**; callback body/source sink still needs its own row-level trace |
| `plugin.install/remove/reload` | `cmds_plugin.rs:15-30, 231-300` | Changes the Session plug-in registry/folder; install can copy a plug-in file, remove can delete an owned file, reload reads plug-in files: **S/P + F** |
| `file.autosave/discard_recovery/recover` | `cmds_recovery.rs:14-110` | Autosave writes a recovery document; discard deletes recovery files; recover replaces the live Document and marks it unsaved: **F** or **D + S** |
| `file.publish` | `publish::publish_cmd` (`publish.rs:1-65, 516-540`) | Plans and runs export steps, writes output files, and extends `Session.published`: **F + S** |

The macro-management commands are external-file operations even though their `Session` parameter is unused. They must not be treated as document edits, but their file writes/deletes need explicit effect and authorization handling.

### 2.4 Query and script-execution paths

- `data.list/get/bindings` (`cmds_data.rs:257-285, 441-450`) traverse document tables/bindings and return data: **R** in the inspected callbacks.
- `library.items/unused` (`cmds_library.rs:122-140, 413-415`) query library usage and unused entries: **R**.
- `macro.history/list` (`cmds_macro.rs:274-279, 387-412`) read saved-command/history metadata: **R**; `macro.list` also lists files.
- `publish.settings_get` (`publish.rs:531-550`) reads the document publish profile and resolves output paths: **R**.
- `control.test_movie` is more than a simple query. `cmds_actions::test_movie` creates a `ScriptRunner` and a `Movie`, then steps the movie and invokes frame scripts (`cmds_actions.rs:296-344`). The frame-script host in `player.rs:25-95, 120-145` permits reading the player view and emitting checked actions; it rejects document/element/command requests. This constrains the plug-in host surface, but the player/action execution graph and all script callbacks still require explicit review. No direct live Session/Document write is shown in this callback path.

### 2.5 Publish and callback invocation context

`file.publish` creates a `Publisher` from a cloned document and path, runs its planned exports, and records written paths in `Session.published` (`publish.rs:516-540`). The Publisher owns a separate Session snapshot and invokes command callbacks there (`publish.rs:424-488`, also recorded in FDM-06.16). This is not the editor's live Session, but it is a separate production `RunFn` invocation context that must remain distinct from live Session mutation ownership.

## 3. Consolidated mutation-route implications

The additional traces reinforce the following boundaries:

1. **Document state is not the whole Session.** Clipboard, frame clipboard, filter clipboard, brush/filter presets, recent files, tool style, view aids, scene/frame, layer and selection are independently mutable Session/editor state.
2. **False-flag callbacks have heterogeneous effects.** A callback can be read-only, change Session context, mutate preference/plug-in state, write/delete external files, or invoke constrained frame-script execution. A single boolean cannot represent that taxonomy.
3. **Outer and nested dispatch remain different.** The outer `Session::execute` false-flag branch runs settlement before and after the callback, calls group-frame and floating settlement, and has no normal snapshot/rollback/history record. `execute_nested` directly calls a false-flag callback without those outer settlement operations. Plugin and macro dispatch use the nested route (FDM-06.16).
4. **Error handling is part of the effect graph.** External writes may already have occurred before an error is returned; outer settlement may still run after a callback error. Document snapshot rollback cannot compensate filesystem, OS, preference or plug-in-registry effects.
5. **Read-only labels require path closure.** `control.test_movie`, exporters and Publisher use plug-in/script/render paths. Their effect claims must include the host capabilities actually exposed, not just the first callback body.

## 4. Consumer audit status

No new compiler-backed consumer evidence was obtained in this pass. The current quantitative evidence remains the FDM-06.14/FDM-06.16 lexical scan: 762 candidate API call-site lines across 94 files, including test and non-test code. It is not 762 unique semantic consumers and does not establish borrow mutability or compile impact.

### 4.1 Source-located production ownership paths

| Consumer | Pinned source path | Ownership/effect evidence |
|---|---|---|
| GUI application | src/app.rs; src/remote.rs:70-145 | App owns the live Session. Remote requests are queued and drained by the GUI frame loop; the Live host calls Session::execute on App.s. A document-changing remote request waits while the user is dragging and ends an open gesture before execution. No remote worker receives a mutable Session reference in this path. |
| CLI JSON-lines | crates/cli/src/lib.rs:180-225, 723-790 | open_session constructs a Session; request handlers call s.execute; serve processes one line at a time in a sequential loop. The CLI request loop is the Session owner for this process path. |
| CLI MCP | crates/cli/src/mcp.rs:118-160, 225-260 | Host and Backend are implemented over Session; command tools route through Host::execute. The headless backend owns a Session and automation roots. |
| GUI remote/MCP bridge | src/remote.rs:70-145; crates/cli/src/mcp.rs | MCP requests are routed back through the GUI app's owner loop; the bridge does not grant the remote client a mutable Session. |
| Plug-in command | crates/engine/src/cmds_plugin.rs:81-99, 251-270 | SessionHost holds &mut Session and maps plug-in command requests to execute_nested after namespace allowlisting. This is nested mutation under plugin.run, not an independent live Session owner. |
| Saved macro | crates/engine/src/cmds_macro.rs:252-271 | Each macro step is allowlisted and dispatched through execute_nested; macro.run is the outer command/undo boundary. |
| Publisher | crates/engine/src/publish.rs:420-490 | Publisher constructs its own Session from a Document snapshot and invokes the selected RunFn against that isolated Session. It is a distinct callback execution context, not the editor's live Session. |
| Autosave/recovery | src/autosave.rs:184-225; crates/engine/src/recovery.rs | Autosave passes a cloned Document and path to the saver; the worker does not receive &mut Session. The snapshot is still Document-oriented rather than the future full persistent bundle. |
| Rendering | crates/engine/src/render_service.rs:25-55, 450-615 | FrameJob owns a Document snapshot; RenderService runs a worker and caches results by render keys. This establishes immutable input transfer, not an FDM freshness-acceptance gate. |
| Multi-document tabs | src/tabs.rs:85-110, 320-365; crates/engine/src/lib.rs:735-752 | Parked tabs own Sessions. Activation swaps the active Session with the parked one and transfers shared editor state with take_shared. Any future owner/session identity must follow the Session across these swaps. |

### 4.2 Remaining consumer proof

The source-located routes above improve ownership classification but do not close the workspace migration inventory. The 762 candidates still need row-level production/test classification, read/write/borrow classification, and a replacement API/owner assignment. The scan spans app src (502 candidate lines), crates/doc (159), crates/render (48), crates/xfl (22), crates/cli (19), and crates/gpu (12). The non-app crate counts include legitimate offline document construction and immutable rendering/import/export consumers; they must not be mechanically treated as live Session mutation bypasses.

Still required:

- A compiler-backed inventory of all public Session fields and methods, RunFn/CommandSpec.run references, and nested Document mutators.
- A distinction between offline Document builders/importers/decoders and mutation of the authoritative live Document behind Session.
- Production/test classification, including inline cfg(test) modules that path-only scans miss.
- Compile-fail/API tests proving external consumers cannot invoke callbacks or mutate the live Document outside the transaction capability.
- A proof that GUI, CLI, MCP, remote, plugin, macro, Publisher, autosave, recovery, render and tab activation preserve one owner per live Session and do not leak mutable references to workers.

## 5. Updated S0 gate matrix

| Gate | Current evidence | Result |
|---|---|---|
| Uploaded archive/source identity | FDM-06.16 SHA/HEAD/tree/clean-tree/fsck record | **PASS (carried forward)** |
| Registry aggregation | 52 providers, 395 unique IDs | **PASS at registry enumeration** |
| All 131 false-flag callback locations | FDM-06.14/06.15 location and first-order index | **PASS at location/triage only** |
| False-flag direct/first-order effect traces | FDM-06.15 second pass plus this addendum | **PARTIAL** |
| Full transitive graph for 131 false-flag callbacks | Helpers, nested dispatch, macros/traits/function pointers, external sinks and error paths remain | **OPEN** |
| Full graph for 264 true-flag callbacks | Not complete | **NOT STARTED / OPEN** |
| Non-command mutation-route inventory | Session APIs, gestures, cuts/floating, history, replacement, import/transfer, plug-ins/macros, workers and context mapped | **PARTIAL** |
| Consumer proof | 762 candidate lines / 94 files; lexical only | **OPEN** |
| Host tests, Clippy, cargo-deny | No exact-baseline host run or local Cargo result | **OPEN** |
| macOS and Windows CI | No verified host run/status | **OPEN** |
| S0 closure | Mandatory evidence gates remain open | **NOT CLOSED** |

## 6. Required next work

1. Reconcile the exact original 36-row inline-callback manifest against the 131-row registry; preserve every original row and explicitly mark any unresolved callback.
2. Complete each false-flag callback's full transitive path, including error branches, helper functions, nested plugin/macro dispatch, script execution, worker calls and filesystem/OS sinks.
3. Continue the same callback graph for the remaining 264 commands; the false-flag subset alone cannot establish command-registry safety.
4. Turn the 762 consumer candidates into a typed inventory: production/test, read/write, caller path, owning thread/session, transaction route, and replacement capability. Then compile-check the workspace migration.
5. Obtain the actual host Linux tests/Clippy/cargo-deny and macOS/Windows workflow results on the pinned source baseline or a source-identical audit branch. No pass may be inferred from the FDM repository's own CI.
6. Submit a separate S0 closure decision only after the evidence is reproducible and all required gates pass.

## 7. Non-authorization

This addendum is audit-only. It authorizes no command-flag change, Session/Document API change, host mutation-boundary implementation, FDM evaluator, AniCraft production integration, adapter bypass, asynchronous evaluation, apply/bake, Native Asset Warp change, Smart Controller/FDM composition, PR merge, PR closure, release or deployment.

**S0 remains OPEN.** The remaining mandatory gates are unchanged: S0 must formally close; FDM-06.6 must then be implemented and verified; and the user must explicitly authorize the implementation stage.
