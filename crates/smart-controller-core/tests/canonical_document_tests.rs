use smart_controller_core::{
    canonical::{BindingMode, Controller, ControllerBinding, MappingConfig, TargetMapping},
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{FallbackPolicy, InputDefinition, InputDimension, InputSource},
    migration::CanonicalDocument,
    schema::{SchemaIdentity, CURRENT_SCHEMA_VERSION, SCHEMA_NAME},
};

fn controller(id: &str, input_id: &str, dependencies: Vec<ControllerId>, bindings: Vec<ControllerBinding>) -> Controller {
    Controller {
        id: ControllerId::new(id).unwrap(),
        enabled: true,
        inputs: vec![InputId::new(input_id).unwrap()],
        dependencies,
        mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
        bindings,
        parent_scoped: false,
    }
}

fn producer_binding() -> ControllerBinding {
    ControllerBinding {
        id: BindingId::new("source-binding").unwrap(),
        target_id: TargetId::new("source-target").unwrap(),
        target: TargetMapping::Property {
            target_id: "arm".into(), property: "rotation".into(), scale: 1.0, additive: false,
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
        "source-controller", "producer-input", vec![],
        if source_binding_exists { vec![producer_binding()] } else { vec![] },
    );
    let consumer = controller(
        "consumer-controller", "derived-input",
        if with_dependency { vec![producer_id] } else { vec![] },
        vec![],
    );
    CanonicalDocument {
        schema: SchemaIdentity { name: SCHEMA_NAME.into(), version: CURRENT_SCHEMA_VERSION },
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
