use smart_controller_core::{
    canonical::{
        BindingMode, Controller, ControllerBinding, EasingConfig, MappingConfig, TargetMapping,
    },
    ids::{BindingId, ControllerId, InputId, TargetId},
    mapping::advanced::{
        AdvancedMappingExtension, AdvancedMappingSpec, PersistedAdvancedMapping,
        PersistedGridBoundaryPolicy, PersistedMeshBoundaryPolicy, PersistedTriangulatedPose,
        grid::GridBoundaryPolicy,
        triangulated::MeshBoundaryPolicy,
    },
    pose::Pose2D,
    TypedValue, Vector2,
};

fn controller_id(value: &str) -> ControllerId { ControllerId::new(value).unwrap() }
fn input_id(value: &str) -> InputId { InputId::new(value).unwrap() }
fn binding_id(value: &str) -> BindingId { BindingId::new(value).unwrap() }

fn pose(value: f64) -> Pose2D {
    Pose2D::new(vec![(binding_id("out"), TypedValue::scalar(value).unwrap())]).unwrap()
}

fn controller() -> Controller {
    Controller {
        id: controller_id("controller"),
        enabled: true,
        inputs: vec![input_id("driver")],
        dependencies: vec![],
        mapping: MappingConfig::ThreeAnchorRange {
            minimum: -1.0,
            neutral: 0.0,
            maximum: 1.0,
            easing: EasingConfig::Linear,
        },
        bindings: vec![ControllerBinding {
            id: binding_id("out"),
            target_id: TargetId::new("target").unwrap(),
            target: TargetMapping::Property {
                target_id: "property".into(),
                property: "value".into(),
                scale: 1.0,
                additive: false,
            },
            mode: BindingMode::AbsolutePose,
            base_value: None,
            composition: None,
        }],
        parent_scoped: false,
    }
}

fn grid_record() -> PersistedAdvancedMapping {
    PersistedAdvancedMapping {
        controller_id: controller_id("controller"),
        input_id: input_id("driver"),
        mapping: AdvancedMappingSpec::Grid {
            x_coordinates: vec![0.0, 1.0],
            y_coordinates: vec![0.0, 1.0],
            poses: vec![pose(0.0), pose(10.0), pose(20.0), pose(30.0)],
            boundary: PersistedGridBoundaryPolicy::Reject,
        },
    }
}

#[test]
fn extension_json_round_trip_rebuilds_the_runtime_registry() {
    let extension = AdvancedMappingExtension::new(vec![grid_record()]);
    let json = extension.to_json().unwrap();
    let decoded = AdvancedMappingExtension::from_json(&json).unwrap();
    assert_eq!(decoded, extension);

    let registry = decoded.into_runtime_registry(&[controller()]).unwrap();
    assert_eq!(registry.len(), 1);
    assert!(registry.get(&controller_id("controller")).is_some());
}

#[test]
fn extension_identity_and_version_are_checked() {
    let mut wrong_id = AdvancedMappingExtension::new(vec![]);
    wrong_id.extension_id = "unknown.extension".into();
    assert!(wrong_id.validate_identity().is_err());

    let mut wrong_version = AdvancedMappingExtension::new(vec![]);
    wrong_version.version += 1;
    assert!(wrong_version.validate_identity().is_err());
}

#[test]
fn extension_rejects_unknown_controller_references() {
    let mut record = grid_record();
    record.controller_id = controller_id("missing-controller");
    let extension = AdvancedMappingExtension::new(vec![record]);
    assert!(extension.into_runtime_registry(&[controller()]).is_err());
}

#[test]
fn extension_rejects_undeclared_driver_inputs() {
    let mut record = grid_record();
    record.input_id = input_id("not-declared");
    let extension = AdvancedMappingExtension::new(vec![record]);
    assert!(extension.into_runtime_registry(&[controller()]).is_err());
}

#[test]
fn extension_rejects_duplicate_controller_registrations() {
    let extension = AdvancedMappingExtension::new(vec![grid_record(), grid_record()]);
    assert!(extension.into_runtime_registry(&[controller()]).is_err());
}

#[test]
fn extension_rejects_pose_binding_schema_mismatch() {
    let mut record = grid_record();
    if let AdvancedMappingSpec::Grid { poses, .. } = &mut record.mapping {
        *poses = vec![
            Pose2D::new(vec![(binding_id("other"), TypedValue::scalar(1.0).unwrap())]).unwrap();
            4
        ];
    }
    let extension = AdvancedMappingExtension::new(vec![record]);
    assert!(extension.into_runtime_registry(&[controller()]).is_err());
}

#[test]
fn extension_revalidates_grid_data_instead_of_trusting_serialized_values() {
    let mut record = grid_record();
    if let AdvancedMappingSpec::Grid { x_coordinates, .. } = &mut record.mapping {
        *x_coordinates = vec![0.0, 0.0];
    }
    let extension = AdvancedMappingExtension::new(vec![record]);
    assert!(extension.into_runtime_registry(&[controller()]).is_err());
}

#[test]
fn triangulated_mapping_configuration_round_trips_and_loads() {
    let record = PersistedAdvancedMapping {
        controller_id: controller_id("controller"),
        input_id: input_id("driver"),
        mapping: AdvancedMappingSpec::Triangulated {
            vertices: vec![
                PersistedTriangulatedPose {
                    position: Vector2::new(0.0, 0.0).unwrap(),
                    pose: pose(0.0),
                },
                PersistedTriangulatedPose {
                    position: Vector2::new(2.0, 0.0).unwrap(),
                    pose: pose(10.0),
                },
                PersistedTriangulatedPose {
                    position: Vector2::new(0.0, 2.0).unwrap(),
                    pose: pose(20.0),
                },
            ],
            triangles: vec![[0, 1, 2]],
            boundary: PersistedMeshBoundaryPolicy::ClampToMesh,
        },
    };
    let extension = AdvancedMappingExtension::new(vec![record]);
    let json = extension.to_json().unwrap();
    let decoded = AdvancedMappingExtension::from_json(&json).unwrap();
    let registry = decoded.into_runtime_registry(&[controller()]).unwrap();
    assert_eq!(registry.len(), 1);
    assert_eq!(
        match &registry.get(&controller_id("controller")).unwrap().mapping {
            smart_controller_core::mapping::advanced::AdvancedPoseMapping::Triangulated(mapping) =>
                mapping.boundary_policy(),
            _ => panic!("expected triangulated mapping"),
        },
        MeshBoundaryPolicy::ClampToMesh,
    );
    let _ = GridBoundaryPolicy::Clamp;
}
