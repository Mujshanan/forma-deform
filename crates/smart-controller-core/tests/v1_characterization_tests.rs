//! Executable characterization of the pre-canonical V1 inputs and outputs.
//!
//! These assertions intentionally describe the existing implementation. Update
//! them only through an explicit compatibility decision, not as part of a
//! refactor that happens to change the observed result.

use smart_controller_core::{
    preview_slider, BindingSet, ControllerMode, ControllerTarget, EndpointPolicy, Mode,
    Params, PreviewResult, Vec2, validation,
};

const LEGACY_BINDING_SET: &str = include_str!("fixtures/legacy_binding_set_v1.json");
const HOST_PARAMS: &str = include_str!("fixtures/legacy_params_slider_unversioned.json");
const MANUAL_PARAMS: &str =
    include_str!("fixtures/legacy_params_slider_manual_unversioned.json");

fn assert_close(actual: f64, expected: f64) {
    let tolerance = 1.0e-12;
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn legacy_binding_set_fixture_deserializes_and_round_trips() {
    let set = BindingSet::from_json(LEGACY_BINDING_SET)
        .expect("the captured V1 BindingSet fixture must remain readable");

    assert_eq!(set.schema_version, 1);
    assert_eq!(set.bindings.len(), 2);
    assert_eq!(set.bindings[0].id, "binding-head-turn");
    assert_eq!(set.bindings[1].id, "binding-arm-corrective");

    let encoded = set.to_json().expect("valid V1 set must serialize");
    let decoded = BindingSet::from_json(&encoded).expect("serialized V1 set must decode");
    assert_eq!(decoded, set);

    let original_json: serde_json::Value =
        serde_json::from_str(LEGACY_BINDING_SET).expect("fixture must be valid JSON");
    let round_trip_json: serde_json::Value =
        serde_json::from_str(&encoded).expect("serialized set must be valid JSON");
    assert_eq!(round_trip_json, original_json);
}

#[test]
fn legacy_symbol_frame_outputs_are_characterized() {
    let set = BindingSet::from_json(LEGACY_BINDING_SET).unwrap();
    let binding = set.get("binding-head-turn").unwrap();

    for (input, expected_frame) in [(-45.0, 1), (0.0, 5), (45.0, 9), (-22.5, 3)] {
        assert_eq!(
            binding.evaluate(input).unwrap(),
            Some(PreviewResult::SymbolFrame {
                symbol_instance_id: "instance-head".into(),
                frame: expected_frame,
            }),
            "unexpected frame for input {input}"
        );
    }

    assert_eq!(binding.evaluate(-90.0).unwrap(), binding.evaluate(-45.0).unwrap());
    assert_eq!(binding.evaluate(90.0).unwrap(), binding.evaluate(45.0).unwrap());
}

#[test]
fn legacy_geometry_relative_delta_and_smoothstep_outputs_are_characterized() {
    let set = BindingSet::from_json(LEGACY_BINDING_SET).unwrap();
    let binding = set.get("binding-arm-corrective").unwrap();

    assert_eq!(binding.mode, ControllerMode::RelativeDelta);

    let Some(PreviewResult::GeometryDeform { offset, .. }) =
        binding.evaluate(0.0).unwrap()
    else {
        panic!("expected geometry preview at neutral");
    };
    assert_eq!(offset, Vec2 { x: 0.0, y: 0.0 });

    let Some(PreviewResult::GeometryDeform { offset, .. }) =
        binding.evaluate(-45.0).unwrap()
    else {
        panic!("expected geometry preview at negative endpoint");
    };
    assert_eq!(offset, Vec2 { x: -5.0, y: 1.0 });

    let Some(PreviewResult::GeometryDeform { offset, .. }) =
        binding.evaluate(45.0).unwrap()
    else {
        panic!("expected geometry preview at positive endpoint");
    };
    assert_eq!(offset, Vec2 { x: 8.0, y: 4.0 });

    // Smoothstep(0.5) = 0.5, so the midpoint lies halfway from neutral
    // (1, 1) to positive (9, 5), then RelativeDelta subtracts neutral.
    let Some(PreviewResult::GeometryDeform { offset, .. }) =
        binding.evaluate(22.5).unwrap()
    else {
        panic!("expected geometry preview at positive midpoint");
    };
    assert_eq!(offset, Vec2 { x: 4.0, y: 2.0 });
}

#[test]
fn host_driven_slider_fixture_deserializes_validates_and_preserves_preview() {
    let params: Params =
        serde_json::from_str(HOST_PARAMS).expect("host-driven Params fixture must decode");

    assert_eq!(params.mode, Mode::Slider);
    assert_eq!(params.driver_layer_id, Some(2));
    assert_eq!(params.driver_bone_name.as_deref(), Some("Head"));
    assert_eq!(params.manual_value, None);
    assert_eq!(params.endpoint_policy(), EndpointPolicy::Exclusive);
    validation::validate(&params).expect("captured host-driven parameters remain valid");

    // 22.5 degrees is the captured bone-angle sample used by the V1 preview
    // path after the adapter's bone.info lookup.
    let output = preview_slider(&params, 22.5).expect("host-driven preview must calculate");
    assert_close(output.normalized, 0.75);
    assert_eq!(output.output_frame, Some(7));
    assert_eq!(output.output_value, None);
}

#[test]
fn manual_input_slider_fixture_deserializes_validates_and_preserves_preview() {
    let params: Params =
        serde_json::from_str(MANUAL_PARAMS).expect("manual-input Params fixture must decode");

    assert_eq!(params.mode, Mode::Slider);
    assert_eq!(params.driver_layer_id, None);
    assert_eq!(params.driver_bone_name, None);
    assert_eq!(params.manual_value, Some(12.5));
    assert_eq!(params.endpoint_policy(), EndpointPolicy::Inclusive);
    validation::validate(&params).expect("captured manual parameters remain valid");

    let driver_value = params.manual_value.expect("manual fixture includes its input");
    let output = preview_slider(&params, driver_value).expect("manual preview must calculate");
    assert_close(output.normalized, 57.5 / 90.0);
    assert_eq!(output.output_frame, Some(6));
    assert_eq!(output.output_value, None);

    let original_json: serde_json::Value =
        serde_json::from_str(MANUAL_PARAMS).expect("fixture must be valid JSON");
    let encoded = serde_json::to_string(&params).expect("Params must serialize");
    let round_trip_json: serde_json::Value =
        serde_json::from_str(&encoded).expect("serialized Params must be valid JSON");
    assert_eq!(round_trip_json, original_json);
}

#[test]
fn endpoint_policy_and_reversed_range_v1_behaviour_are_characterized() {
    use smart_controller_core::mapping::{map_to_frame, normalize_rotation};

    assert_eq!(map_to_frame(1.0, 0, 10, EndpointPolicy::Exclusive), 9);
    assert_eq!(map_to_frame(1.0, 0, 10, EndpointPolicy::Inclusive), 10);
    assert_eq!(map_to_frame(0.0, 4, 4, EndpointPolicy::Inclusive), 4);

    assert_eq!(normalize_rotation(45.0, 45.0, -45.0).unwrap(), 0.0);
    assert_eq!(normalize_rotation(0.0, 45.0, -45.0).unwrap(), 0.5);
    assert_eq!(normalize_rotation(-45.0, 45.0, -45.0).unwrap(), 1.0);
}

#[test]
fn transform_mode_is_a_pure_core_preview_but_not_a_v1_adapter_capability() {
    // This pins the existing core capability without claiming the V1 plugin
    // accepts Transform mode (the adapter explicitly rejects it).
    let mut params: Params = serde_json::from_str(HOST_PARAMS).unwrap();
    params.mode = Mode::Transform;
    params.target_bone_name = Some("Head".into());
    params.property = Some("rotation".into());
    params.scale = Some(90.0);
    params.action_layer_id = None;
    params.action_start_frame = None;
    params.action_end_frame = None;

    validation::validate(&params).expect("pure Transform parameters are valid in core");
    let output = smart_controller_core::preview_transform(&params, 0.0)
        .expect("core Transform calculation remains available");
    assert_eq!(output.output_frame, None);
    assert_eq!(output.output_value, Some(45.0));
}
