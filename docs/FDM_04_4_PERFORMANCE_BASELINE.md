# FDM-04.4 — Performance Baseline and Quality Gates

## Purpose

Establish a reproducible baseline for the adaptive Path2D curve sampler without making wall-clock timing a correctness requirement. The benchmark is dependency-free and uses the Rust standard library.

## Benchmark workloads

Run locally with the default 200 measured iterations per workload:

```sh
cargo bench -p form-deform-core --bench curve_sampling
```

Use a shorter smoke run:

```sh
cargo bench -p form-deform-core --bench curve_sampling -- --iterations 20
```

The harness reports elapsed nanoseconds per sampling iteration and per emitted sample for:

- One high-curvature cubic.
- Thirty-two high-curvature cubic segments.
- One hundred twenty-eight moderate-curvature cubic segments.
- Thirty-two cubic segments at large coordinate magnitudes.

Each workload performs 20 warm-up iterations and a configurable measured iteration count. Exact output equality is checked during warm-up; the timed loop measures sampling and black-boxes the result, then checks the aggregate emitted sample count. Run on an otherwise idle machine, with the same Rust toolchain and release profile when comparing results.

## Quality gates

CI executes the benchmark with a short smoke workload:

```sh
cargo bench -p form-deform-core --bench curve_sampling -- --iterations 20
```

This verifies that all four workloads execute successfully and emit the expected deterministic sample counts. It does not enforce a wall-clock threshold.

Correctness remains enforced by deterministic unit and numerical regression tests:

- Same source path and policy produce exactly equal sample sets.
- Parameters are finite, strictly increasing, and include exact endpoints.
- Numerical deviation remains within the configured tolerance for the high-curvature regression.
- Tightening tolerance does not reduce the sample count for the regression curve.
- Per-curve and total sample budgets fail closed; no partial sample set is returned.
- Large-coordinate and closed-loop cases remain finite.

No fixed nanosecond threshold is imposed in CI. Shared runners, CPU scheduling, virtualization, and toolchain changes make absolute timing thresholds unreliable. Performance regressions should be evaluated by comparing repeated benchmark runs on the same hardware and toolchain, while preserving all correctness gates.

## Scope and boundaries

- No production sampling algorithm change.
- No new third-party benchmark dependency.
- No cache, scheduler, parallel execution, Path2D mutation, host integration, apply, or bake.
- No Smart Controller or bone evaluation composition.
- The benchmark measures source curve sampling only; it does not claim to measure cage mapping or full deformation evaluation.
