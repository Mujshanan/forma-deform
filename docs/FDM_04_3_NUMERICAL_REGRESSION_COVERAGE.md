# FDM-04.3 — Numerical Regression Coverage

## Scope

This slice adds numerical regression tests around adaptive Bézier sampling. It does not change the sampling algorithm or the FDM-04.2 evaluation contract.

## Added coverage

- High-curvature cubic sampling with a measured midpoint-to-chord deviation check against the requested tolerance.
- Monotonic refinement: tightening the sampling tolerance must not reduce the emitted sample count for the same curve.
- Strictly increasing finite Bézier parameters and exact endpoint parameters.
- Large-magnitude coordinates and closed-loop cubic segments, ensuring generated coordinates and parameters remain finite.

## Existing coverage retained

The curve-sampling suite also covers straight and curved quadratic segments, cubic endpoints, command/subpath identity, line/close handling, deterministic output, zero-length chords, invalid policies, and depth/per-curve/total sample-budget failures. Existing mapping tests cover the shared point locator and outside-cage clamping.

## Verification target

Run the full Rust workspace tests, formatting check, adaptive-sampling benchmark smoke run, and all three release WASM builds. The former standalone TypeScript/Bun rectangle demo and its checks have been retired; they are not part of the current verification contract. Any numerical failure should be fixed at the narrowest responsible layer; do not weaken tolerances or discard failing cases to make the suite pass.

## Boundaries

- No production algorithm changes unless a reproducible defect is exposed by the tests.
- No Path2D flattening or mutation.
- No AniCraft UI/host integration, cache, scheduler, document mutation, apply, or bake.
- FDM-04.4 remains the separate performance and quality-gate step.
