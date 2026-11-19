use smart_controller_core::{Mode, Params, validation::validate};

fn slider() -> Params {
    Params {
        mode: Mode::Slider, driver_layer_id: Some(2), driver_bone_name: Some("Head".into()),
        action_layer_id: Some(3), min_rotation_deg: Some(-45.0), max_rotation_deg: Some(45.0),
        action_start_frame: Some(0), action_end_frame: Some(10), endpoint_policy: None,
        target_bone_name: None, property: None, scale: None, additive: false, manual_value: None,
    }
}
#[test]
fn slider_requires_action_layer() {
    let mut p = slider(); p.action_layer_id = None;
    assert!(validate(&p).unwrap_err().contains("action_layer_id"));
}
#[test]
fn slider_requires_rotation_range() {
    let mut p = slider(); p.max_rotation_deg = None;
    assert!(validate(&p).unwrap_err().contains("max_rotation_deg"));
}
#[test]
fn slider_rejects_end_not_after_start() {
    let mut p = slider(); p.action_end_frame = Some(0);
    assert!(validate(&p).unwrap_err().contains("greater than"));
}
#[test]
fn transform_requires_target_bone_property_and_scale() {
    let mut p = slider(); p.mode = Mode::Transform;
    assert!(validate(&p).unwrap_err().contains("target_bone_name"));
    p.target_bone_name = Some("Jaw".into());
    assert!(validate(&p).unwrap_err().contains("property"));
    p.property = Some("rotation".into());
    assert!(validate(&p).unwrap_err().contains("scale"));
}
#[test]
fn explicit_manual_input_does_not_require_a_host_bone() {
    let mut p = slider(); p.driver_layer_id = None; p.driver_bone_name = None; p.manual_value = Some(12.0);
    assert!(validate(&p).is_ok());
}
