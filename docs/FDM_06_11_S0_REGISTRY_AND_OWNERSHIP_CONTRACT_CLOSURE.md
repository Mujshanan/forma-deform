# FDM-06.11 — S0 Registry and Ownership Contract Closure

**Status:** S0 closure assessment; implementation remains blocked  
**Parent:** FDM-06.10, PR #75, head `af162e492e87fc1864641f6f18e54f544b1016e2`  
**AniCraft baseline:** `Mujshanan/anicraft` `main` at `2df7775907d1458ce653e7e6314ec8a5d7d565f5`  
**Tree:** `80071fb3f714944e49608668871d4cf53a6159a5` (complete; 413 Rust files)  
**Purpose:** Convert the S0 findings into a source-pinned command inventory, explicit ownership/dispatch contract, consumer migration map, and evidence-based gate decision.  
**Change policy:** Read-only audit of AniCraft. No AniCraft runtime changes, no FDM runtime changes, no host tests executed, no PR merges, and no implementation authorization.

---

## 1. Executive decision

FDM-06.11 advances S0, but **does not close S0**.

The 52 command-spec sources registered by `engine::commands()` have been enumerated at the command-ID and declared `mutates`-flag level. The current owner/dispatch model is also stated as a normative contract. The audit has not, however, proven the actual direct and indirect write set and test coverage for every command, run AniCraft's CI on the exact pinned commit, or completed a compiler-backed workspace/downstream consumer inventory.

| S0 item | FDM-06.11 result | Gate |
|---|---|---|
| S0-A — Source baseline and workspace inventory | Reconfirmed; 52 command-spec sources and their source blobs pinned | Complete |
| S0-B — Per-command mutation/effect registry | All statically declared command IDs and `mutates` flags inventoried; actual effects and test mapping still require body-level verification | Partial |
| S0-C — Owner-thread and dispatch contract | One owner-thread contract selected and mapped to observed dispatch paths; runtime assertion/enforcement is not implemented | Contract defined; enforcement pending |
| S0-D — Exact host CI/test baseline | No workflow runs or status contexts surfaced for the exact host commit; configured CI commands identified, not executed here | Blocked |
| S0-E — Public API consumer migration inventory | Major in-repository consumer surfaces mapped; exhaustive workspace and external downstream inventory is not proven | Partial |

**S0 exit is not met.** In particular, the registry must not be represented as a complete effect manifest merely because a command's `mutates` flag is true or false.

---

## 2. Reconciled source baseline

The following baseline is unchanged from FDM-06.10:

- AniCraft commit: `2df7775907d1458ce653e7e6314ec8a5d7d565f5`
- Commit message: merge PR #525, principal-engineer review
- Recursive tree SHA: `80071fb3f714944e49608668871d4cf53a6159a5`
- Recursive tree was reported complete, not truncated.
- Rust files: 413.
- `engine::commands()` registers 52 command-spec sources.
- `crates/engine/src/lib.rs` blob: `de144ad9996bf1c29c52c28135b8f00a359ec338`.

Critical source blobs, all fetched at the pinned host commit:

| Source | Blob SHA |
|---|---|
| `crates/engine/src/lib.rs` | `de144ad9996bf1c29c52c28135b8f00a359ec338` |
| `crates/doc/src/lib.rs` | `59559a0b4599fd0ec34c720004b42ceda3080883` |
| `crates/engine/src/history.rs` | `204847cfb538942b495ef58e21a90d62fbfa1ee4` |
| `crates/engine/src/format.rs` | `2120a515ccdddfa7a5404294be884f4290783466` |
| `crates/engine/src/recovery.rs` | `5053f4a32de241cb536f46e6b8d3c73cb75cd88c` |
| `crates/engine/src/cmds_file.rs` | `5b834a0ce1347c5f612a71a0cae56dff6915f7b0` |
| `crates/engine/src/cmds_recovery.rs` | `c1a7f8058a92d90c3f53729b7809a6d613fdd63e` |
| `crates/engine/src/transfer.rs` | `7bbee132c5d0b9ae96d4b96d499d550e2bd99bdf` |
| `crates/engine/src/editing.rs` | `43a6c919e324508db5cf8ccb5d418eeae876cbd5` |
| `crates/engine/src/parenting.rs` | `1df1796968874a5a4b583ed5aa2eea02741a1c69` |
| `crates/engine/src/cmds_motion.rs` | `75a52d1a92a0babfb9753c0ab1704b6ede3819cf` |
| `crates/engine/src/cmds_scene.rs` | `f48b844e534097fc1a8c3b96c1bcd8221a43f030` |
| `crates/engine/src/cmds_group.rs` | `40fec49ebaf6cf45c8e173162e601e3febd68bdd` |
| `crates/engine/src/cmds_modify.rs` | `9bf988164ea0789d45222cbd65cf887661a2605a` |
| `crates/engine/src/cmds_timeline.rs` | `7e9191f5f9dcf467faf11e1981a161978840488f` |
| `crates/engine/src/cmds_frames.rs` | `30f4bffc596f1b86bb6f0a8135bb01fe99ca5789` |
| `crates/engine/src/cmds_draw.rs` | `eb279b835e248f97a397b9611ce0b00b1672470d` |
| `crates/engine/src/cmds_view.rs` | `1e0885632c1d897a71c24d30d1373d7a4a2c75fe` |
| `crates/engine/src/cmds_library.rs` | `9d9b85671d24646774f79926d73ba495fd78ab63` |
| `crates/engine/src/cmds_plugin.rs` | `48cea8e7a94d01f7e9733deac6a1ddb1ef7f8440` |
| `crates/engine/src/render_service.rs` | `abf5c53b83a5d87ef6d6eb1811fb4b07dbabe676` |
| `src/app.rs` | `687524a801286094d1a62c69b47f415f7c4151e3` |
| `src/remote.rs` | `83554d9ac559d440edaabbf1230d79f48e691829` |
| `src/autosave.rs` | `7253613f4bf429c62312cdde58dc1608224418e7` |
| `src/tabs.rs` | `9986017f309c47dcbeaad1b9d5b2e09afb0626d2` |
| `src/stage.rs` | `704c8ca8e065dbafdb2be32ec69753c6d3e06233` |
| `src/timeline.rs` | `c0553edebd08c62ad53355d3a704c08d80ea8235` |
| `crates/cli/src/lib.rs` | `6c67d3e06795b02bb9c98307d1e911e596e827a6` |
| `crates/cli/src/mcp.rs` | `13aa801b89ce939d40ddba8ad891b18b474f22b5` |
| `.github/workflows/ci.yml` | `0a4733afb72f12bfd6a2b0eb81967a37b954c104` |

The source pins are audit evidence for this commit only. They must be refreshed if AniCraft `main` advances before any implementation branch is authorized.

---

## 3. Command-ID inventory

### 3.1 Interpretation rules

The registry below records the literal command IDs found in each `specs()` source and the declared `CommandSpec.mutates` value.

- **T** — declared `mutates: true`; the command is intended to mutate document state through the command path.
- **F** — declared `mutates: false`; this only means the command is not declared as a document-history mutation. It does not prove the command is side-effect-free.
- **External side effect** — filesystem, preferences, operating-system integration, or other state outside the persistent Document.
- **Body audit pending** — actual direct/indirect writes, settlement interactions, transaction boundary, and regression tests have not been exhaustively mapped for that ID.

The command registry in `engine/src/lib.rs` exposes `RunFn = fn(&mut Session, &Value)` and public `CommandSpec.run`. The `mutates` flag is currently a runtime history/dispatch selector, **not an effect declaration or security boundary**.

### 3.2 Registry by source

The IDs are grouped by their declared flag. Semicolon-separated values are deliberate so this table can be mechanically split into a normalized per-command inventory. A source with no listed IDs is not silently considered complete; all 52 sources are listed.

| Source module | Blob SHA | T — declared document mutation | F — declared non-document mutation |
|---|---|---|---|
| `cmds_file` | `5b834a0ce1347c5f612a71a0cae56dff6915f7b0` | `file.import; doc.set_stage` | `file.new; file.new_sample; file.open; file.save; file.save_as; file.export_png; file.export_svg; file.export_video; file.export_sprite_sheet; file.export_png_sequence; file.export_svg_sequence; file.export_gif; file.export_apng; file.export_webp; file.export_lottie; file.export_html; edit.undo; edit.redo; doc.info` |
| `plugin_export` | `2604e904f85ca7a05c5499f787eb9c2a4401f214` | — | `file.export_plugin; file.export_formats` |
| `cmds_envelope` | `bc71d8b3ab34ff93a027edde150dfb7c89508a07` | `modify.envelope_move; modify.envelope_close` | `modify.envelope` |
| `publish` | `6b206131e237833f7cddbfafede2c91de8305b46` | `publish.settings_set` | `publish.settings_get; file.publish` |
| `cmds_recent` | `c2de550859a3d631612aaf328acbcad5281f6fae` | — | `file.open_recent; file.recent_list; prefs.recent_pin; prefs.recent_remove; prefs.recent_clear` |
| `cmds_desktop` | `1a67cc95e79aaba4951265585f61f0f1d70fdee0` | — | `desktop.integrate; desktop.remove; desktop.status` |
| `cmds_drop` | `5282321384d6145ed7a9730f94828856cf3cd383` | `file.drop` | — |
| `cmds_recovery` | `c1a7f8058a92d90c3f53729b7809a6d613fdd63e` | — | `file.recovery_list; file.recover; file.discard_recovery; file.autosave` |
| `cmds_timeline` | `7e9191f5f9dcf467faf11e1981a161978840488f` | `timeline.insert_frame; timeline.remove_frame; timeline.insert_keyframe; timeline.insert_blank_keyframe; timeline.clear_keyframe; timeline.move_keyframes; timeline.move_spans; timeline.create_motion_tween; timeline.create_tween; timeline.create_shape_tween; timeline.remove_tween; timeline.set_easing; timeline.set_orient; timeline.set_keep_area; timeline.set_label; layer.new; layer.new_folder; layer.delete; layer.rename; layer.toggle_visible; layer.toggle_lock; layer.toggle_outline; layer.move_up; layer.move_down; layer.move; layer.toggle_mask; layer.set_type; layer.add_motion_guide` | `timeline.goto; timeline.next_frame; timeline.prev_frame; timeline.first_frame; timeline.last_frame; layer.select` |
| `cmds_layer` | `7b68d79227dee59fb9d776188898bd6aee250437` | `layer.set_onion; layer.set_color; layer.set_height; layer.set_opacity; layer.set_mask; layer.set_properties` | `layer.properties` |
| `cmds_camera` | `579fe605763c27146a80d6b73e5730d6eb67af22` | `camera.add; camera.remove; camera.set; layer.set_depth; layer.remove_depth_key; layer.set_attached` | `camera.info; layer.depths` |
| `parenting` | `1df1796968874a5a4b583ed5aa2eea02741a1c69` | `layer.set_parent; layer.remove_parent` | `layer.parents` |
| `cmds_bone` | `24f93d85ded5b99b88a6e9e86af368d5f962c74d` | `bone.add; bone.add_in_shape; bone.bind; bone.paint_weights; bone.set_weight; bone.remove; bone.set_limits; bone.set_translation; bone.set_spring; bone.enable_springs; bone.pin; bone.pose; bone.solve` | `bone.info` |
| `cmds_warp` | `3d3d923b9ce1ef2d9243d1f79814f00cbba4ec36` | `warp.add; warp.pin_add; warp.pin_move; warp.pin_remove; warp.remove` | `warp.info` |
| `cmds_motion` | `75a52d1a92a0babfb9753c0ab1704b6ede3819cf` | `motion.set_key; motion.remove_key; motion.set_ease; motion.set_path; motion.clear_path; motion.orient; motion.add_point; motion.delete_points; motion.convert_point; motion.move_path; motion.pin_point; motion.unpin_point` | `motion.info` |
| `cmds_retime` | `83ecae6999b8fc2ff152f7a4522069c08fca7c10` | `motion.move_keys; timeline.resize_span; motion.split; motion.join` | — |
| `cmds_ease` | `eea07a7cb19d10911fe9d75d1ac00b3ae029f50d` | `ease.set; ease.add_point; ease.move_point; ease.move_handle; ease.delete_point; ease.convert_point; ease.separate; ease.save; ease.delete; ease.rename; ease.paste` | `ease.info; ease.list; ease.copy` |
| `cmds_hints` | `63f942a417ca247a505c5879f5252fffa0b465e2` | `tween.add_hint; tween.move_hint; tween.remove_hint; tween.clear_hints` | `tween.hints` |
| `cmds_scene` | `f48b844e534097fc1a8c3b96c1bcd8221a43f030` | `scene.new; scene.duplicate; scene.rename; scene.delete; scene.move_up; scene.move_down` | `scene.select; scene.prev; scene.next` |
| `cmds_frames` | `30f4bffc596f1b86bb6f0a8135bb01fe99ca5789` | `timeline.cut_frames; timeline.paste_frames` | `timeline.copy_frames` |
| `onion` | `9adfa704b2fcd92594c359eff13e3f5d34442e2d` | — | `frames.artwork` |
| `multi` | `7f795c8be4a83c2cd757d39f817c21e3f9210da0` | `frames.edit_multiple; frames.transform; frames.align; frames.delete; frames.cut; frames.paste; frames.merge` | `frames.copy` |
| `cmds_draw` | `eb279b835e248f97a397b9611ce0b00b1672470d` | `draw.rect; draw.ellipse; draw.star; draw.line; edit.delete; edit.duplicate; element.translate; element.set_transform; element.set_fill; element.set_stroke; element.set_alpha; element.set_color_effect; element.set_blend; element.set_filters; element.set_loop; symbol.set_kind; symbol.set_scale_grid; element.rename; arrange.bring_to_front; arrange.bring_forward; arrange.send_backward; arrange.send_to_back; modify.convert_to_symbol; modify.convert_to_bitmap; modify.trace_bitmap; modify.break_apart; library.place` | `draw.object_drawing; draw.snap_to_objects; edit.select_all; edit.deselect; element.select; element.pick` |
| `cmds_view` | `1e0885632c1d897a71c24d30d1373d7a4a2c75fe` | `view.grid_set; view.guides_set; view.guide_add; view.guide_move; view.guide_delete; view.guides_clear` | `view.rulers; view.grid_show; view.grid_snap; view.guides_show; view.guides_lock; view.guides_snap; view.snap_pixels; view.aids` |
| `cmds_group` | `40fec49ebaf6cf45c8e173162e601e3febd68bdd` | `modify.group; modify.ungroup; modify.combine; modify.union; modify.intersect; modify.punch; modify.crop` | — |
| `cmds_library` | `9d9b85671d24646774f79926d73ba495fd78ab63` | `library.new_folder; library.rename; library.move; library.duplicate; library.delete; library.delete_unused; library.set_smoothing; element.swap_bitmap; element.swap_symbol` | `library.items; library.unused` |
| `merge` | `1d3b0c2763ffc1d17a4d2e0bfc5d6fb1131ce5f9` | `draw.drop; draw.float` | `element.select_connected` |
| `cmds_modify` | `9bf988164ea0789d45222cbd65cf887661a2605a` | `edit.cut; edit.paste; edit.paste_in_place; modify.transform; modify.rotate_cw; modify.rotate_ccw; modify.flip_h; modify.flip_v; modify.transformation_point; modify.reset_transformation_point; modify.remove_transform; align.left; align.hcenter; align.right; align.top; align.vcenter; align.bottom; align.distribute_h; align.distribute_v` | `edit.copy; modify.transform_origin` |
| `cmds_path` | `7c7201db127e6ab87d80a456e282cc90c7fc1209` | `draw.path; draw.freehand; path.move_points; path.add_point; path.delete_points; path.convert_point; path.extend` | — |
| `cmds_raw` | `093cd8d684e1b658869a0ef91741e55ed34f31c0` | `path.bend_segment; raw.move_point; raw.select_rect; raw.merge` | `raw.hit` |
| `cmds_rawsub` | `58b162ad03fe0eb6c643b59b9dd3783c160a4347` | `raw.move_handle` | `raw.subselect; raw.subselect_rect; raw.points` |
| `cmds_rawpoints` | `fb04262d8d4e16493fe7bb5a5e8a34f0c157d5db` | `raw.add_point; raw.delete_points; raw.convert_points` | `draw.snap` |
| `editing` | `43a6c919e324508db5cf8ccb5d418eeae876cbd5` | — | `symbol.edit; symbol.exit` |
| `find` | `e84ea069f5e303959c6f17b5a56d03351ccfa709` | `find.replace` | `find.search; find.goto` |
| `explorer` | `55c2431b938e5c352996d8c42dc81db74a198a20` | — | `explorer.tree` |
| `cmds_brush` | `0dbb2ce79940dee29f1f39f17ba57efaab0428c9` | `draw.brush` | `brush.preset_list; brush.preset_save; brush.preset_rename; brush.preset_delete; brush.preset_export; brush.preset_import` |
| `cmds_filter` | `34cb2de3791432d1c19b153e68ee46c6f7399c73` | `filter.move; filter.enable; filter.remove; filter.paste; filter.preset_apply` | `filter.copy; filter.preset_list; filter.preset_save; filter.preset_rename; filter.preset_delete; filter.preset_hide; filter.preset_export; filter.preset_import` |
| `cmds_erase` | `f45bf51d2abfc1ab0fb1813ebf4e0acc0c321856` | `draw.erase; draw.faucet; draw.erase_all` | — |
| `cmds_paint` | `320d3ff69177209e169b46211065994a48884f71` | `paint.bucket; paint.ink; paint.apply_style; paint.set_fill; paint.transform_fill` | `paint.pick; paint.lock_fill` |
| `cmds_swatch` | `7c0d3afed85405d099ea33122c4508c28e52ec1d` | `swatch.add; swatch.remove; swatch.move; swatch.duplicate; swatch.rename; swatch.apply; swatch.clear; swatch.reset; swatch.import` | `swatch.list; swatch.export` |
| `cmds_data` | `8ab9aca1089024af13241cd528c35e1399d1d177` | `data.import_table; data.set_table; data.set_cell; data.set_key; data.remove; data.bind; data.unbind; data.bake` | `data.list; data.get; data.bindings` |
| `cmds_text` | `0ee4e81555013f0ee37b708a5abd41ec293f2d26` | `text.create; text.edit; text.style; text.paragraph; text.attach_path; text.path; text.release_path; text.field` | `text.styles; text.fonts` |
| `cmds_width` | `3bc2088a2da63b59eb029d6a49cbbea401b1e477` | `stroke.set_profile; stroke.set_width_point; stroke.delete_width_point` | `stroke.width_at` |
| `cmds_stroke` | `f5d505e1a8b6ccd084fcb84d556b4ffdb4e83adc` | `stroke.set_style` | — |
| `cmds_shape` | `e17a045b22f9b61120afbe27eb7e0214a2589c90` | `modify.lines_to_fills; modify.expand_fill; modify.soften_fill_edges` | `paint.sample` |
| `cmds_plugin` | `48cea8e7a94d01f7e9733deac6a1ddb1ef7f8440` | `plugin.run` | `plugin.install; plugin.list; plugin.remove; plugin.reload` |
| `cmds_sound` | `ce0d4703f7258c194924212c8d27e048fa62d16c` | `file.import_to_library; sound.place; sound.set; layer.toggle_mute; layer.set_volume; sound.remove; sound.delete; sound.rename` | `file.export_audio` |
| `cmds_audio_keys` | `8f96f3b97abf206a77146b6f5c8e6066c7bb280f` | `sound.to_keyframes; sound.lip_sync` | `sound.analyse` |
| `cmds_video` | `df83241095e2da7b5b5edf921b20adf7314c4093` | `file.import_video; layer.set_video` | `file.video_info` |
| `cmds_actions` | `02a4f0a61c86a8268233e84b39a93ec9a18cbfda` | `actions.set; actions.clear; instance.set_name` | `control.test_movie; actions.list` |
| `cmds_script` | `0ddb7de1ef80b72b1640d1bacb6bf41ab3901b8e` | `script.set; script.clear; script.set_class` | `script.list; script.check` |
| `cmds_macro` | `82cafb669a9c65e33122189e34de6d0ccf33dcd5` | `macro.run` | `macro.list; macro.history; macro.save; macro.rename; macro.delete; macro.move; macro.import` |

**Registry caveat:** The table is a static extraction of command declarations. It does not yet prove the command's complete effect set. Examples requiring special treatment include:

- `file.open`, `file.new`, `file.recover`, `edit.undo`, and `edit.redo`: they replace or restore Session/document state even though their command spec is declared non-mutating for the normal command-history branch.
- `file.save`, exports, publish, desktop integration, preset commands, and macro commands can have external side effects despite `mutates: false`.
- `timeline.goto`, `scene.select`, `symbol.edit`, and `symbol.exit` alter evaluation context without being persistent document edits.
- `settle_cut`, `settle_floating`, `settle_rig`, `finish_motion_auto_key`, gesture completion/cancellation, and plugin nested execution can add indirect writes around command dispatch.
- The command module source is not the only mutation owner: public Document APIs, Session methods, import/transfer, history restore, recovery, and direct public field access must also be represented in the final effect registry.

### 3.3 Required normalized registry schema

The inventory above is the command-ID seed. The implementation-facing registry must normalize to one row per command and use at least these fields:

```text
command_id
source_module
source_blob_sha
declared_mutates
direct_document_write
indirect_document_write
session_context_write
external_side_effect
identity_transition
evaluation_dependency_effect
transaction_entry
rollback_owner
regression_test_ids
coverage_status
reviewed_at_host_sha
```

Unknown or unreviewed fields must fail closed. The future CI guard must compare the registered command IDs with `commands()` at build/test time and fail if a new command appears without a registry entry. A generated registry is preferred over hand-maintained duplicate IDs.

---

## 4. Mutation and effect ownership contract

### 4.1 Normative owner

**The live Session owner is the sole authority allowed to mutate Session or persistent document state.**

- In the desktop GUI, the owner is the application event-loop thread that owns `App` and its live `Session`.
- Remote/control-port requests are messages. They are processed by the app's polling/dispatch path and must execute on that same owner thread; the remote client thread must never receive `&mut Session`.
- In CLI/MCP mode, the thread running the sequential request loop owns the Session for that loop. Requests are handled one at a time through `&mut Session`.
- A plugin executes as a nested command inside the existing owner-thread transaction. It receives only the existing constrained plugin ABI; it does not receive the persistent-state bundle or a new mutable Session capability.
- Worker threads receive immutable owned snapshots or cloned read-only data. They return immutable results/messages to the owner. They never mutate Session, Document, history, identity ledgers, or epochs.

This matches the observed call patterns in `src/remote.rs`, `crates/cli/src/lib.rs`, `crates/cli/src/mcp.rs`, `src/autosave.rs`, and `crates/engine/src/cmds_plugin.rs`. The contract formalizes those patterns; it does not claim the current code enforces them.

### 4.2 Enforcement required in the host implementation

The implementation must make this contract mechanically enforceable:

1. Session mutation APIs and persistent document access become private or capability-scoped; unrestricted `pub doc`, public mutable fields, and mutable nested Document APIs cannot remain escape hatches.
2. A private owner token is established when Session is constructed. Transaction entry validates that the current execution context holds the owner capability. A debug/test owner-thread assertion is required where platform support permits.
3. The public command-description API must not expose `CommandSpec.run` as a callable `fn(&mut Session, ...)`. Public metadata can expose ID/label/params/mutates; invocation must route through the owned dispatcher.
4. All command bodies, nested plugin commands, and settlement phases execute against one transaction draft/journal.
5. Gesture updates are explicit transactions or transaction substeps. Gesture cancellation restores authored state but still advances runtime freshness.
6. Undo, redo, recovery, open/new, and document replacement are explicit state transitions owned by the Session owner.
7. Worker completion is a message, not a mutation. The owner validates the complete evaluation stamp before accepting any result.
8. No FDM asynchronous evaluation is introduced in this migration.

### 4.3 Transaction receipt and effect policy

A committed transaction receipt is emitted only after command body, rig/motion settlement, cut/floating settlement, validation, and persistent-state publication have completed.

Every transaction reports typed effects. If an operation cannot yet identify precise target/dependency effects, it emits a conservative document/session-wide invalidation effect. The system must not infer "no effect" from a command's `mutates: false` flag.

External side effects (filesystem, process invocation, OS integration, preferences) must be classified separately from persistent document mutation. They need preflight/commit or post-commit failure semantics; they must not be silently folded into a document transaction without rollback guarantees.

---

## 5. Public API and consumer migration inventory

### 5.1 Verified consumer surfaces

| Consumer | Pinned source evidence | Current dependency | Migration responsibility |
|---|---|---|---|
| Engine command registry | `crates/engine/src/lib.rs` | Public `RunFn`, `CommandSpec.run`, public `Session.doc` and other fields | Private dispatcher/callback; immutable command metadata; transaction capability |
| GUI app shell | `src/app.rs` | Extensive `self.s.doc` reads, direct Session context reads, gesture calls | Replace with immutable document/session view and explicit owner methods |
| Stage UI | `src/stage.rs` | Document reads for rendering/picking; gesture start/end | Read-only snapshot/view; gesture transaction API |
| Timeline UI | `src/timeline.rs` | Document/timeline reads and playhead/context reads | Immutable view plus explicit context setters |
| Tabs | `src/tabs.rs` | Ends gestures and switches/restores per-document Session state | Explicit document/session transition transaction |
| Remote/control bridge | `src/remote.rs` | App-owned Session; remote calls dispatched through app polling | Preserve message-to-owner dispatch; prohibit remote-thread Session access |
| CLI control server | `crates/cli/src/lib.rs` | Public request handlers accept `&mut Session`; sequential serve loop | Retain one owner per Session; route all mutations through transaction API |
| CLI MCP host | `crates/cli/src/mcp.rs` | Reads Session document and invokes `Session::execute` | Read-only accessors and owner-scoped dispatcher |
| Autosave | `src/autosave.rs` | Clones `s.doc` for worker; tracks StateId/revision | Capture one immutable PersistentDocumentState snapshot and bundle version |
| Plugin host | `crates/engine/src/cmds_plugin.rs` | Nested commands borrow mutable Session internally | Nested transaction draft/effect journal; preserve one outer undo step |
| History | `crates/engine/src/history.rs` | Snapshot stores Document and editor context | Snapshot full persistent bundle; keep runtime epoch/session ID out of history |
| Render service | `crates/engine/src/render_service.rs` | Worker jobs carry cloned Document and StateId-derived keys | Continue immutable worker inputs; FDM acceptance gate remains host-owned and separate |

### 5.2 Public mutable API escape hatches

The implementation migration must account for all of the following, not only `Session.doc`:

- Public Session fields: document, scene, editing context, frame, layer, selection, revision, state, path, and other mutable editor/session fields.
- Public `RunFn` type and `CommandSpec.run` callback.
- Public Document fields and public nested mutable methods such as `alloc_id`, `timeline_mut`, `layer_mut`, `keyframe_mut`, and mutable element accessors.
- Public re-export of `anicraft_doc` from the engine crate.
- Internal app panels, CLI, MCP, tests, autosave, recovery, transfer, export, and plugin adapters.
- External crates/downstream consumers of the public engine/doc API.

### 5.3 Inventory completeness limit

The source audit confirms the major workspace consumers above and representative direct field/method usage. It did **not** run a full workspace compile with the proposed fields privatized, enumerate every reference in all 413 Rust files, or inspect every external downstream crate. Therefore this is a migration ownership map, not proof that the public API break is fully enumerated.

Before S0-E closes, produce a compiler-backed consumer report on a temporary API-privacy branch or equivalent exhaustive static scan. Group every consumer by crate/file/API and assign a migration owner. External downstream compatibility must be treated as a deliberate public breaking change, not assumed absent.

---

## 6. Non-command mutation inventory and ownership

| Mutation route | Current owner/path | Required future owner | Required effect |
|---|---|---|---|
| Registered mutating command | `Session::execute` + public command callback | Session transaction coordinator | Command effects + one committed receipt |
| Declared non-mutating command with context changes | command callback / Session fields | Session context transaction | Context generation; no persistent document epoch unless dependencies change |
| Cut settlement | `settle_cut` / healing helpers | Active transaction coordinator | Geometry/topology/identity effects |
| Floating-paste settlement | `settle_floating` | Active transaction coordinator | Create/replace target effects |
| Rig/parent settlement | `editable_keyframe`, `settle_rig`, `rig_end` | Active transaction coordinator | Transform/hierarchy effects |
| Motion auto-key settlement | `finish_motion_auto_key` and related state | Active transaction coordinator | Timeline sample/dependency effects |
| Gesture preview/end/cancel | `begin_gesture`, `end_gesture`, `revert_gesture` | Session-owned gesture transaction | Epoch advances for visible preview and cancellation; undo grouping remains compatible |
| Undo/redo/history travel | History snapshot restore | Session transaction coordinator | Restore persistent bundle; allocate fresh runtime epoch |
| New/open/recovery/document replace | File/recovery commands and `replace_document` | Session owner transition transaction | New session identity; reject prior-session results |
| Import/clipboard transfer | `cmds_file`, `cmds_sound`, `cmds_video`, `transfer.rs` | Session transaction coordinator + identity remapper | New/remapped identities and conservative invalidation |
| Direct public Document mutation | Public fields/mutable methods | Prohibited outside transaction capability | Must become unrepresentable |
| Plugin nested commands | `execute_nested` via plugin SessionHost | Existing outer transaction | One atomic outer receipt/undo unit |
| Autosave/recovery workers | Immutable Document clones / file paths | Snapshot-only workers; owner applies completion | No Session or mutable Document capability on worker |

The exact transaction ownership of every helper remains part of S0-B body-level verification. This table establishes the known route families and intended owner, not a claim that every call site has already been migrated.

---

## 7. CI and test baseline evidence

### 7.1 Configured workflow

The pinned AniCraft `.github/workflows/ci.yml` configures:

```sh
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

The workflow also includes cargo-deny license, ban, and source checks.

### 7.2 Exact commit status observed

For host commit `2df7775907d1458ce653e7e6314ec8a5d7d565f5`:

- The GitHub commit workflow-run lookup returned no PR-triggered workflow runs.
- The combined commit status lookup returned no status contexts.
- No AniCraft `cargo test`, `cargo clippy`, cargo-deny, benchmark, or platform test was executed by this audit.
- No pass or fail result is claimed.

The absence of returned PR-triggered runs/status contexts is not proof that the commit has never been tested locally or by a non-PR workflow. It means exact baseline execution evidence is not available from the inspected GitHub checks.

### 7.3 Required baseline evidence before S0-D closes

Run the pinned host commit in the project-supported Windows environment (Windows is mandatory) and record:

- exact SHA and clean/dirty state;
- Rust toolchain and target details;
- exact workflow/run URL or reproducible command log;
- `cargo test --workspace` result;
- `cargo clippy --workspace --all-targets -- -D warnings` result;
- cargo-deny license/ban/source results;
- GPU/font/ffmpeg/node availability and whether optional tests skipped;
- pre-existing failures, classified separately from any FDM-related changes.

If the exact old commit cannot be tested because the toolchain or dependencies are unavailable, record that as a blocked baseline; do not substitute a later `main` result without explicitly changing the source pin.

---

## 8. S0 closure requirements and decision

### S0-B — Complete mutation/effect registry

To close:

1. Normalize the 52-source command-ID seed into one row per command.
2. Inspect every command body and helper call for direct/indirect persistent writes, Session context writes, external side effects, and identity changes.
3. Explicitly audit every `mutates: false` command that can settle state, change context, save files, touch preferences, or invoke external processes.
4. Map every command to transaction entry, rollback owner, effect type, and concrete regression tests.
5. Include non-command mutators and public Document APIs.
6. Add a machine-checkable registry completeness test; a new command or mutation helper must fail closed until classified.

**Current result:** Command ID and declared flag seed exists; effect and test mapping incomplete.

### S0-C — Owner-thread and dispatch contract

To close:

1. Confirm the app event-loop thread as desktop Session owner.
2. Confirm remote/control requests are only messages dispatched by that owner.
3. Confirm CLI/MCP owns Session on its sequential request-loop thread.
4. Confirm plugins are nested under the existing command transaction and never receive a persistent-state capability.
5. Confirm workers only receive immutable snapshots and return messages/results.
6. Implement the owner token/assertion in the first authorized host implementation stage; no worker or remote path may acquire mutable Session.

**Current result:** Normative contract and observed paths documented. Runtime enforcement is not present and belongs to implementation.

### S0-D — Exact host CI/test baseline

**Current result:** Blocked. No exact host run or status context was returned for the pinned commit, and this audit did not execute AniCraft tests.

### S0-E — Public API consumer migration inventory

To close:

1. Enumerate all in-workspace references to public Session fields, `RunFn`, `CommandSpec.run`, and public mutable Document APIs.
2. Run a compile-consumer inventory on a temporary privacy/capability branch to expose missed consumers.
3. Group app UI, CLI, MCP, remote, plugin, autosave, recovery, history, render, tests, and external API users.
4. Assign each consumer an immutable-view or owner-scoped mutation migration.
5. Document the public breaking-change policy for downstream crates.

**Current result:** Major consumer surfaces are mapped; exhaustive compile/static proof is incomplete.

### Final gate matrix

| Gate | Result |
|---|---|
| Baseline commit and source hashes fixed | PASS |
| All 52 command-spec source modules enumerated | PASS |
| Command IDs and declared `mutates` flags seeded | PASS |
| Every command's actual effects and regression tests mapped | NOT YET |
| Non-command mutation helpers assigned and call paths exhaustively checked | PARTIAL |
| Owner-thread/dispatch contract selected | PASS |
| Owner-thread enforcement implemented | NOT YET — implementation stage |
| Exact AniCraft host CI baseline recorded | BLOCKED |
| Full workspace API consumer inventory proven | PARTIAL |
| No unclassified mutation entry points remain | NOT YET |

**Decision: S0 remains open. FDM-06.11 is an evidence and contract closure checkpoint, not a host implementation authorization.**

---

## 9. Explicit constraints preserved

- No AniCraft runtime or FDM runtime code changed.
- No DocumentId, IncarnationId, extension envelope, transaction API, or evaluation epoch implemented.
- No plugin ABI changes or ABI v1 retirement.
- Native Asset Warp remains unchanged and host-owned.
- Smart Controller and Form Deform evaluation remain independent under CCC-01.5.
- No Asset Warp/FDM composition or multi-deformer composition.
- No asynchronous FDM evaluation.
- No bitmap, text, symbol, group-wrapper, nested-symbol, or nonlinear-parent FDM support added.
- No apply/bake implementation.
- No PR merged, closed, or retargeted.
- No implementation authorization implied.

The hard implementation gate remains: host mutation ownership, persistent-state atomicity, and stale-result rejection must be implemented and verified in AniCraft, and implementation must be separately authorized.

---

## 10. Recommended continuation

Do not start S1 or any FDM runtime work.

The next bounded activity is to close the evidence gaps identified here:

1. Complete body-level effect and test mapping for the 52 command sources and non-command mutation paths.
2. Obtain an exact Windows CI/test baseline for the pinned AniCraft SHA.
3. Finish the compiler-backed public API consumer inventory.
4. Re-review the S0 gate matrix and only then propose the separately authorized host implementation stage.

No later FDM milestone should assume S0 closure until these items have evidence.
