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
| Continuous native playback evaluator | Not established | Requires a separately reviewed AniCraft host extension |
| Durable binding persistence in the document | Not established | Current inspected plugin contract exposes no plugin-owned document metadata API |


## M3B host dependencies (verified against AniCraft main, 2026-11-22)

| Capability | Current status | Required host work |
|---|---|---|
| Native dock/workspace framework | Exists in AniCraft | Add a supported third-party panel contribution boundary; no public plugin panel API currently |
| Command parameter dialogs | Supported | Not sufficient for a persistent inspector/controller editor |
| Stage renderer | Exists in AniCraft | Add safe stage overlay model and rendering lifecycle |
| Document-owned plugin bindings | Not exposed by current plugin ABI | Versioned document data + CRUD commands + migration/undo tests |
| On-demand bone/warp queries | Supported through commands | Reuse for authoring and validation |
| Continuous bone-driven deformation evaluation | Not exposed by command/frame-script plugin API | Deterministic evaluator integrated into canonical frame path |
| Preview/apply | Command preview and atomic command runs exist | Add read-only evaluated binding preview and one-step apply/bake semantics |

See [M3B_HOST_REQUIREMENTS.md](M3B_HOST_REQUIREMENTS.md) for the proposed schema, extension contracts, sequencing, and acceptance tests.
