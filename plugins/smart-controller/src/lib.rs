//! Thin AniCraft adapter for the unified Smart Controller plugin.
//!
//! Pure mapping lives in smart-controller-core; pure pin-deformation math lives
//! in form-deform-core. This crate composes both with the AniCraft ABI/host API.

use anicraft_plugin_sdk as ac;
use form_deform_core::{BoneAngle, DeformRequest, DeformSpec, Vec2, evaluate_deformation};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use smart_controller_core::{Mode, Params, preview_slider, validation};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.smart-controller",
  "name": "AniCraft Smart Controller",
  "version": "0.3.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "On-demand bone-driven animation and geometry controller with Slider and Deform modes.",
  "params": {
    "mode": {"type": "string", "default": "slider", "label": "Controller mode: slider or deform"},
    "driver_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Driver armature/layer ID (Slider mode)"},
    "driver_bone_name": {"type": "string", "default": "Head", "label": "Driver bone name (Slider mode)"},
    "action_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Action layer ID (Slider mode)"},
    "min_rotation_deg": {"type": "number", "default": -45, "label": "Minimum driver angle (Slider mode)"},
    "max_rotation_deg": {"type": "number", "default": 45, "label": "Maximum driver angle (Slider mode)"},
    "action_start_frame": {"type": "int", "min": 0, "max": 1000000000, "default": 0, "label": "Action start frame (Slider mode)"},
    "action_end_frame": {"type": "int", "min": 1, "max": 1000000000, "default": 20, "label": "Action end frame (Slider mode)"},
    "endpoint_policy": {"type": "string", "default": "exclusive", "label": "Endpoint policy: exclusive or inclusive"},
    "manual_value": {"type": "number", "label": "Manual driver angle (Slider mode; overrides bone query)"},
    "bone_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Controller bone ID (Deform mode)"},
    "element_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warped shape element ID (Deform mode)"},
    "pin_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warp pin ID (Deform mode)"},
    "neutral_angle": {"type": "number", "default": 0, "label": "Neutral bone angle (degrees)"},
    "neutral_x": {"type": "number", "default": 0, "label": "Neutral pin X (stage px)"},
    "neutral_y": {"type": "number", "default": 0, "label": "Neutral pin Y (stage px)"},
    "gain_x": {"type": "number", "default": 0, "label": "X displacement per degree (px/degree)"},
    "gain_y": {"type": "number", "default": 0, "label": "Y displacement per degree (px/degree)"},
    "include_spring_swing": {"type": "bool", "default": false, "label": "Include spring swing in bone angle"},
    "apply": {"type": "bool", "default": false, "label": "Apply pin position (creates an undoable document edit)"}
  }
}"##;

ac::plugin!(MANIFEST, run);

/// Host boundary used by the production adapter and deterministic adapter tests.
trait HostApi {
    fn command(&mut self, id: &str, params: Value) -> Result<Value, String>;
    fn log(&mut self, message: &str);
}

struct AniCraftHost;

impl HostApi for AniCraftHost {
    fn command(&mut self, id: &str, params: Value) -> Result<Value, String> {
        ac::command(id, params)
    }

    fn log(&mut self, message: &str) {
        ac::log(message);
    }
}

fn select_fields(raw: &Value, fields: &[&str]) -> Value {
    let Some(source) = raw.as_object() else {
        return Value::Null;
    };
    let mut selected = Map::new();
    for field in fields {
        if let Some(value) = source.get(*field) {
            selected.insert((*field).to_owned(), value.clone());
        }
    }
    Value::Object(selected)
}

fn find_driver_bone(result: &Value, name: &str) -> Option<f64> {
    result["armatures"]
        .as_array()?
        .iter()
        .flat_map(|armature| armature["bones"].as_array().into_iter().flatten())
        .find(|bone| bone["name"].as_str() == Some(name))
        .and_then(|bone| bone["angle"].as_f64())
}

fn query_slider_driver(params: &Params, host: &mut impl HostApi) -> Result<f64, String> {
    let layer_id = params
        .driver_layer_id
        .ok_or("driver_layer_id is required for Slider mode bone input")?;
    let name = params
        .driver_bone_name
        .as_deref()
        .ok_or("driver_bone_name is required for Slider mode bone input")?;
    let response = host.command("bone.info", json!({"layer": layer_id}))?;
    find_driver_bone(&response, name)
        .ok_or_else(|| format!("driver bone '{name}' was not found or has no evaluated angle"))
}

fn run_slider(raw: &Value, host: &mut impl HostApi) -> Result<(), String> {
    const FIELDS: &[&str] = &[
        "mode",
        "driver_layer_id",
        "driver_bone_name",
        "action_layer_id",
        "min_rotation_deg",
        "max_rotation_deg",
        "action_start_frame",
        "action_end_frame",
        "endpoint_policy",
        "manual_value",
    ];
    let payload = select_fields(raw, FIELDS);
    let params: Params = serde_json::from_value(payload)
        .map_err(|error| format!("invalid Slider parameters: {error}"))?;
    validation::validate(&params)?;
    if params.mode != Mode::Slider {
        return Err("Only Slider mode is currently supported by this mapping path; Transform remains deferred.".into());
    }
    let driver_value = match params.manual_value {
        Some(value) => value,
        None => query_slider_driver(&params, host)?,
    };
    let output = preview_slider(&params, driver_value)?;
    let serialized = serde_json::to_string(&output)
        .map_err(|error| format!("could not serialize Slider preview: {error}"))?;
    host.log(&format!(
        "AniCraft Smart Controller — Slider preview: {serialized}"
    ));
    host.log("Slider mode is preview-only: no timeline command or document mutation was issued.");
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeformParams {
    bone_id: u64,
    element_id: u64,
    pin_id: u64,
    neutral_angle: f64,
    neutral_x: f64,
    neutral_y: f64,
    gain_x: f64,
    gain_y: f64,
    #[serde(default)]
    include_spring_swing: bool,
    #[serde(default)]
    apply: bool,
}

#[derive(Debug, Deserialize)]
struct BoneInfoResponse {
    armatures: Vec<ArmatureDto>,
}
#[derive(Debug, Deserialize)]
struct ArmatureDto {
    bones: Vec<BoneDto>,
}
#[derive(Debug, Deserialize)]
struct BoneDto {
    id: u64,
    angle: f64,
    swing: Option<f64>,
}
#[derive(Debug, Deserialize)]
struct WarpInfoResponse {
    warped: bool,
    pins: Vec<WarpPinDto>,
}
#[derive(Debug, Deserialize)]
struct WarpPinDto {
    pin: u64,
}
#[derive(Debug, serde::Serialize)]
struct WarpPinMoveRequest {
    element: u64,
    pin: u64,
    x: f64,
    y: f64,
}

fn run_deform(raw: &Value, host: &mut impl HostApi) -> Result<(), String> {
    const FIELDS: &[&str] = &[
        "bone_id",
        "element_id",
        "pin_id",
        "neutral_angle",
        "neutral_x",
        "neutral_y",
        "gain_x",
        "gain_y",
        "include_spring_swing",
        "apply",
    ];
    let payload = select_fields(raw, FIELDS);
    let params: DeformParams = serde_json::from_value(payload)
        .map_err(|error| format!("invalid Deform parameters: {error}"))?;

    // Host integration ends at typed input extraction. No host DTO crosses into the core.
    let bone_value = host.command("bone.info", json!({}))?;
    let bone_data: BoneInfoResponse = serde_json::from_value(bone_value)
        .map_err(|error| format!("invalid bone.info response: {error}"))?;
    let bone = bone_data
        .armatures
        .iter()
        .flat_map(|armature| armature.bones.iter())
        .find(|bone| bone.id == params.bone_id)
        .ok_or_else(|| format!("bone {} was not returned by bone.info", params.bone_id))?;

    let warp_value = host.command("warp.info", json!({"element": params.element_id}))?;
    let warp: WarpInfoResponse = serde_json::from_value(warp_value)
        .map_err(|error| format!("invalid warp.info response: {error}"))?;
    if !warp.warped {
        return Err(format!(
            "element {} has no Asset Warp mesh",
            params.element_id
        ));
    }
    if !warp.pins.iter().any(|pin| pin.pin == params.pin_id) {
        return Err(format!(
            "pin {} was not returned for element {}",
            params.pin_id, params.element_id
        ));
    }

    let result = evaluate_deformation(DeformRequest {
        spec: DeformSpec {
            neutral_position: Vec2 {
                x: params.neutral_x,
                y: params.neutral_y,
            },
            neutral_angle_degrees: params.neutral_angle,
            gain_per_degree: Vec2 {
                x: params.gain_x,
                y: params.gain_y,
            },
        },
        bone_angle: BoneAngle {
            keyed_degrees: bone.angle,
            spring_swing_degrees: bone.swing,
        },
        include_spring_swing: params.include_spring_swing,
    })
    .map_err(|error| error.to_string())?;

    host.log(&format!(
        "AniCraft Smart Controller — Deform mode: bone {} effective angle {:.3}°",
        params.bone_id, result.effective_angle_degrees
    ));
    host.log(&format!(
        "Pin {} target: ({:.2}, {:.2}) stage px",
        params.pin_id, result.target_position.x, result.target_position.y
    ));
    if params.apply {
        let request = WarpPinMoveRequest {
            element: params.element_id,
            pin: params.pin_id,
            x: result.target_position.x,
            y: result.target_position.y,
        };
        let payload = serde_json::to_value(request).map_err(|error| error.to_string())?;
        host.command("warp.pin_move", payload)?;
        host.log("Applied pin target. This document edit is undoable.");
    } else {
        host.log("Calculation only. Enable Apply to write the result.");
    }
    Ok(())
}

fn run(raw: &Value) -> Result<(), String> {
    let mut host = AniCraftHost;
    run_with_host(raw, &mut host)
}

fn run_with_host(raw: &Value, host: &mut impl HostApi) -> Result<(), String> {
    match raw.get("mode").and_then(Value::as_str).unwrap_or("slider") {
        "slider" => run_slider(raw, host),
        "deform" => run_deform(raw, host),
        "transform" => Err("Transform mode remains deferred until verified host transform read/write APIs are available.".into()),
        other => Err(format!("unsupported Smart Controller mode '{other}'; choose 'slider' or 'deform'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::VecDeque;

    #[test]
    fn slider_bone_resolution_uses_bone_info_response_shape() {
        let data = json!({"armatures":[{"bones":[{"name":"Head","angle":12.5},{"name":"Jaw","angle":-4.0}]}]});
        assert_eq!(find_driver_bone(&data, "Jaw"), Some(-4.0));
        assert_eq!(find_driver_bone(&data, "Missing"), None);
    }

    #[test]
    fn malformed_slider_bone_info_fails_closed() {
        assert_eq!(find_driver_bone(&json!({"armatures":[]}), "Head"), None);
    }

    #[test]
    fn each_mode_ignores_the_other_modes_manifest_fields() {
        let all_fields = json!({
            "mode":"slider", "driver_layer_id":1, "driver_bone_name":"Head",
            "action_layer_id":2, "min_rotation_deg":-45.0, "max_rotation_deg":45.0,
            "action_start_frame":0, "action_end_frame":20, "endpoint_policy":"exclusive",
            "manual_value":null, "bone_id":4, "element_id":8, "pin_id":9,
            "neutral_angle":0.0, "neutral_x":10.0, "neutral_y":20.0,
            "gain_x":0.5, "gain_y":-0.25, "include_spring_swing":false, "apply":false
        });
        let deform: DeformParams = serde_json::from_value(select_fields(
            &all_fields,
            &[
                "bone_id",
                "element_id",
                "pin_id",
                "neutral_angle",
                "neutral_x",
                "neutral_y",
                "gain_x",
                "gain_y",
                "include_spring_swing",
                "apply",
            ],
        ))
        .unwrap();
        assert_eq!(deform.bone_id, 4);
        let slider: Params = serde_json::from_value(select_fields(
            &all_fields,
            &[
                "mode",
                "driver_layer_id",
                "driver_bone_name",
                "action_layer_id",
                "min_rotation_deg",
                "max_rotation_deg",
                "action_start_frame",
                "action_end_frame",
                "endpoint_policy",
                "manual_value",
            ],
        ))
        .unwrap();
        assert_eq!(slider.mode, Mode::Slider);
    }

    #[test]
    fn deform_host_dtos_decode_and_keep_missing_optional_values() {
        let data: BoneInfoResponse = serde_json::from_value(json!({
            "armatures":[{"bones":[{"id":4,"angle":8.0}]}]
        }))
        .unwrap();
        assert_eq!(data.armatures[0].bones[0].swing, None);
        let warp: WarpInfoResponse = serde_json::from_value(json!({
            "warped":true,"pins":[{"pin":9}]
        }))
        .unwrap();
        assert!(warp.warped && warp.pins[0].pin == 9);
    }

    #[test]
    fn malformed_deform_host_data_is_rejected() {
        assert!(serde_json::from_value::<BoneInfoResponse>(json!({"armatures":null})).is_err());
        assert!(serde_json::from_value::<WarpInfoResponse>(json!({"warped":true})).is_err());
        assert!(serde_json::from_value::<DeformParams>(json!({"bone_id":1})).is_err());
    }

    #[test]
    fn pin_move_dto_serializes_only_host_fields() {
        let value = serde_json::to_value(WarpPinMoveRequest {
            element: 2,
            pin: 3,
            x: 4.0,
            y: 5.0,
        })
        .unwrap();
        assert_eq!(value, json!({"element":2,"pin":3,"x":4.0,"y":5.0}));
    }

    #[derive(Default)]
    struct FakeHost {
        responses: VecDeque<Result<Value, String>>,
        commands: Vec<(String, Value)>,
        logs: Vec<String>,
    }

    impl FakeHost {
        fn respond(&mut self, value: Value) {
            self.responses.push_back(Ok(value));
        }

        fn fail(&mut self, message: &str) {
            self.responses.push_back(Err(message.to_owned()));
        }
    }

    impl HostApi for FakeHost {
        fn command(&mut self, id: &str, params: Value) -> Result<Value, String> {
            self.commands.push((id.to_owned(), params));
            self.responses
                .pop_front()
                .unwrap_or_else(|| Err(format!("unexpected host command '{id}'")))
        }

        fn log(&mut self, message: &str) {
            self.logs.push(message.to_owned());
        }
    }

    fn slider_params(manual_value: Option<f64>) -> Value {
        let mut params = json!({
            "mode": "slider",
            "driver_layer_id": 5,
            "driver_bone_name": "Head",
            "action_layer_id": 2,
            "min_rotation_deg": -45.0,
            "max_rotation_deg": 45.0,
            "action_start_frame": 0,
            "action_end_frame": 20,
            "endpoint_policy": "exclusive"
        });
        if let Some(value) = manual_value {
            params["manual_value"] = json!(value);
        }
        params
    }

    fn deform_params(apply: bool) -> Value {
        json!({
            "mode": "deform",
            "bone_id": 4,
            "element_id": 8,
            "pin_id": 9,
            "neutral_angle": 0.0,
            "neutral_x": 120.0,
            "neutral_y": 80.0,
            "gain_x": 0.5,
            "gain_y": -0.25,
            "include_spring_swing": false,
            "apply": apply
        })
    }

    fn bone_response(angle: f64, swing: Option<f64>) -> Value {
        let mut bone = json!({"id": 4, "angle": angle});
        if let Some(value) = swing {
            bone["swing"] = json!(value);
        }
        json!({"armatures":[{"bones":[bone]}]})
    }

    fn warp_response(warped: bool, pin_ids: &[u64]) -> Value {
        json!({
            "warped": warped,
            "pins": pin_ids.iter().map(|pin| json!({"pin": pin})).collect::<Vec<_>>()
        })
    }

    #[test]
    fn slider_manual_input_previews_without_any_host_command() {
        let mut host = FakeHost::default();
        let raw = slider_params(Some(22.5));

        run_with_host(&raw, &mut host).unwrap();

        assert!(host.commands.is_empty());
        assert!(host.logs.iter().any(|line| line.contains("\"normalized\":0.75")));
        assert!(host.logs.iter().any(|line| line.contains("\"output_frame\":15")));
        assert!(host.logs.iter().any(|line| line.contains("preview-only")));
    }

    #[test]
    fn slider_queries_driver_bone_and_never_writes_timeline() {
        let mut host = FakeHost::default();
        host.respond(json!({"armatures":[{"bones":[{"name":"Head","angle":45.0}]}]}));

        run_with_host(&slider_params(None), &mut host).unwrap();

        assert_eq!(host.commands, vec![("bone.info".into(), json!({"layer":5}))]);
        assert!(host.logs.iter().any(|line| line.contains("\"output_frame\":19")));
        assert!(host.logs.iter().any(|line| line.contains("no timeline command")));
    }

    #[test]
    fn slider_missing_bone_fails_without_fallback_value() {
        let mut host = FakeHost::default();
        host.respond(json!({"armatures":[{"bones":[{"name":"Jaw","angle":12.0}]}]}));

        let error = run_with_host(&slider_params(None), &mut host).unwrap_err();

        assert!(error.contains("driver bone 'Head' was not found"));
        assert_eq!(host.commands.len(), 1);
    }

    #[test]
    fn deform_preview_queries_bone_and_pin_but_does_not_apply() {
        let mut host = FakeHost::default();
        host.respond(bone_response(10.0, Some(2.0)));
        host.respond(warp_response(true, &[9]));

        run_with_host(&deform_params(false), &mut host).unwrap();

        assert_eq!(
            host.commands,
            vec![
                ("bone.info".into(), json!({})),
                ("warp.info".into(), json!({"element":8}))
            ]
        );
        assert!(host.logs.iter().any(|line| line.contains("Pin 9 target: (125.00, 77.50)")));
        assert!(host.logs.iter().any(|line| line.contains("Calculation only")));
    }

    #[test]
    fn deform_apply_sends_exact_undoable_pin_move_payload() {
        let mut host = FakeHost::default();
        host.respond(bone_response(10.0, Some(2.0)));
        host.respond(warp_response(true, &[9]));
        host.respond(Value::Null);

        run_with_host(&deform_params(true), &mut host).unwrap();

        assert_eq!(
            host.commands,
            vec![
                ("bone.info".into(), json!({})),
                ("warp.info".into(), json!({"element":8})),
                (
                    "warp.pin_move".into(),
                    json!({"element":8,"pin":9,"x":125.0,"y":77.5})
                )
            ]
        );
        assert!(host.logs.iter().any(|line| line.contains("undoable")));
    }

    #[test]
    fn deform_spring_swing_is_used_only_when_enabled() {
        let mut raw = deform_params(false);
        raw["neutral_x"] = json!(0.0);
        raw["neutral_y"] = json!(0.0);
        raw["gain_x"] = json!(1.0);
        raw["gain_y"] = json!(0.0);
        raw["include_spring_swing"] = json!(true);
        let mut host = FakeHost::default();
        host.respond(bone_response(10.0, Some(2.0)));
        host.respond(warp_response(true, &[9]));

        run_with_host(&raw, &mut host).unwrap();

        assert!(host.logs.iter().any(|line| line.contains("effective angle 12.000°")));
        assert!(host.logs.iter().any(|line| line.contains("Pin 9 target: (12.00, 0.00)")));
    }

    #[test]
    fn deform_missing_bone_fails_before_warp_query() {
        let mut host = FakeHost::default();
        host.respond(json!({"armatures":[{"bones":[]}]}));

        let error = run_with_host(&deform_params(false), &mut host).unwrap_err();

        assert!(error.contains("bone 4 was not returned"));
        assert_eq!(host.commands.len(), 1);
    }

    #[test]
    fn deform_missing_pin_fails_before_apply() {
        let mut host = FakeHost::default();
        host.respond(bone_response(10.0, None));
        host.respond(warp_response(true, &[]));

        let error = run_with_host(&deform_params(true), &mut host).unwrap_err();

        assert!(error.contains("pin 9 was not returned"));
        assert_eq!(host.commands.len(), 2);
    }

    #[test]
    fn deform_non_warped_element_fails_before_apply() {
        let mut host = FakeHost::default();
        host.respond(bone_response(10.0, None));
        host.respond(warp_response(false, &[]));

        let error = run_with_host(&deform_params(true), &mut host).unwrap_err();

        assert!(error.contains("has no Asset Warp mesh"));
        assert_eq!(host.commands.len(), 2);
    }

    #[test]
    fn malformed_host_responses_and_host_errors_propagate() {
        let mut malformed_bone = FakeHost::default();
        malformed_bone.respond(json!({"armatures":null}));
        assert!(run_with_host(&deform_params(false), &mut malformed_bone)
            .unwrap_err()
            .contains("invalid bone.info response"));

        let mut malformed_warp = FakeHost::default();
        malformed_warp.respond(bone_response(10.0, None));
        malformed_warp.respond(json!({"warped":true}));
        assert!(run_with_host(&deform_params(false), &mut malformed_warp)
            .unwrap_err()
            .contains("invalid warp.info response"));

        let mut host_error = FakeHost::default();
        host_error.fail("host refused request");
        assert!(run_with_host(&slider_params(None), &mut host_error)
            .unwrap_err()
            .contains("host refused request"));
    }

    #[test]
    fn repeated_deform_evaluation_uses_the_same_authored_neutral_base() {
        let raw = deform_params(false);
        let mut first_host = FakeHost::default();
        first_host.respond(bone_response(10.0, None));
        first_host.respond(warp_response(true, &[9]));
        run_with_host(&raw, &mut first_host).unwrap();

        let mut second_host = FakeHost::default();
        second_host.respond(bone_response(10.0, None));
        second_host.respond(warp_response(true, &[9]));
        run_with_host(&raw, &mut second_host).unwrap();

        assert_eq!(first_host.logs, second_host.logs);
        assert_eq!(first_host.commands, second_host.commands);
        assert_eq!(first_host.commands.len(), 2);
    }

    #[test]
    fn transform_and_unknown_modes_are_rejected_without_host_calls() {
        let mut host = FakeHost::default();
        assert!(run_with_host(&json!({"mode":"transform"}), &mut host)
            .unwrap_err()
            .contains("Transform mode remains deferred"));
        assert!(run_with_host(&json!({"mode":"not-a-mode"}), &mut host)
            .unwrap_err()
            .contains("unsupported Smart Controller mode"));
        assert!(host.commands.is_empty());
    }
}
