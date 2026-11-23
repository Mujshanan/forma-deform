# FDM-04.2 — Adaptive Curve-Sample Evaluation

## Purpose

Evaluate the adaptive samples derived from an immutable Path2D source against a current certified cage pose. The result is transient tessellation geometry for preview/rendering consumers; it does not replace or mutate authored Path2D.

## Evaluation contract

The public entry point is `evaluate_curve_samples`. It accepts:

- An FDM-03 `ValidatedBindingRecord`, which supplies the authoritative source/topology/outside-policy stamp.
- The exact immutable `GeometrySnapshot` used to create the binding.
- The stable `DeformationTopology`.
- A certified rest cage and a certified current cage with matching topology identity/revision.
- A previously mapped `MappedCurveSampleSet`.

The evaluator validates source freshness and both cage topology identities. It regenerates adaptive samples from the source using the mapped set's recorded sampling policy, remaps them against the supplied certified rest cage using the binding's stamped outside-cage policy, and requires exact agreement with the supplied mapped set. This rejects stale, reordered, tampered, or mismatched derived mappings rather than silently accepting or repairing them.

Each mapped, clamped, or extrapolated sample is reconstructed from its stored face coordinates against the current cage. Unbound samples retain their original source-space position. Extrapolated coordinates remain the full face-coordinate solution; the boundary offset is not added a second time.

## Output

`EvaluatedCurveSampleSet` preserves the command index, subpath index, per-command sample index, and Bézier parameter (t), and returns:

- Evaluated transient sample positions.
- Sampling policy/version.
- Source snapshot stamp.
- Rest and current cage geometry fingerprints.
- Counts for mapped, clamped, extrapolated, and unbound samples.

The sample ordering and parameter values are preserved. The evaluator does not rebuild, flatten, rewrite, or persist Path2D.

## Determinism and safety

- Evaluation always starts from the source-derived mapped sample set and current cage; it never consumes a prior evaluated output.
- Identical validated inputs produce identical output.
- Stale source revisions and topology identity/revision mismatches fail closed.
- Mapped samples are checked against freshly regenerated source samples and deterministic rest-pose mappings.
- Face/control-point order, coordinate kind, subface index, barycentric weight sums, and finite output are validated.
- Sampling/mapping budget and outside-policy failures propagate as errors; no partial result is returned.

## Boundaries

- No AniCraft UI or host adapter changes.
- No cache, scheduler, document mutation, undo/redo, apply, or bake.
- No Smart Controller/bone evaluation or composition.
- Authored Path2D remains authoritative. Adaptive samples are derived transient data only.
- This slice does not complete FDM-04; numerical regression/performance gates remain for FDM-04.3 and FDM-04.4.
