use form_deform_core::{DeformSpec, Vec2 as DeformVec2, evaluate_pin_position};
use smart_controller_core::{
    ControllerBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange,
    PreviewResult, Vec2 as ControllerVec2,
};

#[test]
fn smart_controller_output_feeds_form_deform_stage_position_without_accumulation() {
    let binding = ControllerBinding {
        id: "suite-binding".into(),
        controller_id: "bone-17".into(),
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

    let Some(PreviewResult::GeometryDeform { offset, .. }) = binding.evaluate(22.5).unwrap() else {
        panic!("expected a geometry target");
    };

    let base = DeformVec2 { x: 120.0, y: 80.0 };
    let mapped = DeformSpec {
        neutral_position: DeformVec2 { x: base.x + offset.x, y: base.y + offset.y },
        neutral_angle_degrees: 0.0,
        gain_per_degree: DeformVec2 { x: 0.0, y: 0.0 },
    };
    let first = evaluate_pin_position(mapped, 0.0).unwrap();
    let second = evaluate_pin_position(mapped, 0.0).unwrap();

    assert_eq!(first, DeformVec2 { x: 126.0, y: 78.0 });
    assert_eq!(second, first);
}
