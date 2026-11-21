# CCC-01.5 — Deformation Integration

**Status:** Implemented on a review branch; pending review.  
**Decision:** Keep controller and pin-deformation evaluation independent.

## Purpose

Provide a host-independent orchestration boundary that can invoke the consolidated Smart Controller binding evaluator and the existing Form Deform pin evaluator in one deterministic operation, while preserving their separate inputs, outputs, and errors.

This milestone integrates the evaluators at the orchestration/API boundary. It does not define multi-source deformation composition.

## Canonical implementation

The new forma-deformation-integration crate depends on:

- smart-controller-core for three-anchor binding evaluation.
- form-deform-core for bone-angle-driven absolute pin-position evaluation.

Neither core depends on the integration crate or on the other core. AniCraft, WASM, UI, and host document APIs are not dependencies.

The public evaluate_independent function accepts two optional, explicitly constructed requests and returns two independent result channels:

- Controller: Option<Result<Option<PreviewResult>, ControllerError>>
- Pin deformation: Option<Result<PinDeformationResult, DeformError>>

The nested controller option preserves the existing disabled-binding behavior: no request, evaluation error, disabled binding, and an evaluated preview remain distinguishable.

A failure in one evaluator does not prevent the other evaluator from running. Pin results retain their element and pin identities alongside the existing absolute stage-space position.

## Preserved semantics

- Controller evaluation continues to use its own input range, easing, mode, target anchors, and enabled state.
- Pin evaluation continues to use P = P0 + G * (theta - theta0).
- Controller GeometryDeform.offset is never passed into DeformSpec.
- The integration crate performs no coordinate conversion, displacement addition, source precedence, target matching, or host mutation.
- Smart Controller V1 remains preview-only. The integration API does not issue timeline or document commands.
- The existing AniCraft plugin adapters remain unchanged.

## Explicit non-goals

This milestone does not decide:

- Whether multiple deformation sources are additive, overriding, blended, or ordered.
- Whether controller absolute/relative modes can drive authored pin positions.
- Coordinate-space ownership or conversion.
- Geometry/document ownership, animation ownership, baking, or runtime representation.
- Any FDM v2 deformation model beyond this narrow boundary.

These require separate architecture decisions. Callers must treat the two outputs as independent until such a decision is accepted.

## Verification

Unit tests cover:

1. Both evaluators returning their independent expected outputs.
2. A controller failure not suppressing a valid pin evaluation.
3. A pin failure not suppressing a valid controller preview.
4. Omitted requests and disabled controller bindings.

The integration layer is host-agnostic and performs no document writes.
