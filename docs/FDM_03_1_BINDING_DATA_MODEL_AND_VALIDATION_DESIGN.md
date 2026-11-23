# FDM-03.1 — Binding Data Model and Validation Design

## Status

**Revised design after external-code review.** This is an implementation specification, not a claim that binding types, point location, or binding construction have been implemented.

FDM-03.1 defines the immutable binding data model, source-feature enumeration, geometry access boundary, mapping-policy identity, freshness validation, and fail-closed validation rules. It does not perform point-location search or persist bindings.

## 1. Goals and scope

FDM-03.1 establishes a trustworthy contract between an immutable Path2D v1 source snapshot and an FDM cage before FDM-03.2 implements point location.

Included:
- Strongly typed, snapshot-local source-feature locators and deterministic feature enumeration.
- Binding/source/topology stamps and versioned policy identifiers.
- A read-only validated cage geometry view combining topology and control positions.
- Triangle and quadrilateral face-coordinate types.
- Binding-record validation, freshness checks, and typed diagnostics.
- A prerequisite gate for globally unambiguous cage embedding.

Excluded:
- Point-location search and acceleration structures (FDM-03.2).
- Boundary projection and extrapolation (FDM-03.3).
- Curve tessellation/adaptive subdivision (FDM-03.4).
- Serialization and host persistence orchestration (FDM-03.5).
- Cage deformation, Bézier-handle transformation, UI, Smart Controller composition, native Asset Warp conversion, and baking.

FDM-03.1 may validate a binding record supplied with mapping coordinates, but it must not discover those coordinates. That belongs to FDM-03.2.

## 2. External-review decisions

The reviewed projects provide useful algorithms, not a complete binding contract:

- Aether Canvas offers barycentric point-location and mesh-transfer ideas. Its first-hit triangle selection, fixed absolute degeneracy threshold, and f32 weights are not adopted as FDM policy.
- OSATURA's mean-value polygon cage is a distinct model. Its f64 numerical and boundary-handling practices are useful, but its cage coordinates do not replace FDM's triangle/quad face coordinates.
- OpenBone's path-editing and rebinding ideas are useful, but nearest-old-point fallback is not accepted as a binding rule.
- LibreToon and Fanimate remain references for later bone/weight and controller work, not this binding layer.
- OpenToonz Plastic is a reference for mesh-editing workflows. No ARAP solver assumption is made.

FDM owns its types, numerical policy, validation, and deterministic behavior. No external project code is copied wholesale.

## 3. Mapping policy decision

### 3.1 Triangles

Triangle faces use direct barycentric coordinates (w0, w1, w2), associated with the three control-point IDs in authored face order:

P = w0 A + w1 B + w2 C

For the v1 inside-only policy:
- every weight must be finite and in [0, 1];
- the sum must equal one within the centralized dimensionless weight tolerance;
- negative weights are rejected. Extrapolation is deferred to FDM-03.3 and requires a distinct policy identity.

### 3.2 Quadrilaterals

**Decision: use bilinear coordinates for every quad face in v1. Do not split quads into triangles.**

For the authored vertex order [P00, P10, P11, P01], coordinates (u, v) lie in the unit square:

N00 = (1-u)(1-v)
N10 = u(1-v)
N11 = uv
N01 = (1-u)v

P(u,v) = N00 P00 + N10 P10 + N11 P11 + N01 P01

Reasons:
- all four authored vertices participate in one continuous face mapping;
- no artificial diagonal or diagonal-dependent tie-break is introduced;
- regular-lattice cells and irregular convex quads share the same mapping contract;
- the mapping has an explicit two-parameter domain.

The current irregular-cage validator requires convex quads. FDM-03.2 must nevertheless solve and verify the inverse bilinear map robustly; it must not assume convexity alone guarantees numerical convergence.

A quad mapping is valid only when:
- u and v are finite and in [0, 1];
- the quad is non-degenerate and passes cage geometry validation;
- the inverse solution reconstructs the source point within the mapping tolerance;
- the solver reports a unique accepted solution inside the face domain.

If inverse bilinear solving is singular, ill-conditioned, non-convergent, outside-domain, or ambiguous, the face is not a match. Do not fall back silently to a diagonal split.

### 3.3 Policy identity

The initial mixed-face policy is a named, versioned policy:
- triangle face: barycentric v1;
- quad face: bilinear v1;
- inside-domain only;
- no extrapolation.

Represent this with a typed enum or stable constant, not an arbitrary free-form string. Any change to face-coordinate mathematics, acceptance tolerances, or face-order interpretation requires a new policy version and stale-binding behavior.

## 4. Data model

Introduce a dedicated binding module (proposed: crates/form-deform-core/src/binding.rs). Keep topology types in topology.rs. Names may be adjusted to repository conventions, but semantics must not be weakened.

### 4.1 Source feature locator

A source locator contains:
- command_index: u32;
- role: PathFeatureRole.

Roles:
- MoveToEndpoint;
- LineToEndpoint;
- QuadraticControl;
- QuadraticEndpoint;
- CubicControl1;
- CubicControl2;
- CubicEndpoint.

Close has no coordinate-bearing feature and produces no locator.

The locator is valid only with the exact source fingerprint and Path2D schema version in the binding stamp. It is not a persistent host point ID.

### 4.2 Source feature point and ordering

A source feature point contains its locator, exact source-space Point2, deterministic enumeration ordinal, and subpath context useful for diagnostics.

Enumeration is deterministic:
1. Traverse commands in ascending command index.
2. Emit features in authored command order.
3. For a quadratic command emit control then endpoint.
4. For a cubic command emit control1, control2, then endpoint.
5. Emit no feature for Close.

The source Path2D remains immutable and authoritative. No sampled-polyline vertex is promoted to a source feature identity.

### 4.3 Validated cage geometry view

TopologyView alone is insufficient: it exposes control-point IDs and faces, but not coordinates needed to verify that a mapping reconstructs the source feature.

Introduce a read-only validated geometry view combining:
- one DeformationTopology;
- one DeformationControlState whose topology ID/revision matches;
- a unique ID-to-position lookup derived from control state;
- a geometry-validation/embedding status.

Construction validates the control-state point set against topology and rejects duplicate, missing, extra, or non-finite control positions. Lookups are by ControlPointId, never by assumed shared vector index.

The view is immutable for the lifetime of a binding-construction operation. A later control-state revision may change positions, but it does not mutate an existing view or a stored rest-space binding.

### 4.4 Mapping coordinate and record

Use a tagged coordinate enum:
- TriangleBarycentric { weights: [f64; 3] };
- QuadBilinear { u: f64, v: f64 }.

Each mapping record contains:
- source feature locator;
- face locator (face index, valid only under the stamped topology ID/revision);
- stable control-point IDs in the face's authored order, copied into the record for validation and auditability;
- tagged face coordinates;
- mapping-policy version.

Do not store a bare vector index as a durable identity. The face index is only a revision-bound locator; stable control-point IDs are the identity-bearing references.

### 4.5 Binding stamp and record

The binding stamp contains all FDM-03.0 fields:
- host target identity and generation;
- source geometry revision;
- source Path2D topology revision;
- source fingerprint algorithm and digest;
- coordinate-space descriptor;
- Path2D payload kind and schema version;
- FDM topology ID and topology revision;
- binding schema version;
- mapping-policy version;
- outside-cage policy.

Control-state revision is deliberately excluded.

A binding record contains the immutable stamp, deterministic ordered mapping records, an explicit status for every enumerated source feature, and validation metadata.

For FDM-03.1 the only supported outside policy is UnboundZeroInfluence. Future enum variants may be reserved, but constructing a binding with ClampToBoundary or Extrapolate must return an unsupported-policy error until FDM-03.3 implements them.

A feature outside the cage is represented explicitly as unbound under UnboundZeroInfluence. It must not be omitted, because omission makes completeness unverifiable.

## 5. Global embedding prerequisite

FDM-02.3 validates individual faces and cage boundaries but does not yet prove that non-adjacent interior faces cannot overlap. Point location cannot safely use first-hit selection where unrelated faces cover the same source point.

**Binding construction must require a globally unambiguous cage-embedding certificate.**

The certificate must be produced only by a geometry validator and must establish at minimum:
- all face geometry is finite and non-degenerate;
- face winding and adjacency are consistent;
- non-adjacent face interiors do not overlap;
- non-adjacent edges do not cross or touch except where topology explicitly defines adjacency;
- adjacent faces share only their declared common edge/vertices;
- no face has an invalid self-intersection.

Do not expose a public boolean callers can set to claim validation. Use a validated/certified type whose constructor is private to the validator.

The current FDM-02 implementation does not provide this global certificate. FDM-03.1 may implement its data model and validators, but it must not expose a successful binding-construction path for an uncertified cage.

**Sequencing adjustment:** add FDM-02.6 — Global Cage Embedding Validation as a prerequisite. It produces a certificate tied to topology ID/revision and the cage geometry revision or geometry fingerprint. FDM-03.1 accepts only that certified geometry view; FDM-03.2 performs point location against it.

The certificate is invalidated when cage control positions change. This does not make an existing rest-space binding stale; it means the geometry used for a new bind or point-location operation must be revalidated. If a future product workflow binds against a separately frozen rest cage, that rest-cage snapshot must be explicit.

## 6. Mapping validation rules

Validation is centralized and shared by binding construction and any future deserialization path.

### 6.1 Stamp validation

Reject missing, malformed, unsupported, or inconsistent stamp fields. Freshness comparison includes every FDM-03.0 source/target/topology field and mapping policy. A mismatch returns a stale-binding diagnostic identifying the mismatched dimension; it never auto-rebinds.

### 6.2 Source feature validation

For every mapping:
- command index exists in the stamped Path2D snapshot;
- role is valid for that exact command variant;
- locator appears at most once;
- every coordinate-bearing source feature has exactly one status;
- Close has no mapping;
- source point is finite.

Reject duplicate, missing, or role/command-mismatched locators.

### 6.3 Face validation

For every mapped feature:
- face index exists in the stamped topology;
- face kind agrees with the tagged coordinate variant;
- recorded control-point IDs exactly match the face's authored ordered IDs;
- all IDs exist in the certified geometry view;
- no duplicate IDs occur within a face.

### 6.4 Triangle coordinate validation

- all three weights are finite;
- all weights are in [0, 1];
- sum equals one within the centralized weight-sum tolerance;
- reconstructed point matches the source feature within the scale-aware spatial tolerance.

Do not silently normalize arbitrary weights or clamp negative weights. Any bounded numerical cleanup belongs in point location and must be deterministic and tested.

### 6.5 Quad coordinate validation

- u and v are finite and in [0, 1];
- shape weights are derived from the bilinear formula, not stored as a contradictory second representation;
- reconstructed point matches the source feature within the scale-aware spatial tolerance;
- coordinates are not accepted merely because they are in range if they reconstruct a different source point.

### 6.6 Completeness

Every source feature is accounted for exactly once as either:
- mapped to one valid face coordinate; or
- explicitly unbound under UnboundZeroInfluence.

No feature may be silently dropped. A partial result uses a distinct result/status type and must never be reported as a complete binding.

## 7. Deterministic face selection and ambiguity

FDM-03.1 defines record validation, not face search. FDM-03.2 must honor these invariants:

- Do not use first-hit face selection as the ambiguity policy.
- Stable face iteration order alone does not resolve geometric ambiguity.
- A point on a shared edge or vertex may belong to multiple adjacent faces. FDM-03.2 must apply a documented deterministic ownership rule among adjacent candidates, such as the lowest stable face locator after confirming topological adjacency and equivalent boundary reconstruction.
- A point covered by overlapping non-adjacent faces indicates an invalid cage embedding, not a tie to resolve by face index.
- If multiple non-adjacent candidates remain, return an ambiguity error; never pick one arbitrarily.

Faces currently have no persistent face IDs. The selected face locator is therefore face_index protected by exact topology ID/revision. Do not invent face identity from vector offsets.

## 8. Numerical policy

- Use f64 throughout binding construction and validation.
- Centralize tolerances in a named mapping numerical policy; do not scatter fixed literals such as 1e-12 or -1e-5 through geometry code.
- Spatial tolerances scale with cage bounds/characteristic length and machine precision, with explicit lower/upper safeguards.
- Weight-sum tolerance is dimensionless and separate from spatial tolerance.
- Degeneracy tests are scale-aware; a fixed absolute area threshold is unacceptable.
- Reject NaN, infinity, overflow, singular solves, and non-finite intermediates.
- Tolerance policy/version is part of mapping-policy compatibility. Changing acceptance behavior requires a new policy version.

FDM-03.1 defines the policy surface and validation entry points. FDM-03.2 supplies and tests the concrete inverse-bilinear and barycentric solvers.

## 9. Freshness and revision behavior

Binding freshness requires exact agreement on:
- host target identity and generation;
- source geometry revision;
- source Path2D topology revision;
- source fingerprint;
- coordinate-space descriptor;
- Path2D payload kind/schema;
- FDM topology ID/revision;
- mapping-policy version;
- outside-cage policy.

Control-state revision remains excluded from binding freshness. Cage control movement changes evaluation geometry and invalidates any geometry-validation certificate used for new point location, but does not invalidate a rest-space binding whose topology is unchanged.

Evaluation/cache keys must include current control-state revision independently.

## 10. Proposed module layout

- binding.rs: stamps, source locators, mapping records, policy enums, validation errors, freshness checks.
- source_features.rs (or private submodule): deterministic Path2D feature enumeration.
- cage_geometry.rs (or private submodule): validated topology + control-position view and embedding-certificate boundary.

Avoid over-fragmenting the first implementation. Keep private submodules until the public API stabilizes if that is clearer. Export only stable public contracts from lib.rs. Constructors for validated/certified types must be private or restricted so ordinary callers cannot forge invalid instances.

## 11. Error model

Use typed errors with enough context for host diagnostics. At minimum distinguish:
- unsupported binding schema or mapping policy;
- unsupported outside-cage policy;
- stale source stamp;
- stale topology stamp;
- topology/control-state mismatch;
- missing/duplicate control point;
- missing/invalid source feature;
- face index out of range;
- face-kind/mapping-kind mismatch;
- control-point ID order mismatch;
- non-finite mapping coordinate;
- mapping coordinate outside its valid domain;
- weight sum mismatch;
- source-point reconstruction mismatch;
- uncertified or invalid cage embedding;
- ambiguous face ownership;
- incomplete feature coverage.

Do not collapse all failures into a generic InvalidBinding error.

## 12. Acceptance criteria

FDM-03.1 is complete only when:
1. Source feature enumeration follows Path2D v1 command/role ordering and preserves source geometry unchanged.
2. Every locator is explicitly snapshot-local and validated against the exact command variant.
3. Triangle barycentric and quad bilinear coordinate types are distinct and versioned.
4. Quads use bilinear coordinates; no implicit diagonal split exists in the v1 policy.
5. Binding validation checks reconstructed positions using ID-based control-position lookup.
6. Negative barycentric weights and out-of-domain quad coordinates are rejected for the v1 inside-only policy.
7. Every source feature has exactly one mapped or explicitly unbound status.
8. Freshness excludes control-state revision but includes all source, target, topology, coordinate, and policy stamps from FDM-03.0.
9. A binding cannot be marked complete against a cage lacking a valid global embedding certificate.
10. Validation is deterministic, typed, and fail-closed.
11. Unit tests cover valid/invalid records, duplicate/missing locators, face ID order mismatch, non-finite values, bad weight sums, out-of-domain quad coordinates, stale stamps, incomplete coverage, and uncertified cages.
12. No tests/build/format checks are claimed as passing unless actually run.

## 13. Revised implementation sequence

- **FDM-02.6 — Global Cage Embedding Validation (new prerequisite):** prove non-overlapping, unambiguous cage embedding and issue a revision/geometry-bound certificate.
- **FDM-03.1 — Binding Data Model and Validation:** implement this document's data contracts and validators; no point-location search.
- **FDM-03.2 — Deterministic Point Location:** triangle barycentric and quad inverse-bilinear point location, deterministic shared-boundary ownership, ambiguity rejection.
- **FDM-03.3 — Outside-Cage Policies:** boundary projection and bounded extrapolation under separate explicit policy versions.
- **FDM-03.4 — Curve Feature Sampling:** adaptive curve mapping with explicit error/resource budgets; preserve original Path2D commands.
- **FDM-03.5 — Binding Persistence and Host Adapter:** versioned schema, stale-binding diagnostics, and AniCraft adapter integration.

## 14. Final implementation guardrails

- Do not copy external application architecture or solver code wholesale.
- Do not add dependencies solely to imitate an external project.
- Do not change topology identity/revision semantics.
- Do not combine FDM pin deformation with Smart Controller outputs.
- Do not touch AniCraft native Asset Warp internals.
- Do not silently auto-rebind stale source geometry.
- Do not implement FDM-03.2 point search as part of FDM-03.1.
- Do not mark FDM-03.1 complete until implementation and requested verification are complete.
