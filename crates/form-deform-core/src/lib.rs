//! Pure deformation domain: bone transforms, weighted mesh deformation, and pin targets.
//!
//! Diagnostics are opt-in for metrics. Warnings are generated only when callers provide
//! mesh triangles and bone segment endpoints; this core does not infer topology or bone
//! lengths from rigid transforms. The current algorithm is linear-blend skinning (LBS),
//! not an iterative constraint solver, so solver metrics are explicitly absent.

use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 { pub x: f64, pub y: f64 }
impl Vec2 {
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() }
    pub fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
    pub fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
    pub fn scale(self, factor: f64) -> Self { Self { x: self.x * factor, y: self.y * factor } }
    pub fn length_squared(self) -> f64 { self.x * self.x + self.y * self.y }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneTransform { pub translation: Vec2, pub rotation_degrees: f64 }
impl BoneTransform {
    pub fn transform_point(self, point: Vec2) -> Result<Vec2, DeformError> {
        if !point.is_finite() || !self.translation.is_finite() || !self.rotation_degrees.is_finite() {
            return Err(DeformError::NonFiniteInput);
        }
        let (sin, cos) = self.rotation_degrees.to_radians().sin_cos();
        let result = Vec2 {
            x: self.translation.x + point.x * cos - point.y * sin,
            y: self.translation.y + point.x * sin + point.y * cos,
        };
        if result.is_finite() { Ok(result) } else { Err(DeformError::ArithmeticOverflow) }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BonePose { pub id: u64, pub world: BoneTransform }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneWeight { pub bone_id: u64, pub weight: f64 }
#[derive(Clone, Debug, PartialEq)]
pub struct MeshVertex { pub bind_position: Vec2, pub influences: Vec<BoneWeight> }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshPin { pub id: u64, pub vertex_index: usize, pub target: Vec2 }
#[derive(Clone, Debug, PartialEq)]
pub struct DeformMesh { pub vertices: Vec<MeshVertex>, pub pins: Vec<MeshPin> }

/// Hard upper bounds applied before the core allocates output or diagnostic buffers.
pub const MAX_MESH_VERTICES: usize = 250_000;
pub const MAX_MESH_BONES: usize = 16_384;
pub const MAX_MESH_INFLUENCES: usize = 1_000_000;
pub const MAX_MESH_PINS: usize = 250_000;
pub const MAX_DIAGNOSTIC_TRIANGLES: usize = 500_000;
pub const MAX_DIAGNOSTIC_BONE_SEGMENTS: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeformResource {
    Vertices, Bones, Influences, Pins, Triangles, BoneSegments,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MeshDeformResult { pub vertices: Vec<Vec2>, pub pins: Vec<MeshPin> }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformSpec { pub neutral_position: Vec2, pub neutral_angle_degrees: f64, pub gain_per_degree: Vec2 }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneAngle { pub keyed_degrees: f64, pub spring_swing_degrees: Option<f64> }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformRequest { pub spec: DeformSpec, pub bone_angle: BoneAngle, pub include_spring_swing: bool }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformResult {
    pub keyed_angle_degrees: f64,
    pub spring_swing_degrees: Option<f64>,
    pub effective_angle_degrees: f64,
    pub angle_delta_degrees: f64,
    pub target_position: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeformError {
    NonFiniteInput, MissingSpringSwing, MissingBone(u64), MissingPin(u64), MissingVertex(usize),
    MissingInfluences, MissingInfluenceBone(u64), InvalidWeight, InvalidWeightSum,
    InvalidTriangleIndex { triangle: usize, vertex: usize }, ArithmeticOverflow,
    ResourceLimitExceeded { resource: DeformResource, actual: usize, limit: usize },
    DuplicateBone(u64), DuplicatePin(u64), AllocationFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeformErrorCode {
    NonFiniteInput, MissingSpringSwing, MissingBone, MissingPin, MissingVertex,
    MissingInfluences, MissingInfluenceBone, InvalidWeight, InvalidWeightSum,
    InvalidTriangleIndex, ArithmeticOverflow, ResourceLimitExceeded, DuplicateBone,
    DuplicatePin, AllocationFailed,
}
impl DeformError {
    pub fn code(self) -> DeformErrorCode {
        match self {
            Self::NonFiniteInput => DeformErrorCode::NonFiniteInput,
            Self::MissingSpringSwing => DeformErrorCode::MissingSpringSwing,
            Self::MissingBone(_) => DeformErrorCode::MissingBone,
            Self::MissingPin(_) => DeformErrorCode::MissingPin,
            Self::MissingVertex(_) => DeformErrorCode::MissingVertex,
            Self::MissingInfluences => DeformErrorCode::MissingInfluences,
            Self::MissingInfluenceBone(_) => DeformErrorCode::MissingInfluenceBone,
            Self::InvalidWeight => DeformErrorCode::InvalidWeight,
            Self::InvalidWeightSum => DeformErrorCode::InvalidWeightSum,
            Self::InvalidTriangleIndex { .. } => DeformErrorCode::InvalidTriangleIndex,
            Self::ArithmeticOverflow => DeformErrorCode::ArithmeticOverflow,
            Self::ResourceLimitExceeded { .. } => DeformErrorCode::ResourceLimitExceeded,
            Self::DuplicateBone(_) => DeformErrorCode::DuplicateBone,
            Self::DuplicatePin(_) => DeformErrorCode::DuplicatePin,
            Self::AllocationFailed => DeformErrorCode::AllocationFailed,
        }
    }
}
impl fmt::Display for DeformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "deformation inputs must be finite"),
            Self::MissingSpringSwing => write!(f, "spring swing was requested but is unavailable"),
            Self::MissingBone(id) => write!(f, "bone {id} is not present in the deformation rig"),
            Self::MissingPin(id) => write!(f, "pin {id} is not present in the deformation mesh"),
            Self::MissingVertex(i) => write!(f, "mesh vertex {i} is out of range"),
            Self::MissingInfluences => write!(f, "mesh vertex has no bone influences"),
            Self::MissingInfluenceBone(id) => write!(f, "skin influence references missing bone {id}"),
            Self::InvalidWeight => write!(f, "skin weights must be finite and non-negative"),
            Self::InvalidWeightSum => write!(f, "skin weights must sum to 1 within tolerance"),
            Self::InvalidTriangleIndex { triangle, vertex } => write!(f, "triangle {triangle} references missing vertex {vertex}"),
            Self::ArithmeticOverflow => write!(f, "deformation result is not finite"),
            Self::ResourceLimitExceeded { resource, actual, limit } =>
                write!(f, "{resource:?} count {actual} exceeds configured limit {limit}"),
            Self::DuplicateBone(id) => write!(f, "bone ID {id} is duplicated"),
            Self::DuplicatePin(id) => write!(f, "pin ID {id} is duplicated"),
            Self::AllocationFailed => write!(f, "could not reserve bounded deformation output memory"),
        }
    }
}
impl std::error::Error for DeformError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneSegment { pub bone_id: u64, pub start: Vec2, pub end: Vec2 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterpolationMode { LinearBlendSkinning }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolverMetrics { pub iterations: u32, pub converged: bool }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformMetrics {
    pub vertex_count: usize,
    pub bone_count: usize,
    /// Native elapsed time when timing is supported; None on wasm32-unknown-unknown.
    pub computation_time_ns: Option<u64>,
    pub interpolation_mode: InterpolationMode,
    /// None because the current LBS implementation has no iterative solver.
    pub solver: Option<SolverMetrics>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DeformWarning {
    DegenerateTriangle { triangle_index: usize, area: f64 },
    ZeroLengthBone { bone_id: u64 },
}
#[derive(Clone, Debug, PartialEq)]
pub struct DeformDiagnostics {
    pub metrics: Option<DeformMetrics>,
    pub warnings: Vec<DeformWarning>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DeformExecution {
    pub result: MeshDeformResult,
    pub diagnostics: DeformDiagnostics,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct DeformDiagnosticsOptions<'a> {
    pub collect_metrics: bool,
    /// Optional triangle topology. Degeneracy uses absolute signed area <= 1e-12.
    pub triangles: &'a [[usize; 3]],
    /// Optional rest/stage-space segments used to identify zero-length bones.
    pub bone_segments: &'a [BoneSegment],
}
#[derive(Clone, Debug, PartialEq)]
pub struct DeformDiagnosticError {
    pub code: DeformErrorCode,
    pub source: DeformError,
    pub message: String,
}
impl fmt::Display for DeformDiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", format!("{:?}", self.code), self.message)
    }
}
impl std::error::Error for DeformDiagnosticError {}
impl From<DeformError> for DeformDiagnosticError {
    fn from(source: DeformError) -> Self {
        Self { code: source.code(), message: source.to_string(), source }
    }
}

pub fn evaluate_deformation(request: DeformRequest) -> Result<DeformResult, DeformError> {
    let spec = request.spec;
    let keyed = request.bone_angle.keyed_degrees;
    let spring = request.bone_angle.spring_swing_degrees;
    if !spec.neutral_position.is_finite() || !spec.gain_per_degree.is_finite()
        || !spec.neutral_angle_degrees.is_finite() || !keyed.is_finite()
        || spring.is_some_and(|value| !value.is_finite()) { return Err(DeformError::NonFiniteInput); }
    if request.include_spring_swing && spring.is_none() { return Err(DeformError::MissingSpringSwing); }
    let effective = keyed + if request.include_spring_swing { spring.unwrap_or(0.0) } else { 0.0 };
    let delta = effective - spec.neutral_angle_degrees;
    let target = spec.neutral_position.add(spec.gain_per_degree.scale(delta));
    if !effective.is_finite() || !delta.is_finite() || !target.is_finite() { return Err(DeformError::ArithmeticOverflow); }
    Ok(DeformResult { keyed_angle_degrees: keyed, spring_swing_degrees: spring,
        effective_angle_degrees: effective, angle_delta_degrees: delta, target_position: target })
}

/// Backwards-compatible API. Use deform_mesh_with_diagnostics for structured diagnostics.
pub fn deform_mesh(mesh: &DeformMesh, bones: &[BonePose]) -> Result<MeshDeformResult, DeformError> {
    deform_mesh_with_diagnostics(mesh, bones, DeformDiagnosticsOptions::default())
        .map(|execution| execution.result).map_err(|error| error.source)
}

/// Apply linear-blend skinning and optionally collect metrics and topology warnings.
pub fn deform_mesh_with_diagnostics(
    mesh: &DeformMesh,
    bones: &[BonePose],
    options: DeformDiagnosticsOptions<'_>,
) -> Result<DeformExecution, DeformDiagnosticError> {
    // Validate caller-controlled collection sizes before any output/warning allocation.
    check_limit(DeformResource::Vertices, mesh.vertices.len(), MAX_MESH_VERTICES)?;
    check_limit(DeformResource::Bones, bones.len(), MAX_MESH_BONES)?;
    check_limit(DeformResource::Pins, mesh.pins.len(), MAX_MESH_PINS)?;
    check_limit(DeformResource::Triangles, options.triangles.len(), MAX_DIAGNOSTIC_TRIANGLES)?;
    check_limit(DeformResource::BoneSegments, options.bone_segments.len(), MAX_DIAGNOSTIC_BONE_SEGMENTS)?;
    let influence_count = mesh.vertices.iter().try_fold(0usize, |count, vertex| {
        count.checked_add(vertex.influences.len())
    }).ok_or(DeformError::ArithmeticOverflow)?;
    check_limit(DeformResource::Influences, influence_count, MAX_MESH_INFLUENCES)?;

    let mut bone_lookup = std::collections::HashMap::new();
    bone_lookup.try_reserve(bones.len()).map_err(|_| DeformError::AllocationFailed)?;
    for bone in bones {
        if bone_lookup.insert(bone.id, bone).is_some() {
            return Err(DeformError::DuplicateBone(bone.id).into());
        }
    }
    let mut pin_ids = std::collections::HashSet::new();
    pin_ids.try_reserve(mesh.pins.len()).map_err(|_| DeformError::AllocationFailed)?;
    for pin in &mesh.pins {
        if !pin_ids.insert(pin.id) { return Err(DeformError::DuplicatePin(pin.id).into()); }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let started = if options.collect_metrics { Some(std::time::Instant::now()) } else { None };

    let warning_capacity = options.triangles.len().checked_add(options.bone_segments.len())
        .ok_or(DeformError::ArithmeticOverflow)?;
    let mut warnings = Vec::new();
    warnings.try_reserve_exact(warning_capacity).map_err(|_| DeformError::AllocationFailed)?;
    for (triangle_index, triangle) in options.triangles.iter().enumerate() {
        for vertex in triangle {
            if *vertex >= mesh.vertices.len() {
                return Err(DeformError::InvalidTriangleIndex { triangle: triangle_index, vertex: *vertex }.into());
            }
        }
        let a = mesh.vertices[triangle[0]].bind_position;
        let b = mesh.vertices[triangle[1]].bind_position;
        let c = mesh.vertices[triangle[2]].bind_position;
        if !a.is_finite() || !b.is_finite() || !c.is_finite() { return Err(DeformError::NonFiniteInput.into()); }
        let twice_area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        let area = twice_area.abs() * 0.5;
        if !area.is_finite() { return Err(DeformError::ArithmeticOverflow.into()); }
        if area <= 1e-12 { warnings.push(DeformWarning::DegenerateTriangle { triangle_index, area }); }
    }
    for segment in options.bone_segments {
        if !segment.start.is_finite() || !segment.end.is_finite() { return Err(DeformError::NonFiniteInput.into()); }
        if segment.end.sub(segment.start).length_squared() <= 1e-24 {
            warnings.push(DeformWarning::ZeroLengthBone { bone_id: segment.bone_id });
        }
    }

    let mut positions = Vec::new();
    positions.try_reserve_exact(mesh.vertices.len()).map_err(|_| DeformError::AllocationFailed)?;
    for vertex in &mesh.vertices {
        if !vertex.bind_position.is_finite() { return Err(DeformError::NonFiniteInput.into()); }
        if vertex.influences.is_empty() { return Err(DeformError::MissingInfluences.into()); }
        let mut sum = 0.0;
        let mut position = Vec2::default();
        for influence in &vertex.influences {
            if !influence.weight.is_finite() || influence.weight < 0.0 { return Err(DeformError::InvalidWeight.into()); }
            let bone = bone_lookup.get(&influence.bone_id)
                .copied().ok_or(DeformError::MissingInfluenceBone(influence.bone_id))?;
            let transformed = bone.world.transform_point(vertex.bind_position)?;
            position = position.add(transformed.scale(influence.weight));
            sum += influence.weight;
        }
        if (sum - 1.0).abs() > 1e-6 { return Err(DeformError::InvalidWeightSum.into()); }
        if !position.is_finite() { return Err(DeformError::ArithmeticOverflow.into()); }
        positions.push(position);
    }
    for pin in &mesh.pins {
        if pin.vertex_index >= positions.len() { return Err(DeformError::MissingVertex(pin.vertex_index).into()); }
        if !pin.target.is_finite() { return Err(DeformError::NonFiniteInput.into()); }
    }

    #[cfg(not(target_arch = "wasm32"))]
    let elapsed = started.map(|instant| instant.elapsed().as_nanos().min(u64::MAX as u128) as u64);
    #[cfg(target_arch = "wasm32")]
    let elapsed: Option<u64> = None;

    let metrics = options.collect_metrics.then_some(DeformMetrics {
        vertex_count: mesh.vertices.len(),
        bone_count: bones.len(),
        computation_time_ns: elapsed,
        interpolation_mode: InterpolationMode::LinearBlendSkinning,
        solver: None,
    });
    let mut pins = Vec::new();
    pins.try_reserve_exact(mesh.pins.len()).map_err(|_| DeformError::AllocationFailed)?;
    pins.extend_from_slice(&mesh.pins);
    Ok(DeformExecution {
        result: MeshDeformResult { vertices: positions, pins },
        diagnostics: DeformDiagnostics { metrics, warnings },
    })
}

fn check_limit(resource: DeformResource, actual: usize, limit: usize) -> Result<(), DeformError> {
    if actual > limit {
        Err(DeformError::ResourceLimitExceeded { resource, actual, limit })
    } else {
        Ok(())
    }
}

pub fn find_bone<'a>(bones: &'a [BonePose], id: u64) -> Result<&'a BonePose, DeformError> {
    bones.iter().find(|bone| bone.id == id).ok_or(DeformError::MissingBone(id))
}
pub fn find_pin(mesh: &DeformMesh, id: u64) -> Result<&MeshPin, DeformError> {
    mesh.pins.iter().find(|pin| pin.id == id).ok_or(DeformError::MissingPin(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(angle: f64) -> DeformRequest {
        DeformRequest { spec: DeformSpec { neutral_position: Vec2 { x: 100.0, y: 50.0 },
            neutral_angle_degrees: 0.0, gain_per_degree: Vec2 { x: 0.5, y: -0.25 } },
            bone_angle: BoneAngle { keyed_degrees: angle, spring_swing_degrees: Some(4.0) }, include_spring_swing: false }
    }
    fn bone(id: u64, x: f64) -> BonePose {
        BonePose { id, world: BoneTransform { translation: Vec2 { x, y: 0.0 }, rotation_degrees: 0.0 } }
    }
    #[test] fn neutral_pose_is_identity() {
        assert_eq!(evaluate_deformation(request(0.0)).unwrap().target_position, Vec2 { x: 100.0, y: 50.0 });
    }
    #[test] fn positive_and_negative_rotation_are_deterministic() {
        assert_eq!(evaluate_deformation(request(20.0)).unwrap().target_position, Vec2 { x: 110.0, y: 45.0 });
        assert_eq!(evaluate_deformation(request(-20.0)).unwrap().target_position, Vec2 { x: 90.0, y: 55.0 });
    }
    #[test] fn spring_swing_requires_explicit_opt_in() {
        let mut r = request(10.0);
        assert_eq!(evaluate_deformation(r).unwrap().effective_angle_degrees, 10.0);
        r.include_spring_swing = true;
        assert_eq!(evaluate_deformation(r).unwrap().effective_angle_degrees, 14.0);
        r.bone_angle.spring_swing_degrees = None;
        assert_eq!(evaluate_deformation(r), Err(DeformError::MissingSpringSwing));
    }
    #[test] fn bone_transform_rotates_then_translates() {
        let t = BoneTransform { translation: Vec2 { x: 10.0, y: 5.0 }, rotation_degrees: 90.0 };
        let p = t.transform_point(Vec2 { x: 2.0, y: 0.0 }).unwrap();
        assert!((p.x - 10.0).abs() < 1e-10); assert!((p.y - 7.0).abs() < 1e-10);
    }
    #[test] fn mesh_uses_weighted_bone_transforms() {
        let mesh = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2 { x: 2.0, y: 0.0 },
            influences: vec![BoneWeight { bone_id: 1, weight: 0.5 }, BoneWeight { bone_id: 2, weight: 0.5 }] }],
            pins: vec![MeshPin { id: 9, vertex_index: 0, target: Vec2 { x: 1.0, y: 1.0 } }] };
        let result = deform_mesh(&mesh, &[bone(1, 0.0), bone(2, 2.0)]).unwrap();
        assert_eq!(result.vertices[0], Vec2 { x: 3.0, y: 0.0 });
        assert_eq!(find_pin(&mesh, 9).unwrap().vertex_index, 0);
    }
    #[test] fn missing_bone_and_invalid_weights_are_errors() {
        let mesh = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(),
            influences: vec![BoneWeight { bone_id: 7, weight: 1.0 }] }], pins: vec![] };
        assert_eq!(deform_mesh(&mesh, &[]), Err(DeformError::MissingInfluenceBone(7)));
        let bad = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(),
            influences: vec![BoneWeight { bone_id: 7, weight: 0.5 }] }], pins: vec![] };
        assert_eq!(deform_mesh(&bad, &[bone(7, 0.0)]), Err(DeformError::InvalidWeightSum));
    }
    #[test] fn non_finite_and_missing_spring_are_explicit() {
        assert_eq!(evaluate_deformation(request(f64::NAN)), Err(DeformError::NonFiniteInput));
    }
    #[test] fn diagnostics_report_counts_mode_and_geometry_warnings() {
        let mesh = DeformMesh {
            vertices: vec![
                MeshVertex { bind_position: Vec2 { x: 0.0, y: 0.0 }, influences: vec![BoneWeight { bone_id: 1, weight: 1.0 }] },
                MeshVertex { bind_position: Vec2 { x: 1.0, y: 0.0 }, influences: vec![BoneWeight { bone_id: 1, weight: 1.0 }] },
                MeshVertex { bind_position: Vec2 { x: 2.0, y: 0.0 }, influences: vec![BoneWeight { bone_id: 1, weight: 1.0 }] },
            ], pins: vec![],
        };
        let segments = [BoneSegment { bone_id: 1, start: Vec2::default(), end: Vec2::default() }];
        let options = DeformDiagnosticsOptions { collect_metrics: true, triangles: &[[0, 1, 2]], bone_segments: &segments };
        let output = deform_mesh_with_diagnostics(&mesh, &[bone(1, 0.0)], options).unwrap();
        let metrics = output.diagnostics.metrics.unwrap();
        assert_eq!(metrics.vertex_count, 3);
        assert_eq!(metrics.bone_count, 1);
        assert_eq!(metrics.interpolation_mode, InterpolationMode::LinearBlendSkinning);
        assert_eq!(metrics.solver, None);
        assert_eq!(output.diagnostics.warnings.len(), 2);
        assert!(matches!(output.diagnostics.warnings[0], DeformWarning::DegenerateTriangle { .. }));
        assert!(matches!(output.diagnostics.warnings[1], DeformWarning::ZeroLengthBone { bone_id: 1 }));
    }
    #[test] fn structured_error_exposes_machine_readable_code() {
        let mesh = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(), influences: vec![] }], pins: vec![] };
        let error = deform_mesh_with_diagnostics(&mesh, &[], DeformDiagnosticsOptions::default()).unwrap_err();
        assert_eq!(error.code, DeformErrorCode::MissingInfluences);
        assert_eq!(error.source, DeformError::MissingInfluences);
    }
    #[test]
    fn oversized_counts_are_rejected_before_output_allocation() {
        assert_eq!(
            check_limit(DeformResource::Vertices, MAX_MESH_VERTICES + 1, MAX_MESH_VERTICES),
            Err(DeformError::ResourceLimitExceeded {
                resource: DeformResource::Vertices,
                actual: MAX_MESH_VERTICES + 1,
                limit: MAX_MESH_VERTICES,
            })
        );
    }
    #[test]
    fn duplicate_bone_and_pin_ids_fail_closed() {
        let mesh = DeformMesh {
            vertices: vec![MeshVertex {
                bind_position: Vec2::default(),
                influences: vec![BoneWeight { bone_id: 1, weight: 1.0 }],
            }],
            pins: vec![
                MeshPin { id: 7, vertex_index: 0, target: Vec2::default() },
                MeshPin { id: 7, vertex_index: 0, target: Vec2::default() },
            ],
        };
        assert_eq!(deform_mesh(&mesh, &[bone(1, 0.0)]), Err(DeformError::DuplicatePin(7)));
        let valid_mesh = DeformMesh { pins: vec![], ..mesh };
        assert_eq!(deform_mesh(&valid_mesh, &[bone(1, 0.0), bone(1, 1.0)]), Err(DeformError::DuplicateBone(1)));
    }

    #[test] fn metrics_are_absent_when_not_requested() {
        let mesh = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(), influences: vec![BoneWeight { bone_id: 1, weight: 1.0 }] }], pins: vec![] };
        let output = deform_mesh_with_diagnostics(&mesh, &[bone(1, 0.0)], DeformDiagnosticsOptions::default()).unwrap();
        assert_eq!(output.diagnostics.metrics, None);
    }
}
