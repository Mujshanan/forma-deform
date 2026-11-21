use smart_controller_core::{
    error::CoreError,
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{
        BoneTransformChannel, FallbackPolicy, Freshness, InputDefinition, InputDimension,
        InputSample, InputSnapshot, InputSource, InputValidity,
    },
    value::{Angle, Scalar, TypedValue, Vector2},
};

fn input_id(value: &str) -> InputId {
    InputId::new(value).unwrap()
}

#[test]
fn strongly_typed_ids_validate_and_round_trip_as_strings() {
    let controller = ControllerId::new("controller-main").unwrap();
    let input = InputId::new("input-x").unwrap();
    let binding = BindingId::new("binding-pose").unwrap();
    let target = TargetId::new("target-head").unwrap();

    assert_eq!(controller.as_str(), "controller-main");
    assert_eq!(input.to_string(), "input-x");
    assert_eq!(binding.to_string(), "binding-pose");
    assert_eq!(target.to_string(), "target-head");
    assert!(ControllerId::new("  ").is_err());

    let encoded = serde_json::to_string(&controller).unwrap();
    assert_eq!(encoded, "\"controller-main\"");
    assert_eq!(
        serde_json::from_str::<ControllerId>(&encoded).unwrap(),
        controller
    );
    assert!(serde_json::from_str::<InputId>("\"\\t\"").is_err());
}

#[test]
fn typed_values_reject_non_finite_numbers() {
    assert!(matches!(
        Scalar::new(f64::NAN),
        Err(CoreError::NonFiniteValue { context: "scalar" })
    ));
    assert!(Vector2::new(1.0, f64::INFINITY).is_err());
    assert!(Angle::new(f64::NEG_INFINITY, true).is_err());
    assert!(TypedValue::scalar(f64::NAN).is_err());
}

#[test]
fn typed_value_json_preserves_distinct_angle_semantics() {
    let wrapped = TypedValue::angle(std::f64::consts::PI, true).unwrap();
    let unwrapped = TypedValue::angle(4.0 * std::f64::consts::PI, false).unwrap();

    let wrapped_json = serde_json::to_string(&wrapped).unwrap();
    let decoded: TypedValue = serde_json::from_str(&wrapped_json).unwrap();
    assert_eq!(decoded, wrapped);
    assert!(decoded.as_angle().unwrap().is_wrapped());
    assert!(!unwrapped.as_angle().unwrap().is_wrapped());
    assert_eq!(unwrapped.as_angle().unwrap().radians(), 4.0 * std::f64::consts::PI);
}

#[test]
fn scalar_and_vector_values_round_trip() {
    let values = [
        TypedValue::scalar(12.5).unwrap(),
        TypedValue::vector2(-2.0, 8.25).unwrap(),
    ];
    let encoded = serde_json::to_string(&values).unwrap();
    let decoded: Vec<TypedValue> = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn sample_freshness_uses_strictly_greater_than_maximum_age() {
    let sample = InputSample::valid(input_id("rotation"), TypedValue::angle(0.5, true).unwrap(), Some(10));
    assert_eq!(sample.freshness(10), Freshness::Fresh);
    assert_eq!(sample.freshness(9), Freshness::Stale);
}

#[test]
fn unknown_sample_age_remains_unknown() {
    let sample = InputSample::valid(input_id("manual"), TypedValue::scalar(3.0).unwrap(), None);
    assert_eq!(sample.freshness(0), Freshness::Unknown);
}

#[test]
fn snapshot_rejects_duplicate_ids_and_invalid_status_value_pairs() {
    let duplicate = vec![
        InputSample::valid(input_id("same"), TypedValue::scalar(1.0).unwrap(), Some(0)),
        InputSample::valid(input_id("same"), TypedValue::scalar(2.0).unwrap(), Some(0)),
    ];
    assert_eq!(
        InputSnapshot::new(5, duplicate),
        Err(CoreError::DuplicateInputId("same".into()))
    );

    let inconsistent = InputSample {
        id: input_id("bad"),
        value: Some(TypedValue::scalar(1.0).unwrap()),
        validity: InputValidity::Missing,
        age_ticks: None,
    };
    assert!(inconsistent.validate().is_err());
}

#[test]
fn snapshot_lookup_and_freshness_are_deterministic() {
    let sample = InputSample::valid(input_id("manual"), TypedValue::scalar(2.0).unwrap(), Some(4));
    let snapshot = InputSnapshot::new(4, vec![sample]).unwrap();
    assert_eq!(snapshot.get(&input_id("manual")).unwrap().value.unwrap().as_scalar(), Some(2.0));
    assert_eq!(
        snapshot.freshness(snapshot.get(&input_id("manual")).unwrap()),
        Freshness::Fresh
    );
    assert!(snapshot.get(&input_id("absent")).is_none());
}

#[test]
fn input_definition_validates_fallback_dimension_and_source_identity() {
    let definition = InputDefinition {
        id: input_id("bone-rotation"),
        source: InputSource::BoneTransform {
            bone_id: "bone-head".into(),
            channel: BoneTransformChannel::Rotation,
        },
        dimension: InputDimension::Angle,
        fallback: FallbackPolicy::UseValue(TypedValue::angle(0.0, true).unwrap()),
    };
    definition.validate().unwrap();

    let mismatched = InputDefinition {
        dimension: InputDimension::Vector2,
        fallback: FallbackPolicy::UseValue(TypedValue::scalar(0.0).unwrap()),
        ..definition.clone()
    };
    assert!(mismatched.validate().is_err());

    let empty_bone = InputDefinition {
        source: InputSource::BoneTransform {
            bone_id: " ".into(),
            channel: BoneTransformChannel::Rotation,
        },
        ..definition
    };
    assert!(empty_bone.validate().is_err());
}

#[test]
fn input_definition_checks_sample_identity_and_type() {
    let definition = InputDefinition {
        id: input_id("position"),
        source: InputSource::Manual,
        dimension: InputDimension::Vector2,
        fallback: FallbackPolicy::Unavailable,
    };
    let matching = InputSample::valid(
        input_id("position"),
        TypedValue::vector2(1.0, 2.0).unwrap(),
        Some(0),
    );
    definition.validate_sample(&matching).unwrap();

    let wrong_type = InputSample::valid(
        input_id("position"),
        TypedValue::scalar(1.0).unwrap(),
        Some(0),
    );
    assert!(matches!(
        definition.validate_sample(&wrong_type),
        Err(CoreError::InputValueTypeMismatch { .. })
    ));
    let wrong_id = InputSample::valid(
        input_id("other"),
        TypedValue::vector2(1.0, 2.0).unwrap(),
        Some(0),
    );
    assert!(definition.validate_sample(&wrong_id).is_err());
}

#[test]
fn missing_and_unavailable_samples_have_no_implicit_value() {
    let missing = InputSample::missing(input_id("missing"));
    let unavailable = InputSample::unavailable(input_id("unavailable"));
    assert_eq!(missing.validity, InputValidity::Missing);
    assert_eq!(unavailable.validity, InputValidity::Unavailable);
    assert!(missing.value.is_none());
    assert!(unavailable.value.is_none());
}
