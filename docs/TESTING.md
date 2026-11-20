# Test strategy

This repository uses Rust as the authoritative implementation for document geometry, timelines, and deformation.

## Rust core tests

- `forma-document-core` covers rectangle creation/editing/translation/hit testing, timeline endpoint holds and interpolation, keyframe replacement/removal, JSON round trips, and validation.
- `form-deform-core/tests/deformation_properties.rs` exercises repeatability, identity transforms, finite weighted results, extreme finite inputs, malformed weights, missing bones, arithmetic overflow, repeated-endpoint loop closure, and stable pin indices.
- `form-deform-core` unit tests cover opt-in metrics, structured error codes, interpolation mode, and degenerate-triangle/zero-length-bone warnings.
- `smart-controller-core` retains endpoint, reversed-range, and validation tests.

Run:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo check --workspace --benches
cargo bench -p form-deform-core --bench deform_mesh
```

## Performance and observability

Criterion benchmarks cover 100, 1,000, 10,000, and 100,000 vertices with 64 available bones and two influences per vertex. Record benchmark baselines on a stable runner before setting regression thresholds. Do not make noisy wall-clock thresholds a mandatory general CI gate.

Deformation metrics are opt-in. The current algorithm is linear-blend skinning, not an iterative solver; therefore iteration count and convergence are represented as absent rather than fabricated. Timing is measured by the native core where supported. WASM hosts should time the call at the adapter boundary. Degenerate-triangle and zero-length-bone warnings require explicit topology and segment inputs.

## Plugin and ABI tests

The SDK tests stable status codes, malformed JSON, invalid pointer/length combinations, oversized parameter buffers, allocation failure sentinels, and pointer/length packing. Plugin integration tests exercise the Smart Controller-to-Deform-core composition.

CI builds both plugin modules for `wasm32-unknown-unknown`. This verifies compilation and target compatibility, but is not a substitute for executing a module inside AniCraft's actual wasmi runtime. Host-level WASM conformance tests belong with the host runtime repository.
