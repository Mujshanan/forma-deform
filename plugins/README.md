# Historical AniCraft command adapters

These crates are retained in the workspace as development, diagnostic, and integration probes. They are not three independent production products and do not establish that the native AniCraft tool is complete.

See [Legacy Adapter Status and Retirement Plan](../docs/LEGACY_ADAPTER_RETIREMENT.md) for the replacement gates and retirement order. See the [build and smoke-test guide](INSTALL.md) for development-only adapter testing.

| Directory | Responsibility | Current disposition |
|---|---|---|
| [bone-query](bone-query/README.md) | Read-only inspection of armatures, bones, keyed poses, joint translation, springs, and shape skin weights | Keep as diagnostic tooling until equivalent native diagnostics or an explicit retained dev-tool decision exists |
| [form-deform](form-deform/README.md) | One-bone/one-pin on-demand Asset Warp command | Legacy runtime retirement candidate; do not remove until native FDM host integration reaches the parity gate |
| [smart-controller](smart-controller/README.md) | V1 Slider preview command plus read-only canonical preview foundations | Transitional adapter; retain until native Smart Controller authoring, persistence, preview, and host integration are verified |

## Architecture boundary

- `form-deform-core` and `smart-controller-core` own reusable host-independent domain logic.
- Plugin crates translate between the AniCraft ABI v1 command interface and the corresponding core.
- `anicraft-plugin-sdk` is a vendored guest-side ABI helper used by all three adapters. Retire it only after the last adapter consumer is removed.
- Smart Controller legacy-format compatibility and migration modules belong to the core and are not the same thing as the legacy WASM adapter runtime.

These are on-demand command plugins. They do not run continuously during playback or scrubbing. Persistent plugin-owned bindings, native evaluation scheduling, and symbol-frame visual application require host capabilities beyond the currently verified command-plugin path.

## Verification boundary

The repository CI workflow is configured to run `cargo test --workspace` and build each adapter for `wasm32-unknown-unknown`. A successful build is not an in-app smoke test. Each adapter's host behavior must be checked in AniCraft before its integration can be described as verified.
