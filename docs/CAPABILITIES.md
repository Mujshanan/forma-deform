# Smart Controller capability matrix

This matrix distinguishes pure-core capabilities, adapter preview paths, and capabilities that still require native AniCraft host integration.

| Capability | Status | Boundary |
|---|---|---|
| Host-independent Slider normalization and frame mapping | Implemented in core | Deterministic Rust logic; independently testable |
| Manual driver input | V1 adapter | Explicit value overrides the host bone query |
| On-demand driver bone query | V1 adapter | Uses the verified `bone.info` command |
| Deterministic Slider preview output | V1 adapter | Logs normalized input and calculated action frame; no target write |
| Canonical document preview evaluation | Adapter foundation | Read-only helper path validates a canonical document and evaluates it through an injected mapper |
| Geometry target capability inspection | Adapter foundation | Uses read-only `warp.info` to check element/pin identity; does not call a setter |
| Transform mapping calculation | Core foundation | Pure-core calculation only; not exposed by the active V1 command |
| Transform constraint application | Not established | Requires a verified host write surface and explicit semantics |
| Presets and durable controller bindings | Not established | Requires a versioned, host-owned document persistence contract |
| Native authoring and binding inspector | Not established | Requires the native AniCraft tool and connected document commands |
| Continuous native playback evaluator | Not established | Requires a separately reviewed AniCraft evaluator integration |
| Symbol-frame visual application | Not established | No verified host command currently applies the calculated symbol frame |
| Smart Controller + Form Deform output composition | Explicitly excluded | Keep the two evaluation paths independent unless a future semantic design is approved |

## Adapter retirement implication

The canonical preview helpers are valuable integration foundations, but they do not by themselves replace the native authoring, persistence, target-application, undo, or playback responsibilities. See [Legacy Adapter Status and Retirement Plan](LEGACY_ADAPTER_RETIREMENT.md).
