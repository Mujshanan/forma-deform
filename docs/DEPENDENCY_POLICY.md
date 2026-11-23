# Rust dependency boundary policy

## Direction

Production dependencies flow in one direction:

```text
AniCraft host / plugin adapter
          |
          v
   host-facing adapter ------> anicraft-plugin-sdk (ABI v1)
          |
          v
     domain core
```

The two domain cores are independent, host-neutral Rust libraries:

- `form-deform-core` owns Form Deform data, validation, geometry, bindings, and evaluation contracts.
- `smart-controller-core` owns Smart Controller models, validation, mapping, evaluation, and explicit compatibility/migration behavior.

Neither core may depend on the AniCraft plugin SDK, import its Rust crate, invoke its ABI macros, or depend on host command/ABI types. Host-neutral contracts in a core are allowed when they describe stable domain inputs and outputs without importing host implementation details.

## Adapter boundary

The current command adapters remain transitional consumers of the vendored ABI v1 SDK:

- `plugins/form-deform`
- `plugins/smart-controller`
- `plugins/bone-query`

They may translate host requests and responses to/from core types. AniCraft-specific identifiers, command names, serialization quirks, and SDK macros belong in adapters, not in either core.

The adapters and SDK are intentionally retained during this isolation step. This policy does not authorize deleting plugin crates, deleting the SDK, changing native host contracts, or combining Smart Controller and Form Deform evaluation.

## Enforcement

Run:

```sh
bash scripts/check-core-dependencies.sh
```

The check verifies that:

1. Core manifests and Rust source do not reference the SDK or its ABI macros.
2. Each core's normal dependency tree excludes the SDK.
3. Each transitional adapter still declares the SDK dependency.
4. Each core passes its native Cargo tests independently, without requiring a WASM target.

CI runs this check before the full workspace test and existing adapter WASM builds. The workspace-level WASM checks remain in place until a separately reviewed native integration replacement is implemented and verified.

## SDK retirement conditions

The SDK may leave the production workspace only after all of the following are demonstrated:

- Every production adapter consumer has been replaced or deliberately retired.
- Native AniCraft integration covers the required adapter behavior.
- Equivalent tests cover host translation, preview/undo behavior, and diagnostics.
- CI no longer relies on the SDK or retired WASM artifacts.
- The removal is reviewed as a separate, explicit change.

Smart Controller legacy compatibility and migration modules are core behavior; they are not SDK adapter code and must not be removed as part of SDK retirement.
