use form_deform_core::{
    BoneAngle, DeformRequest, DeformSpec, Vec2 as DeformVec2, evaluate_deformation,
};
use serde_json::json;
use smart_controller_core::{
    ControllerBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange, PreviewResult,
    Vec2 as ControllerVec2,
};

#[test]
fn experimental_binding_core_composes_with_deformation_core() {
    // Synthetic fields resembling a subset of bone.info. This fixture does not invoke
    // the Smart Controller adapter or issue an AniCraft host command.
    let bone_info = json!({
        "frame": 12,
        "armatures": [{
            "bones": [{"id": 17, "angle": 22.5, "swing": 2.0}]
        }]
    });
    let bone = bone_info["armatures"][0]["bones"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"].as_u64() == Some(17))
        .unwrap();
    let input_angle = bone["angle"].as_f64().unwrap();

    let binding = ControllerBinding {
        id: "suite-binding".into(),
        controller_id: "17".into(),
        input_range: InputRange {
            minimum: -45.0,
            neutral: 0.0,
            maximum: 45.0,
        },
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
    let Some(PreviewResult::GeometryDeform { offset, .. }) = binding.evaluate(input_angle).unwrap()
    else {
        panic!("expected a geometry target");
    };

    // This is a separate pure-core calculation with explicit inputs. It does not consume
    // the synthetic bone.info fixture or exercise the plugin's Deform dispatch.
    let base = DeformVec2 {
        x: 120.0 + offset.x,
        y: 80.0 + offset.y,
    };
    let request = DeformRequest {
        spec: DeformSpec {
            neutral_position: base,
            neutral_angle_degrees: 0.0,
            gain_per_degree: DeformVec2 { x: 0.5, y: -0.25 },
        },
        bone_angle: BoneAngle {
            keyed_degrees: 10.0,
            spring_swing_degrees: Some(2.0),
        },
        include_spring_swing: false,
    };
    let first = evaluate_deformation(request).unwrap().target_position;
    let second = evaluate_deformation(request).unwrap().target_position;

    assert_eq!(offset, ControllerVec2 { x: 6.0, y: -2.0 });
    assert_eq!(first, DeformVec2 { x: 131.0, y: 75.5 });
    assert_eq!(second, first);
}
