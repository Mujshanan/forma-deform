use smart_controller_core::{
    ids::BindingId,
    pose::Pose2D,
    value::{Angle, TypedValue},
};

fn id(value: &str) -> BindingId { BindingId::new(value).unwrap() }
fn scalar(value: f64) -> TypedValue { TypedValue::scalar(value).unwrap() }

#[test]
fn pose_preserves_authored_order_and_looks_up_by_binding_identity() {
    let pose = Pose2D::new(vec![(id("b"), scalar(2.0)), (id("a"), scalar(1.0))]).unwrap();
    let ids: Vec<_> = pose.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["b", "a"]);
    assert_eq!(pose.get(&id("a")).unwrap().as_scalar(), Some(1.0));
}

#[test]
fn pose_rejects_duplicate_binding_ids_during_construction_and_deserialization() {
    assert!(Pose2D::new(vec![(id("same"), scalar(1.0)), (id("same"), scalar(2.0))]).is_err());
    let json = r#"{"values":[["same",{"kind":"scalar","value":1.0}],["same",{"kind":"scalar","value":2.0}]]}"#;
    assert!(serde_json::from_str::<Pose2D>(json).is_err());
}

#[test]
fn pose_compatibility_matches_ids_independent_of_order() {
    let first = Pose2D::new(vec![(id("x"), scalar(1.0)), (id("y"), TypedValue::vector2(1.0, 2.0).unwrap())]).unwrap();
    let second = Pose2D::new(vec![(id("y"), TypedValue::vector2(3.0, 4.0).unwrap()), (id("x"), scalar(5.0))]).unwrap();
    assert!(first.validate_compatible(&second).is_ok());
}

#[test]
fn pose_compatibility_returns_typed_errors_for_missing_or_incompatible_values() {
    let first = Pose2D::new(vec![(id("x"), scalar(1.0))]).unwrap();
    let missing = Pose2D::new(vec![(id("y"), scalar(1.0))]).unwrap();
    assert!(matches!(first.validate_compatible(&missing), Err(smart_controller_core::CoreError::PoseBindingMissing { .. })));

    let wrong_type = Pose2D::new(vec![(id("x"), TypedValue::vector2(1.0, 2.0).unwrap())]).unwrap();
    assert!(matches!(first.validate_compatible(&wrong_type), Err(smart_controller_core::CoreError::PoseValueTypeMismatch { .. })));
}

#[test]
fn pose_compatibility_rejects_wrapped_unwrapped_angle_mismatch() {
    let wrapped = Pose2D::new(vec![(id("rotation"), TypedValue::angle(0.0, true).unwrap())]).unwrap();
    let unwrapped = Pose2D::new(vec![(id("rotation"), TypedValue::angle(0.0, false).unwrap())]).unwrap();
    assert!(matches!(wrapped.validate_compatible(&unwrapped), Err(smart_controller_core::CoreError::PoseAngleSemanticsMismatch { .. })));
}

#[test]
fn pose_json_round_trip_preserves_binding_order() {
    let pose = Pose2D::new(vec![(id("second"), scalar(2.0)), (id("first"), scalar(1.0))]).unwrap();
    let json = serde_json::to_string(&pose).unwrap();
    let decoded: Pose2D = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, pose);
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1e-10, "actual={actual}, expected={expected}");
}

#[test]
fn wrapped_angle_interpolation_uses_the_shortest_arc() {
    let from = Angle::new(350.0_f64.to_radians(), true).unwrap();
    let to = Angle::new(10.0_f64.to_radians(), true).unwrap();
    close(from.interpolate(to, 0.5).unwrap().radians(), 0.0);
}

#[test]
fn unwrapped_angle_interpolation_preserves_authored_revolutions() {
    let from = Angle::new(350.0_f64.to_radians(), false).unwrap();
    let to = Angle::new(10.0_f64.to_radians(), false).unwrap();
    close(from.interpolate(to, 0.5).unwrap().radians(), 180.0_f64.to_radians());
}

#[test]
fn antipodal_wrapped_angle_ties_choose_positive_rotation() {
    let zero = Angle::new(0.0, true).unwrap();
    let positive_pi = Angle::new(std::f64::consts::PI, true).unwrap();
    let negative_pi = Angle::new(-std::f64::consts::PI, true).unwrap();
    close(zero.interpolate(positive_pi, 0.5).unwrap().radians(), std::f64::consts::FRAC_PI_2);
    close(zero.interpolate(negative_pi, 0.5).unwrap().radians(), std::f64::consts::FRAC_PI_2);
}

#[test]
fn angle_interpolation_rejects_mixed_semantics_and_non_finite_amounts() {
    let wrapped = Angle::new(0.0, true).unwrap();
    let unwrapped = Angle::new(1.0, false).unwrap();
    assert!(wrapped.interpolate(unwrapped, 0.5).is_err());
    assert!(wrapped.interpolate(wrapped, f64::NAN).is_err());
}

#[test]
fn wrapped_angle_interpolation_handles_large_finite_revolutions() {
    let from = Angle::new(f64::MAX, true).unwrap();
    let to = Angle::new(-f64::MAX, true).unwrap();
    assert!(from.interpolate(to, 0.5).unwrap().radians().is_finite());
}

#[test]
fn typed_value_interpolation_preserves_scalar_and_vector_types() {
    close(scalar(2.0).interpolate(scalar(6.0), 0.5).unwrap().as_scalar().unwrap(), 4.0);
    let vector = TypedValue::vector2(0.0, 10.0).unwrap()
        .interpolate(TypedValue::vector2(10.0, 30.0).unwrap(), 0.5).unwrap();
    let vector = vector.as_vector2().unwrap();
    close(vector.x(), 5.0);
    close(vector.y(), 20.0);
}

#[test]
fn typed_value_interpolation_delegates_angle_semantics_and_rejects_type_mismatch() {
    let from = TypedValue::angle(350.0_f64.to_radians(), true).unwrap();
    let to = TypedValue::angle(10.0_f64.to_radians(), true).unwrap();
    close(from.interpolate(to, 0.5).unwrap().as_angle().unwrap().radians(), 0.0);
    assert!(scalar(1.0).interpolate(TypedValue::vector2(1.0, 2.0).unwrap(), 0.5).is_err());
}
