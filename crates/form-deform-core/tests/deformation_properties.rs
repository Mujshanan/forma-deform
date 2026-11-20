use form_deform_core::{
    deform_mesh, evaluate_deformation, BoneAngle, BonePose, BoneTransform, BoneWeight,
    DeformError, DeformMesh, DeformRequest, DeformSpec, MeshPin, MeshVertex, Vec2,
};
use proptest::prelude::*;

fn pose(id: u64, x: f64, y: f64, angle: f64) -> BonePose {
    BonePose {
        id,
        world: BoneTransform {
            translation: Vec2 { x, y },
            rotation_degrees: angle,
        },
    }
}

fn vertex(x: f64, y: f64, weights: Vec<BoneWeight>) -> MeshVertex {
    MeshVertex {
        bind_position: Vec2 { x, y },
        influences: weights,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn deformation_is_deterministic(
        x in -1.0e6f64..1.0e6,
        y in -1.0e6f64..1.0e6,
        angle in -1.0e6f64..1.0e6,
        gain_x in -1.0e3f64..1.0e3,
        gain_y in -1.0e3f64..1.0e3,
    ) {
        let request = DeformRequest {
            spec: DeformSpec {
                neutral_position: Vec2 { x: 20.0, y: -30.0 },
                neutral_angle_degrees: 0.0,
                gain_per_degree: Vec2 { x: gain_x, y: gain_y },
            },
            bone_angle: BoneAngle { keyed_degrees: angle, spring_swing_degrees: None },
            include_spring_swing: false,
        };
        let first = evaluate_deformation(request);
        let second = evaluate_deformation(request);
        prop_assert_eq!(first, second);
        prop_assert!(x.is_finite() && y.is_finite());
    }

    #[test]
    fn weighted_mesh_results_are_finite_for_finite_bounded_inputs(
        x in -1.0e6f64..1.0e6,
        y in -1.0e6f64..1.0e6,
        tx in -1.0e6f64..1.0e6,
        ty in -1.0e6f64..1.0e6,
        angle in -3600.0f64..3600.0,
        weight in 0.0f64..=1.0,
    ) {
        let mesh = DeformMesh {
            vertices: vec![vertex(x, y, vec![
                BoneWeight { bone_id: 1, weight },
                BoneWeight { bone_id: 2, weight: 1.0 - weight },
            ])],
            pins: vec![],
        };
        let bones = [pose(1, tx, ty, angle), pose(2, -tx, -ty, -angle)];
        let result = deform_mesh(&mesh, &bones).unwrap();
        prop_assert!(result.vertices.iter().all(|p| p.is_finite()));
    }

    #[test]
    fn identity_bone_transform_preserves_every_vertex(
        x in -1.0e9f64..1.0e9,
        y in -1.0e9f64..1.0e9,
    ) {
        let mesh = DeformMesh {
            vertices: vec![vertex(x, y, vec![BoneWeight { bone_id: 7, weight: 1.0 }])],
            pins: vec![],
        };
        let result = deform_mesh(&mesh, &[pose(7, 0.0, 0.0, 0.0)]).unwrap();
        prop_assert_eq!(result.vertices[0], Vec2 { x, y });
    }

    #[test]
    fn extreme_finite_values_never_return_non_finite_success(
        x in -1.0e150f64..1.0e150,
        angle in -1.0e150f64..1.0e150,
        gain in -1.0e150f64..1.0e150,
    ) {
        let request = DeformRequest {
            spec: DeformSpec {
                neutral_position: Vec2 { x, y: 0.0 },
                neutral_angle_degrees: 0.0,
                gain_per_degree: Vec2 { x: gain, y: 0.0 },
            },
            bone_angle: BoneAngle { keyed_degrees: angle, spring_swing_degrees: None },
            include_spring_swing: false,
        };
        if let Ok(result) = evaluate_deformation(request) {
            prop_assert!(result.target_position.is_finite());
            prop_assert!(result.effective_angle_degrees.is_finite());
            prop_assert!(result.angle_delta_degrees.is_finite());
        }
    }
}

#[test]
fn mesh_identity_preserves_closed_vertex_loop_and_pin_indices() {
    // Closure is represented by repeating the first vertex at the end of the vertex list.
    let vertices = [
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 4.0, y: 0.0 },
        Vec2 { x: 4.0, y: 3.0 },
        Vec2 { x: 0.0, y: 3.0 },
        Vec2 { x: 0.0, y: 0.0 },
    ];
    let mesh = DeformMesh {
        vertices: vertices.iter().map(|p| vertex(
            p.x, p.y, vec![BoneWeight { bone_id: 1, weight: 1.0 }]
        )).collect(),
        pins: vec![MeshPin {
            id: 99,
            vertex_index: 2,
            target: Vec2 { x: 4.0, y: 3.0 },
        }],
    };
    let result = deform_mesh(&mesh, &[pose(1, 0.0, 0.0, 0.0)]).unwrap();
    assert_eq!(result.vertices.first(), result.vertices.last());
    assert_eq!(result.pins[0].vertex_index, 2);
}

#[test]
fn malformed_weights_and_missing_bones_are_rejected() {
    let bad_weight = DeformMesh {
        vertices: vec![vertex(0.0, 0.0, vec![BoneWeight { bone_id: 1, weight: f64::NAN }])],
        pins: vec![],
    };
    assert_eq!(deform_mesh(&bad_weight, &[pose(1, 0.0, 0.0, 0.0)]), Err(DeformError::InvalidWeight));

    let missing_bone = DeformMesh {
        vertices: vec![vertex(0.0, 0.0, vec![BoneWeight { bone_id: 2, weight: 1.0 }])],
        pins: vec![],
    };
    assert_eq!(deform_mesh(&missing_bone, &[pose(1, 0.0, 0.0, 0.0)]), Err(DeformError::MissingInfluenceBone(2)));
}

#[test]
fn arithmetic_overflow_is_an_error_not_a_non_finite_result() {
    let mesh = DeformMesh {
        vertices: vec![vertex(
            f64::MAX,
            f64::MAX,
            vec![BoneWeight { bone_id: 1, weight: 1.0 }],
        )],
        pins: vec![],
    };
    assert_eq!(
        deform_mesh(&mesh, &[pose(1, f64::MAX, f64::MAX, 0.0)]),
        Err(DeformError::ArithmeticOverflow)
    );
}
