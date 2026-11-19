# AniCraft plugins

Each plugin has its own directory and independently buildable command-plugin crate.

| Directory | Responsibility | Status |
|---|---|---|
| [bone-query](bone-query/README.md) | Read-only inspection of armatures, bones, keyed poses, joint translation, springs, and shape skin weights | Implemented; verification pending |
| [form-deform](form-deform/README.md) | Deterministic bone-driven Asset Warp pin deformation | Implemented; verification pending |
| [smart-controller](smart-controller/README.md) | General-purpose bone-input to three-anchor geometry-target mapping | Implemented; verification pending |

Shared, host-independent logic belongs in reusable crates under crates/. Plugin directories contain thin AniCraft adapters and their own manifests/entry points.

These are on-demand command plugins. They query or modify the document when invoked; they do not run continuously during playback or scrubbing. Persistent plugin-owned bindings and symbol-frame visual application require host capabilities not exposed by the audited command-plugin API.
