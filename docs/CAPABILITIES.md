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
| Timeline driving during playback | Not in V1; proposed for V4 via MCP | Requires independent verification of external-control workflow |
| Continuous native playback evaluator | Blocked / not exposed | Current command plug-ins are on-demand; frame scripts are keyframe-entry callbacks without bone/warp query access. See [FD-M6 contract](FD-M6-HOST-EVALUATION-INTEGRATION.md). |
| Durable binding persistence in the document | Not established | Current inspected plugin contract exposes no plugin-owned document metadata API |
