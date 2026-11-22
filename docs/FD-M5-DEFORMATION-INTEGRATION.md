# FD-M5 — Deformation Integration

**Status:** first command-adapter slice implemented; native host verification remains required.
**Depends on:** canonical controller mapping, read-only target preview, and the M4 persistence payload contract.
**Ownership:** Forma Deform resolves canonical output to a typed target command plan. AniCraft owns Asset Warp mutation, current-playhead keying, document revision, and undo/redo.

## Implemented: one-target current-playhead apply

The Smart Controller command exposes a native typed `target_action` choice:
- `preview` (default): read-only.
- `apply_current_frame`: explicit write at AniCraft's current playhead.

The apply path:
1. Runs canonical evaluation and the existing composed target preview.
2. Rejects unresolved targets and requires exactly one output.
3. Requires host verification of the exact geometry element and pin.
4. Re-queries `warp.info` immediately before mutation.
5. Converts the finite Vector2 output to the documented numeric `warp.pin_move` parameters.
6. Issues exactly one `warp.pin_move` command. The host command creates the pin key at the current frame and participates in AniCraft undo.

This path is non-accumulating: it applies the absolute evaluated stage-space position, not a delta from the current pin position.

## Target adapter contract

- Element and pin IDs must parse as positive numeric AniCraft identities.
- Pin membership is verified against the exact element's `warp.info` response; array order is never identity.
- Only `GeometryDeform` targets with Vector2 values can be written.
- SymbolFrame, FrameRange and Property outputs remain calculation/preview-only until a matching host setter is source-verified.
- A failed identity check or invalid value fails before mutation.

## Atomicity boundary

The first slice performs one host mutation command per invocation. This allows AniCraft's own command/undo boundary to remain authoritative and avoids a plugin-side pseudo-transaction. The plugin refuses multi-target apply rather than issuing a sequence that could partially succeed.

Timeline-range bake and multi-target atomic apply are deliberately deferred. The inspected ABI does not provide a host transaction/batch API; those features require a first-party AniCraft batch command that validates all targets and commits one undo transaction.

## Acceptance

- [x] Explicit opt-in current-playhead apply action; preview remains default.
- [x] Stable numeric element/pin identity checks before mutation.
- [x] Pure command-payload validation and unit tests.
- [x] Single host mutation per apply, delegated to AniCraft undo.
- [ ] In-app test: apply at playhead, verify keyed pin value, undo, redo.
- [ ] AniCraft batch command design and implementation before timeline-range or multi-target bake.
