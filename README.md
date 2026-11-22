# Forma Deform

Forma Deform is the reusable controller and deformation engine project for AniCraft. This repository contains host-independent Rust libraries, AniCraft command-plug-in adapters, and a small legacy TypeScript geometry/timeline experiment.

**Product direction:** prove controller behavior in a small independent library, validate the host boundary through AniCraft command plug-ins, then deliver the finished Smart Controller as a first-party native AniCraft editor tool. The command plug-in is an integration prototype and compatibility path—not the final authoring UI.

## Current repository contents

| Component | Responsibility | Current boundary |
|---|---|---|
| Smart Controller core | Host-independent controller model, validation, mapping and evaluation | Rust library; model consolidation and verification remain part of development |
| Smart Controller plug-in | On-demand AniCraft command adapter and preview path | Prototype/compatibility stage; not a playback evaluator |
| Form Deform core + plug-in | Bone-angle-driven Asset Warp pin deformation | Independent engine and command adapter |
| Bone Query plug-in | Read-only armature, bone, pose and skin-weight inspection | On-demand diagnostic command |
| AniCraft plug-in SDK | Host command/document bridge used by the plug-ins | Pinned local integration boundary |
| TypeScript demo | Original rectangle/timeline geometry experiment | Separate experiment; not the Smart Controller product UI |

## Target architecture

- The **Smart Controller core** owns controller semantics and deterministic evaluation. It must not depend on AniCraft, its UI, MCP transport, WASM imports, or document internals.
- **AniCraft** owns durable/versioned controller definitions, host identity resolution, registered commands, undo, native tool UI, and any frame-evaluation integration.
- The **native Smart Controller tool** follows AniCraft's existing Tools and Properties conventions.
- The native UI and MCP use the same AniCraft command registry and command contracts. Reuse AniCraft's existing generic MCP tools; do not create a separate Smart Controller MCP server or protocol.
- Smart Controller and Form Deform remain independent. No automatic output composition or data feed between them is part of this plan.

## Documentation authority

There are exactly two canonical project documents:

- **[DEVELOPMENT.md](DEVELOPMENT.md)** — the single source of truth for architecture, scope, current status, roadmap, phases, decisions, and development/verification process.
- **[README.md](README.md)** — the concise project overview and entry point. It links to DEVELOPMENT.md and does not maintain a second roadmap.

Do not add another README or development-plan document in a subdirectory. Component-specific details belong in DEVELOPMENT.md or in focused operational/reference documents that do not duplicate the roadmap. Historical audits are evidence snapshots, not competing project plans.

## Build and verification

Start with the canonical [development guide](DEVELOPMENT.md). Plug-in build/install and in-app smoke-test commands are in [plugins/INSTALL.md](plugins/INSTALL.md).

The native AniCraft tool is **not implemented by this repository update**. It requires separately scoped first-party changes in [Mujshanan/AniCraft](https://github.com/Mujshanan/anicraft). No playback, scrubbing, durable controller persistence, or live MCP streaming capability is claimed.
