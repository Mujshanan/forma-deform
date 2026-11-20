# AniCraft plugins

See the [build and smoke-test guide](INSTALL.md). Each plugin has its own directory and independently buildable command-plugin crate.

| Directory | Responsibility | Status |
|---|---|---|
| [bone-query](bone-query/README.md) | Read-only inspection of armatures, bones, keyed poses, joint translation, springs, and shape skin weights | Implemented; verification pending |
| [smart-controller](smart-controller/README.md) | Unified Slider mapping and bone-driven Asset Warp pin deformation | Implemented; verification pending |

The former standalone Form Deform command has been consolidated into Smart Controller's Deform mode. Its host-independent math remains in `form-deform-core`.

## ABI boundary

The [AniCraft Plugin ABI v1 specification](../docs/PLUGIN_ABI_V1.md) is normative for every plugin. `anicraft-plugin-sdk` is the guest-side implementation of that contract. Plugin crates contain domain logic and typed host adapters; they must not define their own ABI imports, memory conventions, or export wrappers.

Shared, host-independent logic belongs in reusable crates under crates/. Plugin directories contain thin AniCraft adapters and their own manifests/entry points.

These are on-demand command plugins. They query or modify the document when invoked; they do not run continuously during playback or scrubbing. Persistent plugin-owned bindings and symbol-frame visual application require host capabilities not exposed by the audited command-plugin API.
