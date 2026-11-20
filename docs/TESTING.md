# Test strategy

The repository uses Rust as its authoritative implementation for document geometry, timelines, and deformation.

## Continuous integration workflows

- **Platform** — runs `cargo test --workspace` and `cargo build --workspace` on Ubuntu and Windows.
- **WebAssembly** — installs `wasm32-unknown-unknown`, builds the workspace for that target, and builds both plugin modules in release mode.
- **Lint** — runs `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- **Release Build** — on version tags (`v*`), builds optimized workspace artifacts on Ubuntu and Windows and uploads platform-specific artifacts.

All workflows use `Swatinem/rust-cache` to reuse Cargo registry, Git, and build caches. Cache keys are scoped by workflow and platform/target to avoid cross-target contamination.

## Local verification

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --workspace --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p anicraft-bone-query-plugin
cargo build --release --target wasm32-unknown-unknown -p anicraft-smart-controller-plugin
```

## JavaScript commands

The former TypeScript prototype and its `package.json` were removed when document geometry and timeline behavior moved into `forma-document-core`. There is no current npm project or npm test/build script to invoke; adding `npm test` or `npm run build` would target a deleted application. If a JavaScript/WASM frontend is reintroduced, it should have its own package manifest and dedicated web workflow.

## Rust core tests

- `forma-document-core` covers rectangle creation/editing/translation/hit testing, timeline endpoint holds and interpolation, keyframe replacement/removal, JSON round trips, and validation.
- `form-deform-core/tests/deformation_properties.rs` exercises repeatability, identity transforms, finite weighted results, extreme finite inputs, malformed weights, missing bones, arithmetic overflow, repeated-endpoint loop closure, and stable pin indices.
- `form-deform-core` unit tests cover opt-in metrics, structured error codes, interpolation mode, and degenerate-triangle/zero-length-bone warnings.
- `smart-controller-core` retains endpoint, reversed-range, and validation tests.

## Performance and observability

Criterion benchmarks cover 100, 1,000, 10,000, and 100,000 vertices with 64 available bones and two influences per vertex. Record benchmark baselines on a stable runner before setting regression thresholds. Do not make noisy wall-clock thresholds a mandatory general CI gate.

Deformation metrics are opt-in. The current algorithm is linear-blend skinning, not an iterative solver; therefore iteration count and convergence are represented as absent rather than fabricated. Timing is measured by the native core where supported. WASM hosts should time the call at the adapter boundary. Degenerate-triangle and zero-length-bone warnings require explicit topology and segment inputs.

## Plugin and ABI tests

The SDK tests stable status codes, malformed JSON, invalid pointer/length combinations, oversized parameter buffers, allocation failure sentinels, and pointer/length packing. Plugin integration tests exercise the Smart Controller-to-Deform-core composition.

CI builds both plugin modules for `wasm32-unknown-unknown`. This verifies compilation and target compatibility, but is not a substitute for executing a module inside AniCraft's actual wasmi runtime. Host-level WASM conformance tests belong with the host runtime repository.
