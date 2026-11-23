# FDM-05.4 — Native AniCraft Output Consumption Contract

**Status:** contract proposal for native integration; this document does not implement AniCraft host code.

## 1. Core-owned result

`AssembledDeformationOutput` is the core-owned paired result:

- `structured_path()` is the authoritative, command-preserving evaluated `Path2D`.
- `refined_curves()` is derived output-space sampling grouped by original command and subpath.
- `structured()` exposes the evaluated binding result and its provenance/diagnostic information.

The host consumes these values immutably. It must not replace the structured path with sampled points or mutate the returned geometry as a side effect of rendering.

## 2. Host responsibilities

AniCraft owns:

- Request scheduling, cancellation, and stale-result suppression.
- Document and target lifecycle, including target-generation changes.
- Cache keys and invalidation.
- Durable authored source geometry, cage/topology state, binding persistence, and undo/redo.
- UI, selection, editing tools, rendering, apply/bake, and user-facing diagnostics.
- Deciding when a successful core result becomes the document's applied output.

The core owns geometry validation, binding evaluation, curve refinement, assembly invariants, and typed failure results. The host must not duplicate or silently override those calculations.

## 3. Cache and invalidation rules

A cached assembled result is reusable only while all of its evaluation inputs remain identical:

- Source snapshot identity/revision and content fingerprint.
- Validated binding stamp and mapping/outside-cage policy.
- Topology identity and revision.
- Rest-cage geometry fingerprint.
- Current-cage geometry fingerprint / pose.
- Output-refinement policy and policy version.

Any change to one of these inputs invalidates the cached result. A target deleted and recreated under the same visible identifier must be treated as a new target generation. An older asynchronous result must never overwrite a newer request's result.

## 4. Persistence and recomputation

The authored `Path2D`, topology, control state, and validated/persisted binding data are durable document inputs under host ownership.

Refined curve samples are derived data. Prefer recomputation from the authoritative source and current evaluation inputs. If the host elects to cache derived samples durably, the cache must be versioned and keyed by the complete input identity above; it must never become an alternate editable source of truth.

Undo/redo restores authored inputs and revisions, then invalidates or recomputes derived output. It must not feed a previously deformed output back as a new source implicitly.

## 5. Rendering and geometry consumers

- Path editing, persistence, and semantic document operations use `structured_path()`.
- Curve-accurate rendering or downstream curve consumers may use `refined_curves()`.
- Consumers preserve `command_index`, `subpath_index`, and parameter `t`; they must not infer new authored commands from sample points.
- Refined samples are a bounded approximation controlled by the declared policy. `max_observed_deviation` is an observed probe estimate, not a certified global error bound.
- A consumer requiring a different tolerance or budget requests a new evaluation with a different policy; it does not post-process the existing samples and claim a stronger guarantee.

## 6. Failure handling

Treat any core error as a failed evaluation, not as a partial success. Keep the last known-good applied result separate from a failed in-flight request. Surface actionable diagnostics for stale provenance, invalid binding/policy, resource-budget exhaustion, or malformed output.

Do not silently fall back to a different outside-cage policy, a different refinement policy, or a flattened approximation.

## 7. Smart Controller boundary

Smart Controller evaluation and FDM pin-deformation evaluation remain independent. This output contract does not define multi-source composition, ordering, conflict resolution, or blending. Any future composition requires a separate approved semantic contract.

## 8. Explicit non-goals

This contract does not add a new deformation algorithm, implement host UI/rendering, retire historical adapters, or authorize SDK removal. It does not authorize PR merges.
