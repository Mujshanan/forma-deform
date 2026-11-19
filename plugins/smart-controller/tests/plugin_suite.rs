use form_deform_core::{DeformSpec, Vec2 as DeformVec2, evaluate_pin_position};
use serde_json::json;
use smart_controller_core::{
    ControllerBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange,
    PreviewResult, Vec2 as ControllerVec2,
};

#[test]
fn bone_query_input_flows_through_controller_and_form_deform_math() {
    // Same stable-ID / keyed-angle / spring-swing fields reported by AniCraft bone.info
    // and inspected by the Bone Query adapter's response-fixture tests.
    let bone_info = json!({
        "frame": 12,
        "armatures": [{
            "bones": [{"id": 17, "angle": 22.5, "swing": 2.0}]
        }]
    });
    let bone = bone_info["armatures"][0]["bones"].as_array().unwrap()
        .iter().find(|item| item["id"].as_u64() == Some(17)).unwrap();
    let input_angle = bone["angle"].as_f64().unwrap(); // keyed pose; spring is opt-in

    let binding = ControllerBinding {
        id: "suite-binding".into(),
        controller_id: "17".into(),
        input_range: InputRange { minimum: -45.0, neutral: 0.0, maximum: 45.0 },
        easing: EaseCurve::Linear,
        mode: ControllerMode::RelativeDelta,
        target: ControllerTarget::GeometryDeform {
            element_id: "300".into(),
            pin_id: "900".into(),
            negative_offset: ControllerVec2 { x: -12.0, y: 4.0 },
            neutral_offset: ControllerVec2 { x: 0.0, y: 0.0 },
            positive_offset: ControllerVec2 { x: 12.0, y: -4.0 },
        },
        enabled: true,
    };
    let Some(PreviewResult::GeometryDeform { offset, .. }) = binding.evaluate(input_angle).unwrap() else {
        panic!("expected a geometry target");
    };

    // Form Deform takes the controller's neutral-relative result as the authored
    // neutral pin base, then computes its own deterministic bone-driven displacement.
    let base = DeformVec2 { x: 120.0 + offset.x, y: 80.0 + offset.y };
    let deform = DeformSpec {
        neutral_position: base,
        neutral_angle_degrees: 0.0,
        gain_per_degree: DeformVec2 { x: 0.5, y: -0.25 },
    };
    let first = evaluate_pin_position(deform, 10.0).unwrap();
    let second = evaluate_pin_position(deform, 10.0).unwrap();

    assert_eq!(offset, ControllerVec2 { x: 6.0, y: -2.0 });
    assert_eq!(first, DeformVec2 { x: 131.0, y: 75.5 });
    assert_eq!(second, first);
}
