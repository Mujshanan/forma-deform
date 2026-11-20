# Test strategy

This repository contains two different geometry domains: the TypeScript rectangle/timeline demo and the Rust AniCraft deformation/controller engines. Tests must not imply that these are interchangeable implementations.

## Rust core tests

- Keep deterministic example tests beside the algorithm and broader invariants in integration tests under each core crate's `tests/` directory.
- `form-deform-core/tests/deformation_properties.rs` uses Proptest to exercise repeatability, identity transforms, finite bounded weighted results, extreme finite inputs, malformed weights, missing bones, arithmetic overflow, repeated-endpoint loop closure, and stable pin indices.
- The loop-closure case currently models a closed contour by repeating its first vertex at the end. The mesh model has no explicit edge/topology representation yet; this test does not claim to validate arbitrary polygon topology or a constraint solver.
- `smart-controller-core` retains endpoint, reversed-range, and validation tests. Add generated properties there when the mapping invariants are expanded.

Run:

```sh
cargo test --workspace
cargo check --workspace --benches
cargo bench -p form-deform-core --bench deform_mesh
```

## Performance

Criterion benchmarks cover 100, 1,000, 10,000, and 100,000 vertices with 64 available bones and two influences per vertex. Record benchmark baselines on a stable runner before setting regression thresholds. Do not make noisy wall-clock thresholds a mandatory general CI gate.

## Plugin and ABI tests

The SDK tests stable status codes, malformed JSON, invalid pointer/length combinations, oversized parameter buffers, allocation failure sentinels, and pointer/length packing. Plugin integration tests exercise the Smart Controller-to-Deform-core composition.

CI builds both plugin modules for `wasm32-unknown-unknown`. This verifies compilation and target compatibility, but is not a substitute for executing a module inside AniCraft's actual wasmi runtime. Host-level WASM conformance tests belong with the host runtime repository, where the runtime and its import implementation live.

## TypeScript and Rust boundary

The current TypeScript tests cover the rectangle demo's document/geometry/timeline behavior. They do not implement the same deformation algorithm as `form-deform-core`, so cross-frontend numerical parity tests would currently compare unrelated behavior. If a TypeScript deformation implementation is introduced, define a shared language-neutral JSON fixture format and run the same golden cases from Bun and Rust; compare numeric values with explicit tolerances, not formatted floating-point strings.

## Not yet implemented

The current deformation core has no inverse-deformation operation, stiffness parameter, iterative constraint solver, or explicit mesh edge topology. Tests for inverse/stiffness bounds should be added alongside those APIs rather than inventing semantics in test code.
