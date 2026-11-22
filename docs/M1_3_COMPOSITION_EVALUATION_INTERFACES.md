# M1.3.1 — Composition Evaluation Interfaces and Execution Contracts

**Status:** interface contracts defined; planner and runtime execution are not implemented.
**Depends on:** M1 evaluation lifecycle contracts and the FDM persistence/composition schema.

## Purpose

Define the smallest host-independent boundary required to execute an explicitly authored composition graph. This slice does not execute operators, schedule threads, cache results, mutate AniCraft geometry, or change Smart Controller/pin-deformation ownership.

## Core contracts

- `GeometryValue` carries a `Path2D` plus its explicit `GeometryValueKind`. Absolute geometry and displacement values are never silently interchanged.
- `NodeValue` makes output cardinality explicit: one geometry or an ordered collection of parallel branch values.
- `BranchValue` retains its producing node ID. Branch identity is not inferred from vector position or creation order.
- `OperatorContext` exposes the validated immutable `EvaluationRequest`, including explicit time and dependency stamps.
- `DeformerOperator` is a pure, deterministic operator interface identified by deformation kind and a non-zero algorithm version.
- `OperatorRegistry` resolves implementations without coupling the core to a host plugin loader.
- Stable operator failure categories distinguish unsupported definitions, type/space mismatches, invalid parameters, topology mismatch, non-finite output, resource limits and arithmetic failures.

## Execution semantics to preserve

1. Independent evaluation remains the default. A graph is executed only when the request explicitly names its graph stamp and the graph has passed persistence validation.
2. Sequential stages consume exactly one geometry value; each stage receives the previous stage's output.
3. Parallel branches receive the same immutable input and produce independent branch values in authored branch order. Parallel execution must not imply addition, precedence or blending.
4. Weighted blending is a separate explicit operation. It consumes compatible branch values and authored weights; it must reject incompatible topology/coordinate semantics rather than guessing how to align paths.
5. Graph output order is the persisted output-node order. No ordering may be derived from hash-map iteration, thread completion, or node creation time.
6. Identical authored definitions, input snapshots, explicit request dependencies, graph revision and operator algorithm versions must produce identical outputs and fingerprints.
7. The evaluator is read-only. AniCraft remains responsible for scheduling, current-snapshot checks, caching, application, undo and bake transactions.
8. CCC-01.5 remains unchanged: Smart Controller evaluation and pin deformation remain independent. Smart Controller output is not implicitly admitted as an FDM graph source.

## Important schema-to-runtime gap

The current graph schema represents `Parallel` as a node with a single geometry signature, while the runtime interface correctly represents its result as an ordered branch collection. Before M1.3.2 execution planning, graph validation must be tightened so that:

- a branch collection cannot be consumed as an ordinary single geometry;
- sequential stages accept only single-geometry nodes;
- weighted blend consumes explicit compatible geometry-producing branches;
- parallel branch IDs and weighted input IDs have deterministic, validated ordering;
- graph outputs explicitly support the intended output cardinality.

This is a deliberate acceptance gate, not permission to reinterpret existing persisted graphs silently. If schema changes are required, they must be versioned and migrated explicitly.

## M1.3.1 acceptance

- Core exports the operator, registry, context, geometry-value and node-value contracts.
- Operator algorithm identity/version is validated before use.
- Branch values require stable producing-node IDs.
- No host APIs, live scene references, scheduler, cache or mutation APIs are introduced.
- Tests cover value-kind retention, branch identity, output cardinality and invalid operator metadata.

## Next slice

M1.3.2 is the deterministic execution planner and graph/runtime cardinality validation. It must resolve the schema-to-runtime gap above before any operator is executed.
