use smart_controller_core::{
    canonical::{BindingMode, Controller, ControllerBinding, MappingConfig, TargetMapping},
    contribution::CompositionMode,
    evaluator::{evaluate, EvaluationResult},
    graph::ControllerGraph,
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{FallbackPolicy, InputDefinition, InputDimension, InputSample, InputSnapshot, InputSource},
    mapping::CanonicalControllerMapper,
    pose::Pose2D,
    value::TypedValue,
    CoreError,
};

fn binding(id: &str, target: TargetMapping, mode: BindingMode) -> ControllerBinding {
    ControllerBinding {
        id: BindingId::new(id).unwrap(),
        target_id: TargetId::new(format!("target-{id}")).unwrap(),
        target,
        mode,
        base_value: None,
        composition: None,
    }
}

fn property_binding(id: &str, scale: f64) -> ControllerBinding {
    binding(
        id,
        TargetMapping::Property {
            target_id: format!("property-{id}"),
            property: "value".into(),
            scale,
            additive: false,
        },
        BindingMode::AbsolutePose,
    )
}

fn controller(mapping: MappingConfig, bindings: Vec<ControllerBinding>, inputs: Vec<&str>) -> Controller {
    Controller {
        id: ControllerId::new("controller").unwrap(),
        enabled: true,
        inputs: inputs.into_iter().map(|id| InputId::new(id).unwrap()).collect(),
        dependencies: vec![],
        mapping,
        bindings,
        parent_scoped: false,
    }
}

fn run(
    controller: Controller,
    input: TypedValue,
    dimension: InputDimension,
    mapper: &CanonicalControllerMapper,
) -> Result<EvaluationResult, CoreError> {
    let input_ids = controller.inputs.clone();
    let graph = ControllerGraph::new(vec![controller])?;
    let definitions = input_ids.iter().cloned().map(|id| InputDefinition {
        id,
        source: InputSource::Manual,
        dimension,
        fallback: FallbackPolicy::Reject,
    }).collect::<Vec<_>>();
    let samples = input_ids.into_iter()
        .map(|id| InputSample::valid(id, input, Some(0)))
        .collect();
    let snapshot = InputSnapshot::new(0, samples)?;
    evaluate(&graph, &definitions, &snapshot, mapper)
}

fn scalar_output(result: &EvaluationResult, binding_id: &str) -> f64 {
    result.contributions.iter()
        .find(|item| item.binding_id.as_str() == binding_id)
        .unwrap()
        .value
        .as_scalar()
        .unwrap()
}

#[test]
fn range1d_maps_negative_neutral_and_positive_geometry_anchors() {
    let controller = controller(
        MappingConfig::Range1D { minimum: -10.0, neutral: 0.0, maximum: 20.0 },
        vec![binding(
            "pin",
            TargetMapping::GeometryDeform {
                element_id: "shape".into(),
                pin_id: "pin".into(),
                negative_offset: [-10.0, 2.0],
                neutral_offset: [1.0, 3.0],
                positive_offset: [9.0, 11.0],
            },
            BindingMode::AbsolutePose,
        )],
        vec!["driver"],
    );
    let mapper = CanonicalControllerMapper::new();

    for (input, expected) in [
        (-10.0, (-10.0, 2.0)),
        (0.0, (1.0, 3.0)),
        (20.0, (9.0, 11.0)),
        (100.0, (9.0, 11.0)),
    ] {
        let result = run(
            controller.clone(),
            TypedValue::scalar(input).unwrap(),
            InputDimension::Scalar,
            &mapper,
        ).unwrap();
        let vector = result.contributions[0].value.as_vector2().unwrap();
        assert!((vector.x() - expected.0).abs() < 1.0e-10);
        assert!((vector.y() - expected.1).abs() < 1.0e-10);
    }
}

#[test]
fn three_anchor_smoothstep_is_symmetric_and_preserves_neutral() {
    let controller = controller(
        MappingConfig::ThreeAnchorRange {
            minimum: -10.0,
            neutral: 0.0,
            maximum: 10.0,
            easing: smart_controller_core::mapping::easing::Easing::Smoothstep,
        },
        vec![property_binding("property", 10.0)],
        vec!["driver"],
    );
    let mapper = CanonicalControllerMapper::new();

    let positive = run(
        controller.clone(),
        TypedValue::scalar(2.5).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    let negative = run(
        controller.clone(),
        TypedValue::scalar(-2.5).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    let neutral = run(
        controller,
        TypedValue::scalar(0.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();

    assert!((scalar_output(&positive, "property") - 1.5625).abs() < 1.0e-10);
    assert!((scalar_output(&negative, "property") + 1.5625).abs() < 1.0e-10);
    assert_eq!(scalar_output(&neutral, "property"), 0.0);
}

#[test]
fn directed_range_preserves_reversed_endpoints_and_frame_endpoint_policy() {
    let controller = controller(
        MappingConfig::DirectedRange1D { start: 10.0, end: -10.0 },
        vec![binding(
            "frame",
            TargetMapping::FrameRange {
                action_layer_id: 1,
                start_frame: 5,
                end_frame: 15,
                endpoint_policy: Some(smart_controller_core::EndpointPolicy::Inclusive),
            },
            BindingMode::AbsolutePose,
        )],
        vec!["driver"],
    );
    let mapper = CanonicalControllerMapper::new();

    let start = run(controller.clone(), TypedValue::scalar(10.0).unwrap(), InputDimension::Scalar, &mapper).unwrap();
    let middle = run(controller.clone(), TypedValue::scalar(0.0).unwrap(), InputDimension::Scalar, &mapper).unwrap();
    let end = run(controller, TypedValue::scalar(-10.0).unwrap(), InputDimension::Scalar, &mapper).unwrap();

    assert_eq!(scalar_output(&start, "frame"), 5.0);
    assert_eq!(scalar_output(&middle, "frame"), 10.0);
    assert_eq!(scalar_output(&end, "frame"), 15.0);
}

#[test]
fn discrete_mapping_selects_nearest_authored_value_and_preserves_type() {
    let scalar_controller = Controller {
        bindings: vec![property_binding("selected", 1.0)],
        mapping: MappingConfig::Discrete {
            values: vec![
                TypedValue::scalar(2.0).unwrap(),
                TypedValue::angle(0.5, true).unwrap(),
                TypedValue::scalar(8.0).unwrap(),
            ],
        },
        ..controller
    };
    let mapper = CanonicalControllerMapper::new();

    let first = run(
        scalar_controller.clone(),
        TypedValue::scalar(-1.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    assert_eq!(scalar_output(&first, "selected"), 2.0);

    let middle = run(
        scalar_controller.clone(),
        TypedValue::scalar(0.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    let middle_value = middle.contributions[0].value.as_angle().unwrap();
    assert_eq!(middle_value.radians(), 0.5);
    assert!(middle_value.is_wrapped());

    let last = run(
        scalar_controller,
        TypedValue::scalar(1.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    assert_eq!(scalar_output(&last, "selected"), 8.0);
}

#[test]
fn pose_set_interpolates_by_binding_identity_not_authored_order() {
    let mut mapper = CanonicalControllerMapper::new();
    let first = Pose2D::new(vec![
        (BindingId::new("a").unwrap(), TypedValue::scalar(0.0).unwrap()),
        (BindingId::new("b").unwrap(), TypedValue::scalar(10.0).unwrap()),
    ]).unwrap();
    let second = Pose2D::new(vec![
        (BindingId::new("b").unwrap(), TypedValue::scalar(30.0).unwrap()),
        (BindingId::new("a").unwrap(), TypedValue::scalar(20.0).unwrap()),
    ]).unwrap();
    mapper.register_pose("first", first).unwrap();
    mapper.register_pose("second", second).unwrap();

    let controller = controller(
        MappingConfig::PoseSet1D { pose_ids: vec!["first".into(), "second".into()] },
        vec![property_binding("a", 1.0), property_binding("b", 1.0)],
        vec!["driver"],
    );
    let result = run(
        controller,
        TypedValue::scalar(0.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();

    assert_eq!(scalar_output(&result, "a"), 10.0);
    assert_eq!(scalar_output(&result, "b"), 20.0);
}

#[test]
fn relative_geometry_mapping_returns_offset_from_neutral_anchor() {
    let mut relative = binding(
        "pin",
        TargetMapping::GeometryDeform {
            element_id: "shape".into(),
            pin_id: "pin".into(),
            negative_offset: [-10.0, 1.0],
            neutral_offset: [2.0, 3.0],
            positive_offset: [10.0, 7.0],
        },
        BindingMode::RelativeDelta,
    );
    relative.base_value = Some(TypedValue::vector2(2.0, 3.0).unwrap());
    let controller = controller(
        MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
        vec![relative],
        vec!["driver"],
    );
    let result = run(
        controller,
        TypedValue::scalar(1.0).unwrap(),
        InputDimension::Scalar,
        &CanonicalControllerMapper::new(),
    ).unwrap();
    let delta = result.contributions[0].value.as_vector2().unwrap();
    assert_eq!((delta.x(), delta.y()), (8.0, 4.0));
    assert_eq!(result.contributions[0].mode, CompositionMode::Additive);
}

#[test]
fn angle_driver_keeps_angle_output_semantics_for_property_targets() {
    let controller = controller(
        MappingConfig::Range1D {
            minimum: -std::f64::consts::PI,
            neutral: 0.0,
            maximum: std::f64::consts::PI,
        },
        vec![property_binding("rotation", 2.0)],
        vec!["driver"],
    );
    let result = run(
        controller,
        TypedValue::angle(std::f64::consts::FRAC_PI_2, true).unwrap(),
        InputDimension::Angle,
        &CanonicalControllerMapper::new(),
    ).unwrap();
    let angle = result.contributions[0].value.as_angle().unwrap();
    assert!((angle.radians() - 1.0).abs() < 1.0e-10);
    assert!(angle.is_wrapped());
}

#[test]
fn pose_library_rejects_duplicate_and_empty_ids() {
    let pose = Pose2D::new(vec![(BindingId::new("a").unwrap(), TypedValue::scalar(0.0).unwrap())]).unwrap();
    let mut mapper = CanonicalControllerMapper::new();
    assert!(mapper.register_pose(" ", pose.clone()).is_err());
    mapper.register_pose("neutral", pose.clone()).unwrap();
    assert!(mapper.register_pose("neutral", pose).is_err());
}

#[test]
fn mapper_rejects_missing_pose_references_and_unsupported_joystick_explicitly() {
    let mapper = CanonicalControllerMapper::new();
    let pose_controller = controller(
        MappingConfig::PoseSet1D { pose_ids: vec!["not-registered".into()] },
        vec![property_binding("a", 1.0)],
        vec!["driver"],
    );
    let missing = run(
        pose_controller,
        TypedValue::scalar(0.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    assert!(missing.contributions.is_empty());
    assert!(missing.diagnostics.iter().any(|item| item.code == "SC_MAPPING_DOMAIN"));

    let joystick = controller(
        MappingConfig::JoystickFivePose {
            center: "center".into(),
            up: "up".into(),
            right: "right".into(),
            down: "down".into(),
            left: "left".into(),
        },
        vec![property_binding("a", 1.0)],
        vec!["driver"],
    );
    let result = run(
        joystick,
        TypedValue::vector2(0.0, 0.0).unwrap(),
        InputDimension::Vector2,
        &mapper,
    ).unwrap();
    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| item.code == "SC_MAPPING_DOMAIN"));
}

#[test]
fn mapper_rejects_ambiguous_multi_input_controller() {
    let controller = controller(
        MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
        vec![property_binding("a", 1.0)],
        vec!["driver", "secondary"],
    );
    let result = run(
        controller,
        TypedValue::scalar(0.5).unwrap(),
        InputDimension::Scalar,
        &CanonicalControllerMapper::new(),
    ).unwrap();
    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| item.code == "SC_MAPPING_DOMAIN"));
}

#[test]
fn pose_set_requires_exact_controller_binding_set() {
    let mut mapper = CanonicalControllerMapper::new();
    let pose = Pose2D::new(vec![(BindingId::new("extra").unwrap(), TypedValue::scalar(1.0).unwrap())]).unwrap();
    mapper.register_pose("pose", pose).unwrap();
    let controller = controller(
        MappingConfig::PoseSet1D { pose_ids: vec!["pose".into()] },
        vec![property_binding("expected", 1.0)],
        vec!["driver"],
    );
    let result = run(
        controller,
        TypedValue::scalar(0.0).unwrap(),
        InputDimension::Scalar,
        &mapper,
    ).unwrap();
    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| item.code == "SC_MAPPING_DOMAIN"));
}
