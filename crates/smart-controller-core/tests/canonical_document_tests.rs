use smart_controller_core::{
    canonical::{
        BindingMode, CompositionConfig, Controller, ControllerBinding, MappingConfig, TargetMapping,
    },
    contribution::{BaseRef, CompositionMode},
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{FallbackPolicy, InputDefinition, InputDimension, InputSource},
    migration::CanonicalDocument,
    schema::{SchemaIdentity, CURRENT_SCHEMA_VERSION, SCHEMA_NAME},
    CoreError,
};

fn controller(
    id: &str,
    input_id: &str,
    dependencies: Vec<ControllerId>,
    bindings: Vec<ControllerBinding>,
) -> Controller {
    Controller {
        id: ControllerId::new(id).unwrap(),
        enabled: true,
        inputs: vec![InputId::new(input_id).unwrap()],
        dependencies,
        mapping: MappingConfig::Range1D {
            minimum: -1.0,
            neutral: 0.0,
            maximum: 1.0,
        },
        bindings,
        parent_scoped: false,
    }
}

fn producer_binding() -> ControllerBinding {
    ControllerBinding {
        id: BindingId::new("source-binding").unwrap(),
        target_id: TargetId::new("source-target").unwrap(),
        target: TargetMapping::Property {
            target_id: "arm".into(),
            property: "rotation".into(),
            scale: 1.0,
            additive: false,
        },
        mode: BindingMode::AbsolutePose,
        base_value: None,
        composition: None,
    }
}

fn document(with_dependency: bool, source_binding_exists: bool) -> CanonicalDocument {
    let producer_id = ControllerId::new("source-controller").unwrap();
    let input_id = InputId::new("derived-input").unwrap();
    let input = InputDefinition {
        id: input_id,
        source: InputSource::ControllerOutput {
            controller_id: producer_id.clone(),
            binding_id: BindingId::new("source-binding").unwrap(),
        },
        dimension: InputDimension::Scalar,
        fallback: FallbackPolicy::Reject,
    };
    let producer = controller(
        "source-controller",
        "producer-input",
        vec![],
        if source_binding_exists {
            vec![producer_binding()]
        } else {
            vec![]
        },
    );
    let consumer = controller(
        "consumer-controller",
        "derived-input",
        if with_dependency {
            vec![producer_id]
        } else {
            vec![]
        },
        vec![],
    );
    CanonicalDocument {
        schema: SchemaIdentity {
            name: SCHEMA_NAME.into(),
            version: CURRENT_SCHEMA_VERSION,
        },
        inputs: vec![
            InputDefinition {
                id: InputId::new("producer-input").unwrap(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
            input,
        ],
        controllers: vec![producer, consumer],
    }
}

#[test]
fn controller_output_inputs_require_declared_producer_dependencies() {
    assert!(document(true, true).validate().is_ok());
    assert!(document(false, true).validate().is_err());
}

#[test]
fn controller_output_inputs_must_reference_an_existing_binding() {
    assert!(document(true, false).validate().is_err());
}

#[test]
fn explicit_binding_composition_configuration_is_validated_by_mode() {
    let mut weighted = controller("weighted", "input", vec![], vec![producer_binding()]);
    weighted.bindings[0].composition = Some(CompositionConfig {
        mode: CompositionMode::WeightedBlend,
        weight: Some(0.75),
        priority: None,
        base_ref: None,
    });
    weighted.validate_local().unwrap();

    let mut priority = controller("priority", "input", vec![], vec![producer_binding()]);
    priority.bindings[0].composition = Some(CompositionConfig {
        mode: CompositionMode::PriorityOverride,
        weight: None,
        priority: Some(10),
        base_ref: None,
    });
    priority.validate_local().unwrap();

    let mut neutral_additive = controller("additive", "input", vec![], vec![producer_binding()]);
    neutral_additive.bindings[0].mode = BindingMode::RelativeDelta;
    neutral_additive.bindings[0].target = TargetMapping::Property {
        target_id: "arm".into(),
        property: "rotation".into(),
        scale: 1.0,
        additive: true,
    };
    neutral_additive.bindings[0].composition = Some(CompositionConfig {
        mode: CompositionMode::Additive,
        weight: None,
        priority: None,
        base_ref: Some(BaseRef::Neutral),
    });
    neutral_additive.validate_local().unwrap();

    weighted.bindings[0].composition.as_mut().unwrap().weight = Some(f64::NAN);
    assert!(weighted.validate_local().is_err());

    priority.bindings[0].composition.as_mut().unwrap().priority = None;
    assert!(priority.validate_local().is_err());

    neutral_additive.bindings[0]
        .composition
        .as_mut()
        .unwrap()
        .base_ref = None;
    assert!(neutral_additive.validate_local().is_err());
}

#[test]
fn optional_composition_config_round_trips_and_old_documents_still_deserialize() {
    let mut current = document(true, true);
    current.controllers[0].bindings[0].composition = Some(CompositionConfig {
        mode: CompositionMode::WeightedBlend,
        weight: Some(0.5),
        priority: None,
        base_ref: None,
    });
    let json = current.to_json().unwrap();
    let decoded = CanonicalDocument::from_json(&json).unwrap();
    assert_eq!(decoded, current);

    let mut old_json: serde_json::Value = serde_json::from_str(&json).unwrap();
    old_json["controllers"][0]["bindings"][0]
        .as_object_mut()
        .unwrap()
        .remove("composition");
    let decoded_old =
        CanonicalDocument::from_json(&serde_json::to_string(&old_json).unwrap()).unwrap();
    assert!(decoded_old.controllers[0].bindings[0].composition.is_none());
}


#[test]
fn validate_local_preserves_controller_reference_error_precedence() {
    let mut controller = controller("ordered", "input", vec![], vec![]);
    controller.inputs.clear();
    controller.dependencies.push(controller.id.clone());
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidModel(
            "controller ordered has no inputs".into()
        ))
    );

    controller.inputs.push(InputId::new("input").unwrap());
    controller.dependencies = vec![controller.id.clone(), controller.id.clone()];
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidGraph(
            "controller ordered depends on itself".into()
        ))
    );

    controller.dependencies = vec![
        ControllerId::new("other").unwrap(),
        ControllerId::new("other").unwrap(),
    ];
    controller.inputs.push(InputId::new("input").unwrap());
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidGraph(
            "controller ordered contains duplicate dependency edges".into()
        ))
    );

    controller.dependencies.clear();
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidModel(
            "controller ordered contains duplicate input references".into()
        ))
    );
}

#[test]
fn validate_local_checks_binding_errors_before_mapping_errors() {
    let mut controller = controller("ordered", "input", vec![], vec![producer_binding()]);
    controller.mapping = MappingConfig::Range1D {
        minimum: 0.0,
        neutral: 0.0,
        maximum: 0.0,
    };
    controller.bindings[0].target = TargetMapping::FrameRange {
        action_layer_id: 0,
        start_frame: 5,
        end_frame: 4,
        endpoint_policy: None,
    };

    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidModel(
            "frame range requires a nonzero layer and increasing endpoints".into()
        ))
    );
}

#[test]
fn validate_local_preserves_binding_and_composition_error_precedence() {
    let mut first = producer_binding();
    let mut duplicate = producer_binding();
    duplicate.composition = Some(CompositionConfig {
        mode: CompositionMode::WeightedBlend,
        weight: Some(f64::NAN),
        priority: None,
        base_ref: None,
    });
    let mut controller = controller("ordered", "input", vec![], vec![first.clone(), duplicate]);
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::InvalidModel(
            "duplicate binding ID source-binding".into()
        ))
    );

    first.composition = Some(CompositionConfig {
        mode: CompositionMode::WeightedBlend,
        weight: Some(f64::NAN),
        priority: None,
        base_ref: Some(BaseRef::Input(" ".into())),
    });
    controller.bindings = vec![first];
    assert_eq!(
        controller.validate_local(),
        Err(CoreError::CompositionWeightInvalid {
            binding_id: "source-binding".into()
        })
    );
}
