# AniCraft Smart Controller capability matrix

| Capability | Status | Boundary |
|---|---|---|
| Slider normalization and action-frame mapping | V1 | Pure smart-controller-core; deterministic and independently testable; adapter logs preview only |
| Manual driver input | V1 | Explicit value overrides the host bone query |
| On-demand driver bone query | V1 | Uses the verified bone.info command |
| Deterministic Slider preview output | V1 | Logs normalized input and calculated frame; does not set the timeline |
| Deform neutral-angle pin-target calculation | V1 | Pure form-deform-core; absolute target from authored neutral data |
| Optional spring-swing contribution | V1 | Explicit opt-in; fails if the host does not provide swing |
| Asset Warp pin validation and explicit Apply | V1 | Uses verified warp.info and warp.pin_move commands; Apply is undoable |
| Weighted mesh skinning math | Core foundation | Pure form-deform-core API; not wired to live host mesh because complete geometry/weights/world poses are not exposed |
| Transform mapping calculation | V2 foundation | Pure-core calculation only; not exposed by the adapter |
| Transform constraint application | V2 | Requires a verified host write surface |
| Three-anchor ControllerBinding / BindingSet | Experimental core API; excluded from V1 | Core-only evaluator and JSON helpers; not called by the plugin runtime and not persisted in AniCraft |
| Presets and durable bindings | Not established | Current inspected plugin contract exposes no plugin-owned document metadata API |
| Timeline driving during playback | Not in V1 | Requires independent verification of external-control workflow |
| Continuous native playback evaluator | Not established | Requires a separately reviewed AniCraft host extension |
