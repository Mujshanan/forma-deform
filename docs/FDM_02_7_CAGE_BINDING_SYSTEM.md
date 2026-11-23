# FDM-02.7 — Cage Binding System

## Goal

Attach authored geometry points to a globally validated triangular cage and evaluate those attachments against later cage poses. This milestone builds directly on FDM-02.6; it does not introduce another topology, manifoldness, or intersection validator.

## Authoritative geometry contract

- A reference cage must pass `CageEmbedding::validate_global()` before any binding is created.
- Evaluation reuses that same validator for the current cage pose.
- The binding layer must not repair, weld, reorder, retriangulate, or otherwise mutate cage geometry.
- Stable `CageVertex::id` values, not vector indices, identify attachment support vertices.
- The binding layer consumes the cage's existing indexed triangles and `Point2` coordinate contract.

## Binding representation

Each attached point stores:

- its stable authored point ID;
- the three stable cage vertex IDs of its support triangle, sorted ascending;
- three corresponding barycentric weights, normalized to sum to one.

Triangle array indices are deliberately not persisted in a binding. Reordering the triangle array therefore does not change the identity of an attachment. The support vertex identity set must remain unchanged between bind and evaluation.

## Point location and deterministic edge behavior

- Point location uses triangle barycentric coordinates.
- Points outside the cage are rejected. There is no nearest-edge fallback or implicit extrapolation.
- A point on a shared edge or vertex can be contained by more than one triangle. Select the candidate whose sorted stable vertex-ID triple is lexicographically smallest.
- Duplicate authored point IDs are rejected.
- The initial implementation uses a dimensionless barycentric boundary tolerance of `1e-12`; this is a point-location policy, not a second geometric embedding validator.

## Evaluation

For each bound point, evaluate the weighted sum of its three support vertices in the current cage pose. This is absolute evaluation from the current cage, not incremental displacement from a previously evaluated point.

The current cage must pass the existing global embedding validator and have exactly the same stable vertex ID set as the reference cage. A topology/identity change requires an explicit rebind; it is never silently accommodated.

## Initial implementation scope

- Host-independent binding model in `form-deform-core`.
- Bind and evaluate APIs.
- Stable shared-edge tie breaking.
- Outside-cage, duplicate-point-ID, invalid-cage, and changed-vertex-set errors.
- Tests for rest-pose identity, cage translation, deterministic shared-edge selection, and rejection cases.

## Explicit non-goals

- AniCraft document persistence or undo/redo integration.
- Host selection and Stage adapters.
- Automatic rebinding after topology edits.
- Cage construction or triangulation.
- A second topology/intersection/manifoldness implementation.
- Higher-order deformation, extrapolation, or nearest-boundary behavior.

## Follow-on work

After this core contract is validated, add document persistence for binding records and revision/fingerprint stamps, then connect host geometry extraction and evaluation scheduling. Persistence must record enough cage identity and source geometry revision information to detect stale bindings rather than silently reusing them.
