//! Pure deformation domain: bone transforms, weighted mesh deformation, and pin targets.
//!
//! No host SDK, serialization, JSON, or WASM concerns belong in this crate.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 { pub x: f64, pub y: f64 }

impl Vec2 {
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() }
    pub fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
    pub fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
    pub fn scale(self, factor: f64) -> Self { Self { x: self.x * factor, y: self.y * factor } }
}

/// A 2D rigid bone transform in stage space. Angles are degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneTransform { pub translation: Vec2, pub rotation_degrees: f64 }

impl BoneTransform {
    pub fn transform_point(self, point: Vec2) -> Result<Vec2, DeformError> {
        if !point.is_finite() || !self.translation.is_finite() || !self.rotation_degrees.is_finite() {
            return Err(DeformError::NonFiniteInput);
        }
        let radians = self.rotation_degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        let result = Vec2 {
            x: self.translation.x + point.x * cos - point.y * sin,
            y: self.translation.y + point.x * sin + point.y * cos,
        };
        if result.is_finite() { Ok(result) } else { Err(DeformError::ArithmeticOverflow) }
    }
}

/// One bone's stable identity and evaluated world transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BonePose { pub id: u64, pub world: BoneTransform }

/// One bone influence in a mesh vertex's bind pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneWeight { pub bone_id: u64, pub weight: f64 }

/// A mesh vertex expressed in the bind coordinate system with explicit skin weights.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshVertex { pub bind_position: Vec2, pub influences: Vec<BoneWeight> }

/// A mesh pin refers to a stable vertex index and an authored stage-space target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshPin { pub id: u64, pub vertex_index: usize, pub target: Vec2 }

/// Pure input geometry and evaluated bone poses for one mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct DeformMesh { pub vertices: Vec<MeshVertex>, pub pins: Vec<MeshPin> }

/// Evaluated positions for every vertex and pin in a mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshDeformResult { pub vertices: Vec<Vec2>, pub pins: Vec<MeshPin> }

/// Authored pin-driving parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformSpec {
    pub neutral_position: Vec2,
    pub neutral_angle_degrees: f64,
    pub gain_per_degree: Vec2,
}

/// Evaluated angle supplied by a host adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneAngle { pub keyed_degrees: f64, pub spring_swing_degrees: Option<f64> }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformRequest {
    pub spec: DeformSpec,
    pub bone_angle: BoneAngle,
    pub include_spring_swing: bool,
}

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
    NonFiniteInput,
    MissingSpringSwing,
    MissingBone(u64),
    MissingPin(u64),
    MissingVertex(usize),
    MissingInfluences,
    MissingInfluenceBone(u64),
    InvalidWeight,
    InvalidWeightSum,
    ArithmeticOverflow,
}

impl std::fmt::Display for DeformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            Self::ArithmeticOverflow => write!(f, "deformation result is not finite"),
        }
    }
}
impl std::error::Error for DeformError {}

/// Evaluate one bone-driven absolute pin target from authored neutral data.
pub fn evaluate_deformation(request: DeformRequest) -> Result<DeformResult, DeformError> {
    let spec = request.spec;
    let keyed = request.bone_angle.keyed_degrees;
    let spring = request.bone_angle.spring_swing_degrees;
    if !spec.neutral_position.is_finite() || !spec.gain_per_degree.is_finite()
        || !spec.neutral_angle_degrees.is_finite() || !keyed.is_finite()
        || spring.is_some_and(|value| !value.is_finite()) {
        return Err(DeformError::NonFiniteInput);
    }
    if request.include_spring_swing && spring.is_none() { return Err(DeformError::MissingSpringSwing); }
    let effective = keyed + if request.include_spring_swing { spring.unwrap_or(0.0) } else { 0.0 };
    let delta = effective - spec.neutral_angle_degrees;
    let target = spec.neutral_position.add(spec.gain_per_degree.scale(delta));
    if !effective.is_finite() || !delta.is_finite() || !target.is_finite() { return Err(DeformError::ArithmeticOverflow); }
    Ok(DeformResult { keyed_angle_degrees: keyed, spring_swing_degrees: spring,
        effective_angle_degrees: effective, angle_delta_degrees: delta, target_position: target })
}

/// Apply linear-blend skinning to every mesh vertex using evaluated world-space bone poses.
///
/// Each influence transforms the bind-space vertex by its bone's evaluated transform.
/// Weights must be finite, non-negative, and sum to one (tolerance 1e-6).
/// Pins are validated and returned as explicit authored targets; the caller can apply
/// pin constraints in a subsequent mesh-solver stage without host-specific behavior here.
pub fn deform_mesh(mesh: &DeformMesh, bones: &[BonePose]) -> Result<MeshDeformResult, DeformError> {
    let mut positions = Vec::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        if !vertex.bind_position.is_finite() { return Err(DeformError::NonFiniteInput); }
        if vertex.influences.is_empty() { return Err(DeformError::MissingInfluences); }
        let mut sum = 0.0;
        let mut position = Vec2::default();
        for influence in &vertex.influences {
            if !influence.weight.is_finite() || influence.weight < 0.0 { return Err(DeformError::InvalidWeight); }
            let bone = bones.iter().find(|bone| bone.id == influence.bone_id)
                .ok_or(DeformError::MissingInfluenceBone(influence.bone_id))?;
            let transformed = bone.world.transform_point(vertex.bind_position)?;
            position = position.add(transformed.scale(influence.weight));
            sum += influence.weight;
        }
        if (sum - 1.0).abs() > 1e-6 { return Err(DeformError::InvalidWeightSum); }
        if !position.is_finite() { return Err(DeformError::ArithmeticOverflow); }
        positions.push(position);
    }
    for pin in &mesh.pins {
        if pin.vertex_index >= positions.len() { return Err(DeformError::MissingVertex(pin.vertex_index)); }
        if !pin.target.is_finite() { return Err(DeformError::NonFiniteInput); }
    }
    Ok(MeshDeformResult { vertices: positions, pins: mesh.pins.clone() })
}

/// Resolve a stable bone ID from a typed pose collection.
pub fn find_bone<'a>(bones: &'a [BonePose], id: u64) -> Result<&'a BonePose, DeformError> {
    bones.iter().find(|bone| bone.id == id).ok_or(DeformError::MissingBone(id))
}

/// Resolve a stable pin ID from a typed mesh.
pub fn find_pin(mesh: &DeformMesh, id: u64) -> Result<&MeshPin, DeformError> {
    mesh.pins.iter().find(|pin| pin.id == id).ok_or(DeformError::MissingPin(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(angle: f64) -> DeformRequest {
        DeformRequest { spec: DeformSpec { neutral_position: Vec2 { x: 100.0, y: 50.0 },
            neutral_angle_degrees: 0.0, gain_per_degree: Vec2 { x: 0.5, y: -0.25 } },
            bone_angle: BoneAngle { keyed_degrees: angle, spring_swing_degrees: Some(4.0) },
            include_spring_swing: false }
    }
    #[test] fn neutral_pose_is_identity() {
        assert_eq!(evaluate_deformation(request(0.0)).unwrap().target_position, Vec2 { x: 100.0, y: 50.0 });
    }
    #[test] fn positive_and_negative_rotation_are_deterministic() {
        assert_eq!(evaluate_deformation(request(20.0)).unwrap().target_position, Vec2 { x: 110.0, y: 45.0 });
        assert_eq!(evaluate_deformation(request(-20.0)).unwrap().target_position, Vec2 { x: 90.0, y: 55.0 });
    }
    #[test] fn spring_swing_requires_explicit_opt_in() {
        let mut r = request(10.0); assert_eq!(evaluate_deformation(r).unwrap().effective_angle_degrees, 10.0);
        r.include_spring_swing = true; assert_eq!(evaluate_deformation(r).unwrap().effective_angle_degrees, 14.0);
        r.bone_angle.spring_swing_degrees = None; assert_eq!(evaluate_deformation(r), Err(DeformError::MissingSpringSwing));
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
        let bones = [
            BonePose { id: 1, world: BoneTransform { translation: Vec2::default(), rotation_degrees: 0.0 } },
            BonePose { id: 2, world: BoneTransform { translation: Vec2 { x: 2.0, y: 0.0 }, rotation_degrees: 0.0 } },
        ];
        let result = deform_mesh(&mesh, &bones).unwrap();
        assert_eq!(result.vertices[0], Vec2 { x: 3.0, y: 0.0 });
        assert_eq!(find_pin(&mesh, 9).unwrap().vertex_index, 0);
    }
    #[test] fn missing_bone_and_invalid_weights_are_errors() {
        let mesh = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(),
            influences: vec![BoneWeight { bone_id: 7, weight: 1.0 }] }], pins: vec![] };
        assert_eq!(deform_mesh(&mesh, &[]), Err(DeformError::MissingInfluenceBone(7)));
        let bad = DeformMesh { vertices: vec![MeshVertex { bind_position: Vec2::default(),
            influences: vec![BoneWeight { bone_id: 7, weight: 0.5 }] }], pins: vec![] };
        let bone = BonePose { id: 7, world: BoneTransform { translation: Vec2::default(), rotation_degrees: 0.0 } };
        assert_eq!(deform_mesh(&bad, &[bone]), Err(DeformError::InvalidWeightSum));
    }
    #[test] fn non_finite_and_missing_spring_are_explicit() {
        assert_eq!(evaluate_deformation(request(f64::NAN)), Err(DeformError::NonFiniteInput));
    }
}
