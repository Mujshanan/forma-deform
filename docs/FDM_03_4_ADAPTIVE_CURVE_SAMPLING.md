# FDM-03.4 — Adaptive Curve Sampling and Cage Mapping

## Status

The first implementation is committed but **unverified**. Rust tests, formatting, compilation, and CI have not been run. This step must not be treated as complete until verification passes.

## Scope

FDM-03.4 derives deterministic temporary samples for authored quadratic and cubic Path2D commands, then maps those samples through the existing FDM-03.2 point locator and FDM-03.3 outside-cage policies.

The canonical Path2D command stream remains authoritative and unchanged. Sampling does not replace Bézier commands, mutate handles, or create persistent source-point identities. Sample ordinals and parameters are valid only for the exact stamped source snapshot.

LineTo and Close commands are not subdivided. MoveTo establishes the current point and subpath identity; each quadratic/cubic command is sampled from its original current point to its authored endpoint.

## Public API

- `CurveSamplingPolicy`: maximum source-space deviation, maximum recursive depth, maximum samples per curve, and maximum total samples.
- `CURVE_SAMPLING_POLICY_VERSION`: explicit algorithm version; each `CurveSampleSet` retains both this version and the exact policy values used.
- `sample_path_curves(&Path2D, CurveSamplingPolicy)`: returns ordered `CurveSegmentSamples` for quadratic/cubic commands.
- `map_curve_samples(&CurveSampleSet, &ValidatedCageGeometry, OutsideCagePolicy)`: maps each sample using the shared deterministic point locator and explicit outside-cage resolver, returning a `MappedCurveSampleSet` that retains the sampling-policy version and values.
- `CurveSampleMapping`: transient `Mapped`, `Clamped`, `Extrapolated`, or `Unbound` result, including the owning face, stable face control IDs, and face coordinates where applicable.

A sample is identified by its source command index, subpath ordinal, per-command sample ordinal, and parameter (t). This is derived metadata, not a host-persistent point ID. The host must associate the results with the source snapshot fingerprint and binding stamp. Any derived cache key must also include the sampling-policy version and exact policy values; changing the tolerance or any resource limit changes the derived sample set.

## Error bound

For a quadratic or cubic Bézier segment, the sampler measures each interior control point's Euclidean distance to the closed endpoint chord segment.

The distance-to-segment function is convex. A Bézier curve lies in the convex hull of its control polygon, so if every interior control point is within tolerance of the chord segment, every point on that curve segment is also within that tolerance of the chord. This is a one-sided curve-to-chord deviation bound; it is not a claim that the chord is within the same distance of the curve.

If the bound is not met, the curve is split at (t=0.5) using de Casteljau midpoint subdivision. Left children are processed before right children. Original-command parameters are propagated by midpoint subdivision, producing deterministic, increasing (t) values from 0 through 1.

The tolerance is expressed in the source snapshot coordinate-space linear units. The coordinate-space descriptor is not inferred or transformed by this module.

## Resource and failure policy

Default policy:

- Maximum deviation: 0.25 source-space units.
- Maximum recursive subdivision depth: 16.
- Maximum samples per curve: 65,537, including both endpoints.
- Maximum total samples: 1,000,000, including endpoints.

Hard limits:

- Maximum supported subdivision depth: 32.
- Maximum total sample count: 1,000,000.

All limits are validated before sampling. If the requested deviation cannot be met before the depth or sample budget is exhausted, the operation returns a typed error. It never returns a partial sample set while implying completion. Arithmetic overflow and invalid command sequencing also fail closed.

## Cage mapping

Every sample is located independently using FDM-03.2. An inside sample retains the located face index, authored stable control-point IDs, and triangle barycentric or quad bilinear coordinates.

Only an explicit `OutsideCage` result is passed to FDM-03.3. The selected outside policy produces a clamped boundary mapping, bounded extrapolation mapping, or unbound sample. Point-location errors and outside-policy errors remain errors; they are never converted into unbound samples.

Mapping is transactional at the API boundary: if any sample fails, no partial mapped vector is returned. Sample mapping is transient and is not yet serialized by FDM-03.4.

## Numerical implementation

- Midpoint subdivision uses half-sums to avoid overflow in direct coordinate addition.
- Point-to-segment distance uses scale normalization before projection and distance evaluation.
- Every emitted (t) and sample point must be finite; (t) must remain in [0, 1].
- Zero-length chords are handled as point-distance tests.
- No floating-point sample coordinates are used as persistent identity.
- The original Path2D remains unchanged.

## Tests authored

- Straight quadratic endpoint-only sampling.
- Deterministic curved quadratic subdivision.
- Cubic endpoint and parameter preservation.
- Line and Close exclusion.
- Multi-subpath ordinal preservation.
- Invalid policy, depth exhaustion, per-curve budget exhaustion, and total budget exhaustion.
- Zero-length chord handling.
- Source Path2D immutability.
- Adaptive sample mapping through the shared point locator and clamping policy.

## Scope boundary

This step does not persist sample mappings, modify the binding schema, rebind stale source snapshots, deform Bézier handles, evaluate cage motion, or integrate with AniCraft scheduling/UI. Those responsibilities remain in FDM-03.5 and later host/evaluation work.

## Files

- `crates/form-deform-core/src/curve_sampling.rs`
- `crates/form-deform-core/src/lib.rs`
- `docs/FDM_03_4_ADAPTIVE_CURVE_SAMPLING.md`
