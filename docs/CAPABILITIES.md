# Smart Controller capability matrix

| Capability | Status | Boundary |
|---|---|---|
| Host-independent Slider normalization and frame mapping | V1 | Native Rust core; deterministic and independently testable |
| Manual driver input | V1 | Explicit value overrides the host bone query |
| On-demand driver bone query | V1 | Uses the verified bone.info command |
| Deterministic preview output | V1 | Logs normalized input and calculated frame |
| Transform mapping calculation | V2 foundation | Pure-core calculation only; not exposed by the V1 adapter |
| Transform constraint application | V2 | Requires a verified host write surface |
| Presets | V3 | Persistence contract remains to be designed |
| MCP command discovery | V4 prototype | anicraft_commands discovers commands from the running host |
| MCP driver-bone read | V4 prototype | bone.info reads a keyed joint angle at an explicit source frame |
| MCP global playhead driving | V4 prototype | timeline.goto changes the scene playhead; it is not a nested-symbol frame setter |
| Interactive watch mode | V4 prototype | Polls only at a fixed source frame; artist returns to source frame between adjustments |
| Live latency and 10-cycle acceptance | Pending | Requires testing against a running AniCraft desktop bridge |
| Timeline driving during playback | Not established | MCP polling is not frame-synchronized playback |
| Independent nested-symbol frame control | Not established | Requires a dedicated host command or native host extension |
| Continuous native playback evaluator | Not established | Requires a separately reviewed AniCraft host extension |
| Durable binding persistence in the document | Not established | Current inspected plugin contract exposes no plugin-owned document metadata API |
