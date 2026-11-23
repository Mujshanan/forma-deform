# FDM-01.3 — First Deformation Operator

**Status:** Initial implementation on `feat/fdm-01-3-single-handle-displacement`  
**Base:** FDM-01.2 operator invocation contract  
**Algorithm:** `fdm.single-handle-displacement` v1  
**Scope:** Forma Deform core only; no AniCraft changes

## Purpose

Provide one small, deterministic reference deformation that proves the FDM operator contract can transform canonical `Path2D` geometry without replacing its representation. This is not a replacement for AniCraft's native Asset Warp or its MLS pin solver.

## Authored parameter schema

The `PinDeformation` definition must declare `AbsoluteGeometry` as both input and output. The operator accepts exactly these five numeric parameters:

| Parameter | Meaning | Constraint |
|---|---|---|
| `handle_x`, `handle_y` | Handle center in the definition's declared coordinate space | Finite |
| `displacement_x`, `displacement_y` | Absolute displacement vector at the handle center | Finite |
| `influence_radius` | Radial support radius | Finite and strictly positive |

Missing parameters, extra parameters, non-numeric values, or a non-positive radius fail with `OperatorFailure::InvalidParameters`. The operator does not infer units or convert coordinate spaces; FDM-01.2 checks exact descriptor compatibility before dispatch.

## Mathematical definition

For each stored point (p=(x,y)), handle center (h=(h_x,h_y)), displacement vector (v), and radius (r>0):

[
d = \frac{\lVert p-h \rVert}{r}
]

For (0 \le d < 1), define cubic smoothstep falloff:

[
w(d)=1-(3d^2-2d^3)
]

For (d \ge 1), (w(d)=0). The mapped point is:

[
p' = p + w(d)v
]

The weight is 1 at the handle center, 0 at and beyond the radius, and has zero first derivative at both ends of the support interval. This gives a compact, smooth radial influence without a hard boundary jump.

The implementation normalizes coordinate deltas by the radius before calculating the Euclidean norm. It checks the axis-aligned support bound first, avoiding overflow from squaring large coordinates. Any non-finite output point is rejected; it is never silently clamped.

## Geometry fidelity

The operator maps every explicitly stored point:
- `MoveTo` and `LineTo` anchors;
- quadratic control and endpoint points;
- both cubic control points and the cubic endpoint.

It preserves command order, subpath structure, command kinds, and `Close` commands. It does not tessellate curves or replace `Path2D` with sampled polylines.

**Approximation boundary:** a nonlinear radial map does not generally map a Bézier curve to another Bézier curve of the same degree. Applying the field independently to stored anchors and control handles is therefore a control-point approximation to the pointwise-transformed curve, not an exact curve deformation. This limitation is explicit in v1. Adaptive sampling or a higher-fidelity curve representation would require a separately specified policy and is not silently introduced here.

## Evaluation semantics

- The operator is stateless and deterministic for identical definition, input geometry, request, and algorithm version.
- It always computes from the supplied input geometry; it never modifies the input or accumulates onto a previous output.
- It returns an absolute transformed `Path2D` (`GeometryValueKind::AbsoluteGeometry`), not a displacement-field payload.
- It does not read the host document, query the playhead, schedule work, mutate authored parameters, or apply results.
- The caller must use `evaluate_operator`; the shared dispatch contract validates the definition, operator identity, source snapshot stamp, coordinate space, and input/output kinds.

## Tests

The suite covers stable algorithm identity/version, full displacement at the handle center, half displacement at half radius, unchanged boundary/exterior points, zero-displacement identity, repeatability/non-accumulation, command topology preservation, Bézier control-point transformation, invalid parameter schema/radius, NaN/infinite parameters, tiny-radius/extreme-coordinate behavior, non-finite output rejection, and a regression test demonstrating that the mapped control-point curve is not generally identical to pointwise mapping of the original curve.

## Non-goals

- no AniCraft changes or host adapter;
- no replacement or reimplementation of native Asset Warp / MLS;
- no pin persistence, keyframing, tweening, baking, or UI;
- no graph scheduling or FDM-01.4 evaluator lifecycle;
- no Smart Controller integration or change to CCC-01.5;
- no curve tessellation, adaptive subdivision, or exact nonlinear Bézier remapping.

## Verification

CI is required on the final branch head for FDM-core rustfmt and Clippy gates, Rust workspace tests, existing TypeScript checks/tests/build, and all three release WASM plugin builds. Do not mark FDM-01.3 complete until those checks pass.
