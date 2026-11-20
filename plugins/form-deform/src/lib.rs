//! AniCraft host adapter for the typed Form Deform domain model.

use anicraft_plugin_sdk as ac;
use form_deform_core::{
    BoneAngle, DeformRequest, DeformSpec, Vec2, evaluate_deformation,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.form-deform",
  "name": "Form Deform",
  "version": "0.1.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Drive one Asset Warp pin from a bone angle using an authored neutral position and per-degree gains.",
  "params": {
    "bone_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Controller bone ID"},
    "element_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warped shape element ID"},
    "pin_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warp pin ID"},
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

/// Typed command parameters decoded at the plugin boundary.
#[derive(Debug, Deserialize)]
struct PluginParams {
    bone_id: u64,
    element_id: u64,
    pin_id: u64,
    neutral_angle: f64,
    neutral_x: f64,
    neutral_y: f64,
    gain_x: f64,
    gain_y: f64,
    include_spring_swing: bool,
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

/// Explicit host command payload; JSON conversion is confined to this adapter.
#[derive(Debug, Serialize)]
struct WarpPinMoveRequest {
    element: u64,
    pin: u64,
    x: f64,
    y: f64,
}

fn run(raw_params: &Value) -> Result<(), String> {
    let params: PluginParams = serde_json::from_value(raw_params.clone())
        .map_err(|error| format!("invalid Form Deform parameters: {error}"))?;
    if params.bone_id == 0 || params.element_id == 0 || params.pin_id == 0 {
        return Err("bone, element, and pin IDs must be positive".to_string());
    }

    let bone_value = ac::command("bone.info", json!({}))?;
    let bone_data: BoneInfoResponse = serde_json::from_value(bone_value)
        .map_err(|error| format!("invalid bone.info response: {error}"))?;
    let bone = bone_data.armatures.iter()
        .flat_map(|armature| armature.bones.iter())
        .find(|bone| bone.id == params.bone_id)
        .ok_or_else(|| format!("bone {} is not present in an armature at the playhead", params.bone_id))?;

    let warp_value = ac::command("warp.info", json!({"element": params.element_id}))?;
    let warp: WarpInfoResponse = serde_json::from_value(warp_value)
        .map_err(|error| format!("invalid warp.info response: {error}"))?;
    if !warp.warped {
        return Err(format!("element {} has no usable Asset Warp mesh; add a warp and pin first", params.element_id));
    }
    if !warp.pins.iter().any(|pin| pin.pin == params.pin_id) {
        return Err(format!("pin {} does not belong to warped element {}", params.pin_id, params.element_id));
    }

    let result = evaluate_deformation(DeformRequest {
        spec: DeformSpec {
            neutral_position: Vec2 { x: params.neutral_x, y: params.neutral_y },
            neutral_angle_degrees: params.neutral_angle,
            gain_per_degree: Vec2 { x: params.gain_x, y: params.gain_y },
        },
        bone_angle: BoneAngle {
            keyed_degrees: bone.angle,
            spring_swing_degrees: bone.swing,
        },
        include_spring_swing: params.include_spring_swing,
    }).map_err(|error| error.to_string())?;

    ac::log(&format!(
        "Form Deform — bone {}: keyed angle {:.3}°, spring swing {}, effective angle {:.3}°",
        params.bone_id,
        result.keyed_angle_degrees,
        result.spring_swing_degrees.map(|value| format!("{value:.3}°")).unwrap_or_else(|| "unavailable".to_string()),
        result.effective_angle_degrees,
    ));
    ac::log(&format!(
        "Pin {} on element {}: neutral ({:.2}, {:.2}) → target ({:.2}, {:.2}) stage px",
        params.pin_id, params.element_id, params.neutral_x, params.neutral_y,
        result.target_position.x, result.target_position.y,
    ));

    if params.apply {
        let command = WarpPinMoveRequest {
            element: params.element_id,
            pin: params.pin_id,
            x: result.target_position.x,
            y: result.target_position.y,
        };
        let command_params = serde_json::to_value(command)
            .map_err(|error| format!("could not encode warp.pin_move request: {error}"))?;
        ac::command("warp.pin_move", command_params)?;
        ac::log("Applied deformation. This document edit is undoable.");
    } else {
        ac::log("Calculation only. Enable Apply pin position to write the result to the document.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_bone_info_and_finds_stable_host_id() {
        let data: BoneInfoResponse = serde_json::from_value(json!({
            "armatures": [{"bones": [{"id": 42, "angle": 12.5, "swing": 1.5}, {"id": 91, "angle": -3.0, "swing": 0.0}]}]
        })).unwrap();
        assert_eq!(data.armatures[0].bones.iter().find(|bone| bone.id == 91).unwrap().angle, -3.0);
    }

    #[test]
    fn absent_spring_value_is_preserved_as_absent() {
        let data: BoneInfoResponse = serde_json::from_value(json!({
            "armatures": [{"bones": [{"id": 42, "angle": 12.5}]}]
        })).unwrap();
        assert_eq!(data.armatures[0].bones[0].swing, None);
    }

    #[test]
    fn parses_typed_warp_info_and_pin_membership() {
        let data: WarpInfoResponse = serde_json::from_value(json!({
            "warped": true, "pins": [{"pin": 7}, {"pin": 19}]
        })).unwrap();
        assert!(data.warped);
        assert!(data.pins.iter().any(|pin| pin.pin == 19));
        assert!(!data.pins.iter().any(|pin| pin.pin == 2));
    }

    #[test]
    fn malformed_host_responses_are_rejected() {
        assert!(serde_json::from_value::<BoneInfoResponse>(json!({"armatures": null})).is_err());
        assert!(serde_json::from_value::<WarpInfoResponse>(json!({"warped": true})).is_err());
        assert!(serde_json::from_value::<PluginParams>(json!({"bone_id": 1})).is_err());
    }

    #[test]
    fn pin_move_request_has_explicit_fields() {
        let value = serde_json::to_value(WarpPinMoveRequest { element: 2, pin: 3, x: 4.0, y: 5.0 }).unwrap();
        assert_eq!(value, json!({"element": 2, "pin": 3, "x": 4.0, "y": 5.0}));
    }
}
