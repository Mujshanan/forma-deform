use std::{cell::Cell, collections::BTreeMap};

use smart_controller_core::{
    canonical::{
        BindingMode, Controller, ControllerBinding, EasingConfig, MappingConfig, TargetMapping,
    },
    evaluator::{evaluate, ControllerMapper},
    graph::ControllerGraph,
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{
        FallbackPolicy, InputDefinition, InputDimension, InputSample, InputSnapshot, InputSource,
    },
    mapping::advanced::{
        AdvancedControllerMapper, AdvancedMappingRegistry,
        grid::{GridBoundaryPolicy, GridMapping},
        triangulated::{MeshBoundaryPolicy, Triangle, TriangulatedMapping, TriangulatedPose},
    },
    pose::Pose2D,
    CoreError, TypedValue, Vector2,
};

fn id_input(value: &str) -> InputId { InputId::new(value).unwrap() }
fn id_controller(value: &str) -> ControllerId { ControllerId::new(value).unwrap() }
fn id_binding(value: &str) -> BindingId { BindingId::new(value).unwrap() }

fn pose(value: f64) -> Pose2D {
    Pose2D::new(vec![(id_binding("out"), TypedValue::scalar(value).unwrap())]).unwrap()
}

fn grid() -> GridMapping {
    GridMapping::new(
        vec![0.0, 1.0],
        vec![0.0, 1.0],
        vec![pose(0.0), pose(10.0), pose(20.0), pose(30.0)],
        GridBoundaryPolicy::Reject,
    ).unwrap()
}

fn controller(input_id: &str) -> Controller {
    Controller {
        id: id_controller("controller"),
        enabled: true,
        inputs: vec![id_input(input_id)],
        dependencies: vec![],
        mapping: MappingConfig::ThreeAnchorRange {
            minimum: -1.0,
            neutral: 0.0,
            maximum: 1.0,
            easing: EasingConfig::Linear,
        },
        bindings: vec![ControllerBinding {
            id: id_binding("out"),
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

fn definition(input_id: &str, dimension: InputDimension) -> InputDefinition {
    InputDefinition {
        id: id_input(input_id),
        source: InputSource::Manual,
        dimension,
        fallback: FallbackPolicy::Reject,
    }
}

fn vector_sample(input_id: &str, x: f64, y: f64) -> InputSample {
    InputSample::valid(
        id_input(input_id),
        TypedValue::vector2(x, y).unwrap(),
        Some(0),
    )
}

fn scalar_sample(input_id: &str, value: f64) -> InputSample {
    InputSample::valid(
        id_input(input_id),
        TypedValue::scalar(value).unwrap(),
        Some(0),
    )
}

fn run<M: ControllerMapper>(
    controller: Controller,
    definition: InputDefinition,
    sample: InputSample,
    mapper: &M,
) -> smart_controller_core::evaluator::EvaluationResult {
    let graph = ControllerGraph::new(vec![controller]).unwrap();
    let snapshot = InputSnapshot::new(0, vec![sample]).unwrap();
    evaluate(&graph, &[definition], &snapshot, mapper).unwrap()
}

#[test]
fn registered_grid_mapping_runs_through_controller_mapper_and_evaluator() {
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("driver"), grid()).unwrap();

    let fallback = |_: &Controller, _| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        Err(CoreError::InvalidMapping("fallback must not run for a registered controller".into()))
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Vector2),
        vector_sample("driver", 0.5, 0.5),
        &mapper,
    );

    assert_eq!(result.contributions.len(), 1);
    assert_eq!(result.contributions[0].value.as_scalar(), Some(15.0));
    assert!(result.diagnostics.is_empty());
}

#[test]
fn registered_triangulated_mapping_runs_through_controller_mapper_and_evaluator() {
    let mapping = TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(Vector2::new(0.0, 0.0).unwrap(), pose(0.0)),
            TriangulatedPose::new(Vector2::new(2.0, 0.0).unwrap(), pose(10.0)),
            TriangulatedPose::new(Vector2::new(0.0, 2.0).unwrap(), pose(20.0)),
        ],
        vec![Triangle::new(0, 1, 2)],
        MeshBoundaryPolicy::Reject,
    ).unwrap();
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_triangulated(
        id_controller("controller"),
        id_input("driver"),
        mapping,
    ).unwrap();

    let fallback = |_: &Controller, _| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        Err(CoreError::InvalidMapping("fallback must not run for a registered controller".into()))
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Vector2),
        vector_sample("driver", 0.5, 0.5),
        &mapper,
    );

    assert_eq!(result.contributions.len(), 1);
    assert_eq!(result.contributions[0].value.as_scalar(), Some(7.5));
    assert!(result.diagnostics.is_empty());
}

#[test]
fn unregistered_controller_is_delegated_to_existing_mapper_unchanged() {
    let fallback = |controller: &Controller, _| {
        Ok(BTreeMap::from([(
            controller.bindings[0].id.clone(),
            TypedValue::scalar(42.0)?,
        )]))
    };
    let mapper = AdvancedControllerMapper::new(fallback, AdvancedMappingRegistry::new());

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Scalar),
        scalar_sample("driver", 0.25),
        &mapper,
    );

    assert_eq!(result.contributions.len(), 1);
    assert_eq!(result.contributions[0].value.as_scalar(), Some(42.0));
}

#[test]
fn registered_mapping_failure_does_not_silently_fall_back() {
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("driver"), grid()).unwrap();

    let fallback_calls = Cell::new(0);
    let fallback = |controller: &Controller, _| {
        fallback_calls.set(fallback_calls.get() + 1);
        Ok(BTreeMap::from([(
            controller.bindings[0].id.clone(),
            TypedValue::scalar(99.0)?,
        )]))
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Vector2),
        vector_sample("driver", 5.0, 5.0),
        &mapper,
    );

    assert!(result.contributions.is_empty());
    assert_eq!(fallback_calls.get(), 0);
    assert!(result.diagnostics.iter().any(|item| {
        item.code == smart_controller_core::diagnostic_codes::MAPPING_DOMAIN
    }));
}

#[test]
fn advanced_mapping_requires_its_registered_input_to_be_declared() {
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("other"), grid()).unwrap();
    let fallback = |_: &Controller, _| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        panic!("registered mapping must not delegate to fallback")
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Vector2),
        vector_sample("driver", 0.5, 0.5),
        &mapper,
    );

    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| {
        item.code == smart_controller_core::diagnostic_codes::MAPPING_DOMAIN
    }));
}

#[test]
fn advanced_mapping_rejects_non_vector2_driver_inputs() {
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("driver"), grid()).unwrap();
    let fallback = |_: &Controller, _| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        panic!("registered mapping must not delegate to fallback")
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Scalar),
        scalar_sample("driver", 0.5),
        &mapper,
    );

    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| {
        item.code == smart_controller_core::diagnostic_codes::MAPPING_DOMAIN
    }));
}

#[test]
fn advanced_pose_must_emit_exactly_the_controller_binding_set() {
    let incompatible_pose = Pose2D::new(vec![(
        id_binding("not-the-controller-binding"),
        TypedValue::scalar(7.0).unwrap(),
    )]).unwrap();
    let mapping = GridMapping::new(
        vec![0.0, 1.0],
        vec![0.0, 1.0],
        vec![incompatible_pose.clone(); 4],
        GridBoundaryPolicy::Reject,
    ).unwrap();
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("driver"), mapping).unwrap();
    let fallback = |_: &Controller, _| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        panic!("registered mapping must not delegate to fallback")
    };
    let mapper = AdvancedControllerMapper::new(fallback, registry);

    let result = run(
        controller("driver"),
        definition("driver", InputDimension::Vector2),
        vector_sample("driver", 0.5, 0.5),
        &mapper,
    );

    assert!(result.contributions.is_empty());
    assert!(result.diagnostics.iter().any(|item| {
        item.code == smart_controller_core::diagnostic_codes::MAPPING_DOMAIN
    }));
}

#[test]
fn registry_rejects_duplicate_controller_registration() {
    let mut registry = AdvancedMappingRegistry::new();
    registry.register_grid(id_controller("controller"), id_input("driver"), grid()).unwrap();
    assert!(registry.register_grid(id_controller("controller"), id_input("driver"), grid()).is_err());
}
