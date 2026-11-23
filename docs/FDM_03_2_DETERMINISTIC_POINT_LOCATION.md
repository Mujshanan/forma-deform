# FDM-03.2 — Deterministic Point Location

## Implementation status

Point-location implementation and focused unit tests have been authored. **No tests, formatting, compilation, or CI checks have been run**. This step is therefore committed as unverified implementation work, not as verified/complete.

## Public API

- `point_location::locate_point(Point2, &ValidatedCageGeometry)`
- `PointLocationOutcome::Located(LocatedFace)` or `OutsideCage`
- `LocatedFace` carries the revision-bound face index, explicit v1 subface index zero, stable control-point IDs in authored face order, and tagged `FaceCoordinates`.
- `PointLocationError` distinguishes degenerate/singular numerical cases, non-finite arithmetic, non-adjacent candidate ambiguity, and invalid shared-boundary ownership.

The input geometry must be constructed through FDM-03.1's `ValidatedCageGeometry`, which requires a current FDM-02.6 certificate. The locator cannot accept raw topology/control-state pairs or a caller-supplied validation flag.

## Triangle location

Triangle coordinates are computed with signed cross products in f64. Degenerate determinants are rejected using a scale-relative floating-point threshold. Candidate weights must be inside the unit simplex within the centralized numerical tolerance; only tolerance-sized boundary drift is clamped, then weights are normalized and the reconstructed point is checked against a scale-aware spatial tolerance.

## Quadrilateral inverse

Quads use the authored vertex order `P00, P10, P11, P01` and the bilinear map `P(u,v)=a+bu+cv+duv`. The inverse eliminates one parameter to form a quadratic, solves it with a cancellation-resistant quadratic formula, derives the other parameter by projection, and validates domain membership and geometric reconstruction.

- No diagonal split or triangle fallback exists.
- Singular/ill-conditioned inverse systems and multiple accepted solutions return typed errors.
- No in-domain solution means that face is not a candidate.
- The discriminant tolerance is relative to coefficient magnitude, not an arbitrary unit-scale floor.

## Deterministic shared-boundary ownership

The locator examines every face and sorts accepted candidates by revision-bound face index. A single candidate is returned directly. Multiple candidates are accepted only if every pair shares stable control-point IDs and the candidate coordinates place the query on the boundary formed by those shared IDs. The lowest face index then owns the point.

Candidates on unrelated faces return `AmbiguousFaceOwnership`; candidates sharing IDs without the query lying on the shared boundary return `InvalidCandidateBoundary`. This is a defensive check in addition to the global embedding certificate, not a way to repair invalid cages.

## Tests authored

- Triangle interior barycentric location.
- Shared triangle edge and shared vertex ownership.
- Outside-cage result.
- Inverse bilinear solution on a skewed convex quad.
- Quad corner inclusion.
- Adjacent quad shared-edge ownership.
- Defensive rejection of unrelated candidate boundaries.

## Scope boundary

This step does not enumerate Path2D source features, create complete binding records, implement outside-cage clamping/extrapolation, sample curves, persist bindings, or evaluate deformation. The caller combines the returned face coordinates with FDM-03.1 source-feature locators and binding records. Outside points remain explicitly reported as `OutsideCage` for the v1 `UnboundZeroInfluence` policy.
