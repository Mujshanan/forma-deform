# FDM-04.4 — Performance and Quality Gates

## Purpose

Establish a repeatable, dependency-free performance measurement path for adaptive curve sampling while keeping correctness and CI stable across shared runners.

## Benchmark coverage

The `curve_sampling` benchmark runs two fixed workloads:

- Moderate-curvature Path2D: 128 cubic commands with moderate control-point offsets.
- High-curvature Path2D: 128 cubic commands with larger control-point offsets.

Each run performs a warm-up, measures repeated complete sampling calls, checks that the emitted sample count is deterministic, and reports elapsed nanoseconds per run and samples per second. Iterations are configurable with `--iterations`.

Run locally:

```sh
cargo bench -p form-deform-core --bench curve_sampling -- --iterations 200
```

## CI gate

CI runs the benchmark with a short 20-iteration smoke workload after the full Rust workspace test suite. This verifies that the benchmark builds, executes, and returns deterministic output for both workloads.

The benchmark deliberately does not impose a hard wall-clock pass/fail threshold. GitHub-hosted runners are shared and noisy; a timing threshold here would create flaky CI and encourage optimizing for runner variance. The reported measurements provide a baseline for controlled local or dedicated-runner comparisons.

## Correctness gates

- Full workspace unit/integration tests remain mandatory.
- Adaptive sampling enforces finite positive tolerance, bounded subdivision depth, and per-curve/total sample budgets.
- Benchmark workloads assert deterministic sample counts across repeated executions.
- FDM-04.3 numerical regression tests remain part of the workspace suite.

## Scope and limitations

- No production sampling algorithm changes.
- No new third-party benchmark dependency.
- No absolute latency promise or hardware-independent performance claim.
- This benchmark currently measures adaptive Path2D sampling; it does not yet benchmark cage mapping/evaluation end-to-end.
- No AniCraft UI/host integration, cache, scheduler, document mutation, undo/redo, apply, or bake.
- FDM-04 remains dependent on the unmerged FDM-04.2 and FDM-04.3 draft PRs.
