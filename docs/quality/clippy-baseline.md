# Clippy Baseline — Report Only

**Date:** 2026-11-23  
**Workflow:** [Clippy report run 45122340562](https://github.com/Mujshanan/forma-deform/actions/runs/45122340562)  
**Artifact:** `clippy-report` (artifact ID `14793379573`; retained for 30 days)  
**Toolchain:** Rust 1.100.0 stable  
**Command:** `cargo clippy --workspace --all-targets --message-format=short`

## Result

Clippy completed with exit code **0**. The captured output contains 10 distinct warning locations:

| Category | Count | Location(s) |
|---|---:|---|
| Collapsible `if` | 2 | `plugins/bone-query/src/lib.rs:61`; `crates/smart-controller-core/src/migration.rs:368` |
| Derivable implementation | 1 | `crates/smart-controller-core/src/endpoint.rs:13` |
| Field assignment after `Default::default()` | 1 | `crates/smart-controller-core/src/evaluator.rs:248` |
| Type complexity | 2 | `crates/smart-controller-core/src/evaluator.rs:498,550` |
| `BTreeMap` contains-then-insert | 1 | `plugins/smart-controller/src/input_provider.rs:81` |
| Boolean literal assertion | 1 | `crates/smart-controller-core/tests/migration_tests.rs:41` |
| Cloned reference used to build a slice | 2 | `plugins/smart-controller/src/input_provider.rs:172,229` |

The output includes duplicate summary lines for library/test targets; the table counts distinct source locations, not repeated Cargo summary lines.

## Policy

- This is a baseline report, not a failure gate.
- Do not apply `cargo clippy --fix` wholesale.
- Review each warning for readability, compatibility, performance, and behavior before remediation.
- In particular, type-complexity warnings may justify named types, while test assertions and small adapter idioms should be changed only when the result is clearer.
- Rustqual and other heuristic analyzer scores remain report-only and are not CI gates.

## Next candidates

1. Review the two evaluator type-complexity sites for named domain types.
2. Review the `input_provider.rs` map entry and cloned-reference suggestions.
3. Review the two collapsible-if suggestions and derivable default separately.
4. Decide whether the boolean assertion warning improves test clarity if rewritten.
5. Re-run the report after any independently reviewed cleanup to measure the actual change.

Coverage, rustdoc warnings, and dependency/license auditing are not included in this baseline.
