use smart_controller_core::{
    contribution::{BaseRef, CompositionMode, Contribution},
    ids::{BindingId, TargetId},
    schema::{SchemaEnvelope, CURRENT_SCHEMA_VERSION, SCHEMA_NAME},
    value::TypedValue,
};

#[test]
fn schema_envelope_has_explicit_identity_and_rejects_wrong_version() {
    let envelope = SchemaEnvelope::new(serde_json::json!({"id":"controller-a"}));
    assert_eq!(envelope.schema.name, SCHEMA_NAME);
    assert_eq!(envelope.schema.version, CURRENT_SCHEMA_VERSION);
    envelope.validate_identity().unwrap();
    let mut invalid = envelope;
    invalid.schema.version += 1;
    assert!(invalid.validate_identity().is_err());
}

fn contribution(mode: CompositionMode) -> Contribution {
    Contribution {
        binding_id: BindingId::new("binding-a").unwrap(),
        target_id: TargetId::new("target-a").unwrap(),
        value: TypedValue::scalar(2.0).unwrap(),
        mode,
        weight: None,
        priority: None,
        base_ref: None,
    }
}

#[test]
fn contribution_requires_mode_specific_fields() {
    assert!(contribution(CompositionMode::Replace).validate().is_ok());
    let mut additive = contribution(CompositionMode::Additive);
    assert!(additive.validate().is_err());
    additive.base_ref = Some(BaseRef::Neutral);
    assert!(additive.validate().is_ok());

    let mut weighted = contribution(CompositionMode::WeightedBlend);
    weighted.weight = Some(0.0);
    assert!(weighted.validate().is_ok());
    weighted.weight = Some(f64::NAN);
    assert!(weighted.validate().is_err());

    let mut priority = contribution(CompositionMode::PriorityOverride);
    priority.priority = Some(10);
    assert!(priority.validate().is_ok());
}
