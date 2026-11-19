# AniCraft plugins

Each plugin has its own directory and independently buildable command-plugin crate where applicable.

| Directory | Responsibility | Status |
|---|---|---|
| [bone-query](bone-query/README.md) | Read-only inspection of AniCraft armatures, bones, poses, limits, springs, and optional shape-point weights | First implementation |
| [form-deform](form-deform/README.md) | Shape deformation driven by bone changes | Planned |
| [smart-controller](smart-controller/README.md) | General-purpose controller bindings and coordination | Planned |

Shared, host-independent logic belongs in reusable crates under `crates/`. Plugin directories contain thin AniCraft adapters and their own manifests/entry points.

These are command plugins. They query or modify the document when invoked; they do not run continuously during playback or scrubbing.
