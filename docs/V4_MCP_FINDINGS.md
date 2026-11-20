# V4 MCP Slider — feasibility findings

**Status:** Prototype implemented; live AniCraft validation pending  
**Audited:** 2026-11-20  
**AniCraft repository:** [Mujshanan/anicraft](https://github.com/Mujshanan/anicraft)  
**Forma repository:** [Mujshanan/forma-deform](https://github.com/Mujshanan/forma-deform)

## Executive result

The documented AniCraft command registry contains a viable read-and-drive path for an **experimental global-playhead controller**:

- `bone.info` reads a bone's evaluated keyed joint angle at an explicit frame and armature layer.
- `timeline.goto` moves the current scene's global playhead to a 0-based frame.
- MCP exposes both through `anicraft_commands` and `anicraft_run`; the existing `--bridge` mode routes tool calls to the open desktop session.

This does **not** establish a Rigby2D-equivalent nested-symbol Slider. `timeline.goto` changes the scene playhead, not a target symbol instance's local displayed frame. The inspected command reference has no dedicated setter for that target. Therefore V4 can test bone-driven global timeline scrubbing, not isolated per-layer or nested-symbol playback.

## Verified command surface

| Command | Parameters used | Role | Effect |
|---|---|---|---|
| `bone.info` | `{layer, frame}` | Read driver bone | Read-only; response includes `armatures[].layer` and `bones[].{id, angle, swing}` |
| `timeline.goto` | `{frame}` | Drive action time | Changes current scene playhead; not a nested-symbol frame setter |
| `bone.pose` | `{bone, angle, frame}` | Optional manual pose command | Writes a keyed pose; deliberately not used by this prototype |

The MCP server's `anicraft_document` tool reports the current scene, frame, layers and keyframes. It does not provide the evaluated bone angle. The prototype therefore calls `bone.info` for the input and uses document inspection only to determine whether the UI is currently at the configured source frame.

## Important interaction constraint

The scene has one global playhead. If the script reads the driver at the current playhead and then calls `timeline.goto`, the next read would sample the bone at the output frame. That creates a feedback path and can make the mapping unstable.

To avoid that, the prototype uses a fixed `--source-frame` in watch mode:

1. The artist edits the driver bone at the configured source frame.
2. The watcher samples the same source frame and waits for two stable readings.
3. It reads the driver through `bone.info`, applies the V1 mapping, then calls `timeline.goto`.
4. The artist returns the UI playhead to the source frame before making the next adjustment.

This is a quasi-interactive experiment with a manual return-to-source step. It is not seamless continuous slider control. A separate local playhead per controller/target, or a host-side evaluator, would be needed to remove this limitation.

## Prototype

File: [tools/v4_mcp_slider.py](../tools/v4_mcp_slider.py)

The script:
- starts `anicraft-cli mcp --bridge PORT` as an MCP stdio child process;
- performs MCP initialize and tools/call requests;
- discovers `bone.info`, `timeline.goto`, and `bone.pose` from the live command registry;
- uses the V1 endpoint semantics (Exclusive by default, Inclusive optional);
- writes UTC timestamps and per-call/round-trip durations to CSV;
- never writes the driver bone pose and never changes authored keyframes.

Example one-shot run:

```sh
python tools/v4_mcp_slider.py --layer 7 --bone 12 --source-frame 0
```

Watch for 20 updates (return the UI playhead to source frame 0 and adjust the bone between updates):

```sh
python tools/v4_mcp_slider.py --layer 7 --bone 12 --source-frame 0 --watch --cycles 20 --interval 0.25
```

The numeric layer and bone IDs must come from the current project; the example IDs are placeholders.

## Verification performed in this implementation

- Python syntax compilation: passed.
- Mapping and bone-info parsing tests: 6 passed.
- MCP handshake/tool-call smoke test against a local protocol simulator: passed; the simulated `0°` driver mapped to global frame 10 and produced a CSV row.

These checks validate the client and deterministic mapping logic only. They are **not** a live AniCraft desktop smoke test.

## Not yet measured

| Criterion | Current status |
|---|---|
| Live command registry discovery in user's running AniCraft | Pending |
| 10 consecutive live bone-to-frame updates | Pending |
| Median live round-trip below 500 ms | Pending |
| Actual UI screen recording | Pending; CSV transcript is supported |
| Latency under 2–5 Hz polling and real user dragging | Pending |

No real latency values are claimed. The local simulator timing is not representative of the desktop bridge.

## Recommendation

Proceed with the prototype as a **global-playhead feasibility test**, but do not call it a complete Smart Bone/Slider implementation. Run it against the user's open AniCraft session and collect at least 20 CSV samples. If live latency is acceptable, the MCP path can demonstrate the external-control concept. A separate AniCraft feature request is still warranted for an independent nested-symbol frame setter or a native controller evaluation hook if the intended behavior is Rigby2D-style action playback.

## Source audit

- `docs/mcp.md`: MCP stdio bridge, live `--bridge` workflow, and five tool names.
- `crates/cli/src/mcp.rs`: JSON-RPC initialize/tools/list/tools/call handling; `anicraft_run` dispatches through `Session::execute`; `anicraft_document` returns `Session::info()`.
- `docs/guide/reference/commands.md`: command registry reference lists `bone.info`, `bone.pose`, and `timeline.goto`; the latter accepts `{frame}`.
- Forma's `docs/ANICRAFT_SDK_AUDIT.md`: existing V1 bone-info response contract and the previously unverified nested-symbol frame target.
