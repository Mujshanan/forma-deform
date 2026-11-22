use serde_json::json;
use smart_controller_core::{
    ControllerBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange,
    PreviewResult, Vec2 as ControllerVec2,
};

#[test]
fn bone_query_input_produces_independent_controller_geometry_preview() {
    // Same stable-ID / keyed-angle / spring-swing fields reported by AniCraft bone.info.
    // Smart Controller V1 evaluates this binding as preview data only.
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
        panic!("expected a geometry target preview");
    };

    assert_eq!(offset, ControllerVec2 { x: 6.0, y: -2.0 });
}
